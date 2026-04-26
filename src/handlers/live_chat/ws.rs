use std::{net::SocketAddr, sync::Arc};

use axum::{
    Extension,
    extract::{
        ConnectInfo, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use chrono::{Duration as ChronoDuration, Utc};
use tracing::{info, warn};
use uuid::Uuid;

use crate::{
    domain::live_chat::{
        binary_codec::LIVE_CHAT_BINARY_PROTOCOL,
        cache::{ChatActor, ChatConnectionState, DEFAULT_LIVE_CHAT_ROOM, LiveChatServerEvent},
    },
    init::state::ServerState,
    routers::middleware::is_logged_in::{AuthSession, AuthStatus},
    util::extract::client_ip::extract_client_ip,
};

mod persistence;
mod presence;
mod protocol;

use persistence::{persist_live_chat_ban, persist_message};
use presence::{broadcast_typing_set, cleanup_live_chat_connection, handle_typing};
use protocol::{
    DecodedLiveChatClientEvent, decode_binary_client_event, decode_json_client_event, send_event,
};

const LIVE_CHAT_INITIAL_MESSAGES: usize = 50;
const LIVE_CHAT_MAX_MESSAGE_CHARS: usize = 300;
pub(super) const LIVE_CHAT_MAX_FRAME_BYTES: usize = 2 * 1024;
pub(super) const LIVE_CHAT_TYPING_TTL_SECONDS: i64 = 4;

#[derive(Clone, Copy)]
pub(super) enum LiveChatWireProtocol {
    Json,
    Binary,
}

pub async fn live_chat_ws_handler(
    Extension(auth_status): Extension<AuthStatus>,
    Extension(auth_session): Extension<Option<AuthSession>>,
    State(state): State<Arc<ServerState>>,
    ConnectInfo(socket_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Response {
    let client_ip = match extract_client_ip(&headers, socket_addr) {
        Some(ip) => ip,
        None => socket_addr.ip(),
    };
    let actor = resolve_actor(state.clone(), auth_status, auth_session, client_ip).await;

    if state
        .live_chat_cache
        .is_banned(actor.user_id, client_ip)
        .await
    {
        return (StatusCode::FORBIDDEN, "Live chat access denied.").into_response();
    }

    let ws = ws.protocols([LIVE_CHAT_BINARY_PROTOCOL]);
    let wire_protocol = match ws.selected_protocol().and_then(|value| value.to_str().ok()) {
        Some(LIVE_CHAT_BINARY_PROTOCOL) => LiveChatWireProtocol::Binary,
        _ => LiveChatWireProtocol::Json,
    };

    ws.on_upgrade(move |socket| {
        handle_live_chat_socket(socket, state, actor, client_ip, wire_protocol)
    })
}

async fn resolve_actor(
    state: Arc<ServerState>,
    auth_status: AuthStatus,
    auth_session: Option<AuthSession>,
    client_ip: std::net::IpAddr,
) -> ChatActor {
    match auth_status {
        AuthStatus::LoggedIn(user_id) => {
            let (display_name, country_flag, user_profile_picture_url) = match auth_session {
                Some(session) if session.user_id == user_id => {
                    let country_flag = state
                        .country_flag_for_country_code(session.user_country)
                        .await;
                    let user_profile_picture_url =
                        state.latest_user_profile_picture_url(user_id).await;
                    (session.user_name, country_flag, user_profile_picture_url)
                }
                _ => (format!("user@{user_id}"), None, None),
            };
            ChatActor::user(
                user_id,
                display_name,
                country_flag,
                user_profile_picture_url,
            )
        }
        AuthStatus::LoggedOut => {
            let country_flag = state.country_flag_for_ip(client_ip).await;
            ChatActor::guest(client_ip, country_flag)
        }
    }
}

async fn handle_live_chat_socket(
    mut socket: WebSocket,
    state: Arc<ServerState>,
    actor: ChatActor,
    client_ip: std::net::IpAddr,
    wire_protocol: LiveChatWireProtocol,
) {
    let connection_id = Uuid::now_v7();
    state
        .live_chat_cache
        .register_connection(
            connection_id,
            ChatConnectionState {
                actor: actor.clone(),
                room_key: DEFAULT_LIVE_CHAT_ROOM.to_string(),
                connected_at: Utc::now(),
            },
        )
        .await;
    let mut broadcast_rx = state.live_chat_cache.subscribe();

    let recent_messages = state
        .live_chat_cache
        .get_recent_chat_messages(LIVE_CHAT_INITIAL_MESSAGES)
        .await;
    let hello = LiveChatServerEvent::Hello {
        actor: actor.clone(),
        recent_messages,
        connected_count: state.live_chat_cache.connected_count(),
    };
    if send_event(&mut socket, &hello, wire_protocol)
        .await
        .is_err()
    {
        cleanup_live_chat_connection(state, connection_id, &actor).await;
        return;
    }

    state
        .live_chat_cache
        .broadcast(LiveChatServerEvent::Presence {
            connected_count: state.live_chat_cache.connected_count(),
        });

    loop {
        tokio::select! {
            socket_message = socket.recv() => {
                let should_continue = match socket_message {
                    Some(Ok(message)) => {
                        handle_client_message(
                            &mut socket,
                            state.clone(),
                            actor.clone(),
                            client_ip,
                            message,
                            wire_protocol,
                        ).await
                    }
                    Some(Err(e)) => {
                        info!(error = ?e, connection_id = %connection_id, "Live chat WebSocket receive error");
                        false
                    }
                    None => false,
                };

                if !should_continue {
                    break;
                }
            }
            broadcast_event = broadcast_rx.recv() => {
                match broadcast_event {
                    Ok(event) => {
                        if send_event(&mut socket, &event, wire_protocol).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        warn!(error = ?e, connection_id = %connection_id, "Live chat broadcast receiver lagged or closed");
                    }
                }
            }
        }
    }

    cleanup_live_chat_connection(state, connection_id, &actor).await;
}

async fn handle_client_message(
    socket: &mut WebSocket,
    state: Arc<ServerState>,
    actor: ChatActor,
    client_ip: std::net::IpAddr,
    message: Message,
    wire_protocol: LiveChatWireProtocol,
) -> bool {
    if matches!(message, Message::Close(_)) {
        return false;
    }

    let client_event = match wire_protocol {
        LiveChatWireProtocol::Json => match decode_json_client_event(socket, &message).await {
            Some(event) => event,
            None => return true,
        },
        LiveChatWireProtocol::Binary => match decode_binary_client_event(socket, message).await {
            Some(event) => event,
            None => return true,
        },
    };

    match client_event {
        DecodedLiveChatClientEvent::SendMessage {
            client_message_id,
            body,
        } => {
            return handle_send_message(
                socket,
                state,
                actor,
                client_ip,
                client_message_id,
                body,
                wire_protocol,
            )
            .await;
        }
        DecodedLiveChatClientEvent::Typing { is_typing } => {
            handle_typing(state, actor, is_typing).await;
        }
        DecodedLiveChatClientEvent::Heartbeat { nonce } => {
            let event = LiveChatServerEvent::HeartbeatAck { nonce };
            let _ = send_event(socket, &event, wire_protocol).await;
        }
    }

    true
}

async fn handle_send_message(
    socket: &mut WebSocket,
    state: Arc<ServerState>,
    actor: ChatActor,
    client_ip: std::net::IpAddr,
    client_message_id: String,
    body: String,
    wire_protocol: LiveChatWireProtocol,
) -> bool {
    let now = Utc::now();
    if state
        .live_chat_cache
        .is_banned(actor.user_id, client_ip)
        .await
    {
        let event = LiveChatServerEvent::Error {
            code: "banned".to_string(),
            message: "Live chat access denied.".to_string(),
        };
        let _ = send_event(socket, &event, wire_protocol).await;
        return false;
    }

    if state
        .live_chat_cache
        .record_message_attempt(actor.user_id, client_ip, now)
        .await
    {
        if let Some(ban) = persist_live_chat_ban(state.clone(), &actor, client_ip).await {
            state.live_chat_cache.cache_ban(ban).await;
        }
        let event = LiveChatServerEvent::Error {
            code: "banned".to_string(),
            message: "Live chat access denied for abnormal messaging patterns.".to_string(),
        };
        let _ = send_event(socket, &event, wire_protocol).await;
        return false;
    }

    let body = body.trim().to_string();
    if body.is_empty() {
        let event = LiveChatServerEvent::Error {
            code: "empty_message".to_string(),
            message: "Message cannot be empty.".to_string(),
        };
        let _ = send_event(socket, &event, wire_protocol).await;
        return true;
    }

    if body.chars().count() > LIVE_CHAT_MAX_MESSAGE_CHARS {
        let event = LiveChatServerEvent::Error {
            code: "message_too_large".to_string(),
            message: format!("Message must be {LIVE_CHAT_MAX_MESSAGE_CHARS} characters or fewer."),
        };
        let _ = send_event(socket, &event, wire_protocol).await;
        return true;
    }

    let persisted = match persist_message(state.clone(), &actor, body).await {
        Some(message) => message,
        None => {
            let event = LiveChatServerEvent::Error {
                code: "persist_failed".to_string(),
                message: "Message could not be saved.".to_string(),
            };
            let _ = send_event(socket, &event, wire_protocol).await;
            return true;
        }
    };

    state
        .live_chat_cache
        .append_persisted_chat_message(persisted.clone())
        .await;
    let typing_changed = state.live_chat_cache.clear_typing(&actor.actor_key).await;
    if typing_changed {
        let expires_at = Utc::now() + ChronoDuration::seconds(LIVE_CHAT_TYPING_TTL_SECONDS);
        broadcast_typing_set(state.clone(), expires_at).await;
    }

    let ack = LiveChatServerEvent::MessageAck {
        client_message_id,
        message: persisted.clone(),
    };
    let _ = send_event(socket, &ack, wire_protocol).await;

    state
        .live_chat_cache
        .broadcast(LiveChatServerEvent::Message { message: persisted });

    true
}
