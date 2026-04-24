use std::{net::SocketAddr, sync::Arc};

use axum::{
    Extension,
    body::Bytes,
    extract::{
        ConnectInfo, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use chrono::{Duration as ChronoDuration, Utc};
use diesel_async::RunQueryDsl;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::{
    domain::live_chat::{
        ban::{LIVE_CHAT_BAN_SOURCE_ABNORMAL_MESSAGING, LiveChatBan, LiveChatBanInsertable},
        binary_codec::{
            LIVE_CHAT_BINARY_PROTOCOL, LiveChatBinaryClientEvent, decode_client_event,
            encode_server_event,
        },
        cache::{
            CachedChatMessage, CachedLiveChatBan, ChatActor, ChatConnectionState,
            DEFAULT_LIVE_CHAT_ROOM, LiveChatServerEvent, TypingState,
        },
        message::LiveChatMessageInsertable,
    },
    dto::requests::live_chat::LiveChatClientEvent,
    init::state::ServerState,
    routers::middleware::is_logged_in::{AuthSession, AuthStatus},
    schema::{live_chat_bans, live_chat_messages},
    util::extract::client_ip::extract_client_ip,
};

const LIVE_CHAT_INITIAL_MESSAGES: usize = 50;
const LIVE_CHAT_MAX_MESSAGE_CHARS: usize = 300;
const LIVE_CHAT_MAX_FRAME_BYTES: usize = 2 * 1024;
const LIVE_CHAT_TYPING_TTL_SECONDS: i64 = 4;

#[derive(Clone, Copy)]
enum LiveChatWireProtocol {
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

enum DecodedLiveChatClientEvent {
    SendMessage {
        client_message_id: String,
        body: String,
    },
    Typing {
        is_typing: bool,
    },
    Heartbeat {
        nonce: String,
    },
}

impl From<LiveChatClientEvent> for DecodedLiveChatClientEvent {
    fn from(event: LiveChatClientEvent) -> Self {
        match event {
            LiveChatClientEvent::SendMessage {
                client_message_id,
                body,
            } => Self::SendMessage {
                client_message_id,
                body,
            },
            LiveChatClientEvent::Typing { is_typing } => Self::Typing { is_typing },
            LiveChatClientEvent::Heartbeat { nonce } => Self::Heartbeat { nonce },
        }
    }
}

impl From<LiveChatBinaryClientEvent> for DecodedLiveChatClientEvent {
    fn from(event: LiveChatBinaryClientEvent) -> Self {
        match event {
            LiveChatBinaryClientEvent::SendMessage {
                client_message_id,
                body,
            } => Self::SendMessage {
                client_message_id,
                body,
            },
            LiveChatBinaryClientEvent::Typing { is_typing } => Self::Typing { is_typing },
            LiveChatBinaryClientEvent::Heartbeat { nonce } => Self::Heartbeat { nonce },
        }
    }
}

async fn decode_json_client_event(
    socket: &mut WebSocket,
    message: &Message,
) -> Option<DecodedLiveChatClientEvent> {
    let text = match message.to_text() {
        Ok(text) => text,
        Err(e) => {
            let event = LiveChatServerEvent::Error {
                code: "invalid_frame".to_string(),
                message: "Expected UTF-8 text frame".to_string(),
            };
            error!(error = ?e, "Failed to parse live chat WebSocket frame as text");
            let _ = send_event(socket, &event, LiveChatWireProtocol::Json).await;
            return None;
        }
    };

    if text.len() > LIVE_CHAT_MAX_FRAME_BYTES {
        let event = LiveChatServerEvent::Error {
            code: "frame_too_large".to_string(),
            message: "Live chat event payload is too large.".to_string(),
        };
        let _ = send_event(socket, &event, LiveChatWireProtocol::Json).await;
        return None;
    }

    match serde_json::from_str::<LiveChatClientEvent>(text) {
        Ok(event) => Some(event.into()),
        Err(e) => {
            let event = LiveChatServerEvent::Error {
                code: "invalid_json".to_string(),
                message: "Invalid live chat event payload".to_string(),
            };
            warn!(error = ?e, "Failed to parse live chat client event");
            let _ = send_event(socket, &event, LiveChatWireProtocol::Json).await;
            None
        }
    }
}

async fn decode_binary_client_event(
    socket: &mut WebSocket,
    message: Message,
) -> Option<DecodedLiveChatClientEvent> {
    let bytes = match message {
        Message::Binary(bytes) => bytes,
        Message::Close(_) => return None,
        _ => {
            let event = LiveChatServerEvent::Error {
                code: "invalid_frame".to_string(),
                message: "Expected binary live chat frame".to_string(),
            };
            let _ = send_event(socket, &event, LiveChatWireProtocol::Binary).await;
            return None;
        }
    };

    if bytes.len() > LIVE_CHAT_MAX_FRAME_BYTES {
        let event = LiveChatServerEvent::Error {
            code: "frame_too_large".to_string(),
            message: "Live chat event payload is too large.".to_string(),
        };
        let _ = send_event(socket, &event, LiveChatWireProtocol::Binary).await;
        return None;
    }

    match decode_client_event(&bytes) {
        Ok(event) => Some(event.into()),
        Err(e) => {
            let event = LiveChatServerEvent::Error {
                code: "invalid_binary".to_string(),
                message: "Invalid live chat binary event payload".to_string(),
            };
            warn!(error = ?e, "Failed to parse live chat binary client event");
            let _ = send_event(socket, &event, LiveChatWireProtocol::Binary).await;
            None
        }
    }
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

async fn persist_message(
    state: Arc<ServerState>,
    actor: &ChatActor,
    body: String,
) -> Option<CachedChatMessage> {
    let mut conn = match state.get_conn().await {
        Ok(conn) => conn,
        Err(e) => {
            error!(error = ?e, "Failed to get DB connection for live chat message");
            return None;
        }
    };

    let new_message = LiveChatMessageInsertable {
        live_chat_message_id: Uuid::now_v7(),
        room_key: DEFAULT_LIVE_CHAT_ROOM.to_string(),
        user_id: actor.user_id,
        guest_ip: actor.guest_ip.map(ipnet::IpNet::from),
        sender_kind: actor.sender_kind,
        sender_display_name: actor.display_name.clone(),
        message_body: body,
        message_created_at: Utc::now(),
    };

    let persisted = diesel::insert_into(live_chat_messages::table)
        .values(new_message)
        .returning(live_chat_messages::all_columns)
        .get_result::<crate::domain::live_chat::message::LiveChatMessage>(&mut conn)
        .await
        .map_err(|e| {
            error!(error = ?e, "Failed to persist live chat message");
            e
        })
        .ok();

    drop(conn);
    persisted.map(|message| {
        let mut cached_message = CachedChatMessage::from(message);
        cached_message.sender_country_flag = actor.country_flag.clone();
        cached_message.user_profile_picture_url = actor.user_profile_picture_url.clone();
        cached_message
    })
}

async fn persist_live_chat_ban(
    state: Arc<ServerState>,
    actor: &ChatActor,
    client_ip: std::net::IpAddr,
) -> Option<CachedLiveChatBan> {
    let mut conn = match state.get_conn().await {
        Ok(conn) => conn,
        Err(e) => {
            error!(error = ?e, "Failed to get DB connection for live chat ban");
            return None;
        }
    };

    let new_ban = LiveChatBanInsertable {
        live_chat_ban_id: Uuid::now_v7(),
        user_id: actor.user_id,
        banned_ip: Some(ipnet::IpNet::from(client_ip)),
        reason: "More than 10 live chat message events in one second.".to_string(),
        ban_source: LIVE_CHAT_BAN_SOURCE_ABNORMAL_MESSAGING.to_string(),
        banned_at: Utc::now(),
        expires_at: None,
    };

    let persisted = diesel::insert_into(live_chat_bans::table)
        .values(new_ban)
        .returning(live_chat_bans::all_columns)
        .get_result::<LiveChatBan>(&mut conn)
        .await
        .map_err(|e| {
            error!(error = ?e, user_id = ?actor.user_id, client_ip = %client_ip, "Failed to persist live chat ban");
            e
        })
        .ok();

    drop(conn);
    persisted.map(CachedLiveChatBan::from)
}

async fn handle_typing(state: Arc<ServerState>, actor: ChatActor, is_typing: bool) {
    let expires_at = Utc::now() + ChronoDuration::seconds(LIVE_CHAT_TYPING_TTL_SECONDS);
    let changed = if is_typing {
        state
            .live_chat_cache
            .set_typing(TypingState {
                actor: actor.clone(),
                room_key: DEFAULT_LIVE_CHAT_ROOM.to_string(),
                expires_at,
            })
            .await
    } else {
        state.live_chat_cache.clear_typing(&actor.actor_key).await
    };

    if is_typing || changed {
        broadcast_typing_set(state, expires_at).await;
    }
}

async fn broadcast_typing_set(state: Arc<ServerState>, expires_at: chrono::DateTime<Utc>) {
    let actors = state.live_chat_cache.active_typing_actors(Utc::now()).await;
    state
        .live_chat_cache
        .broadcast(LiveChatServerEvent::TypingSet { actors, expires_at });
}

async fn cleanup_live_chat_connection(
    state: Arc<ServerState>,
    connection_id: Uuid,
    actor: &ChatActor,
) {
    state
        .live_chat_cache
        .unregister_connection(connection_id)
        .await;
    let typing_changed = state.live_chat_cache.clear_typing(&actor.actor_key).await;
    if typing_changed {
        let expires_at = Utc::now() + ChronoDuration::seconds(LIVE_CHAT_TYPING_TTL_SECONDS);
        broadcast_typing_set(state.clone(), expires_at).await;
    }
    state
        .live_chat_cache
        .broadcast(LiveChatServerEvent::Presence {
            connected_count: state.live_chat_cache.connected_count(),
        });
    info!(connection_id = %connection_id, "Live chat WebSocket disconnected");
}

async fn send_event(
    socket: &mut WebSocket,
    event: &LiveChatServerEvent,
    wire_protocol: LiveChatWireProtocol,
) -> Result<(), axum::Error> {
    if matches!(wire_protocol, LiveChatWireProtocol::Binary) {
        let payload = match encode_server_event(event) {
            Ok(payload) => payload,
            Err(e) => {
                error!(error = ?e, "Failed to serialize binary live chat server event");
                return Ok(());
            }
        };

        return socket.send(Message::Binary(Bytes::from(payload))).await;
    }

    let payload = match serde_json::to_string(event) {
        Ok(payload) => payload,
        Err(e) => {
            error!(error = ?e, "Failed to serialize live chat server event");
            return Ok(());
        }
    };

    socket.send(Message::Text(payload.into())).await
}
