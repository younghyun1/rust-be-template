use std::{net::SocketAddr, sync::Arc};

use axum::{
    Extension,
    extract::{
        ConnectInfo, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::HeaderMap,
    response::Response,
};
use chrono::{Duration as ChronoDuration, Utc};
use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use tokio::time::{Duration, Instant};
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::{
    domain::live_chat::{
        cache::{
            CachedChatMessage, ChatActor, ChatConnectionState, DEFAULT_LIVE_CHAT_ROOM,
            LiveChatServerEvent, TypingState,
        },
        message::LiveChatMessageInsertable,
    },
    dto::requests::live_chat::LiveChatClientEvent,
    init::state::ServerState,
    routers::middleware::is_logged_in::AuthStatus,
    schema::{live_chat_messages, users},
    util::extract::client_ip::extract_client_ip,
};

const LIVE_CHAT_INITIAL_MESSAGES: usize = 50;
const LIVE_CHAT_MAX_MESSAGE_BYTES: usize = 4096;
const LIVE_CHAT_MIN_SEND_INTERVAL_MS: u64 = 500;
const LIVE_CHAT_TYPING_TTL_SECONDS: i64 = 4;

pub async fn live_chat_ws_handler(
    Extension(auth_status): Extension<AuthStatus>,
    State(state): State<Arc<ServerState>>,
    ConnectInfo(socket_addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Response {
    let actor = resolve_actor(state.clone(), auth_status, &headers, socket_addr).await;
    ws.on_upgrade(move |socket| handle_live_chat_socket(socket, state, actor))
}

async fn resolve_actor(
    state: Arc<ServerState>,
    auth_status: AuthStatus,
    headers: &HeaderMap,
    socket_addr: SocketAddr,
) -> ChatActor {
    match auth_status {
        AuthStatus::LoggedIn(user_id) => {
            let display_name = match get_user_display_name(state, user_id).await {
                Some(name) => name,
                None => format!("user@{user_id}"),
            };
            ChatActor::user(user_id, display_name)
        }
        AuthStatus::LoggedOut => {
            let guest_ip = match extract_client_ip(headers, socket_addr) {
                Some(ip) => ip,
                None => socket_addr.ip(),
            };
            ChatActor::guest(guest_ip)
        }
    }
}

async fn get_user_display_name(state: Arc<ServerState>, user_id: Uuid) -> Option<String> {
    let mut conn = match state.get_conn().await {
        Ok(conn) => conn,
        Err(e) => {
            error!(error = ?e, user_id = %user_id, "Failed to get DB connection for live chat actor");
            return None;
        }
    };

    let user_name = users::table
        .filter(users::user_id.eq(user_id))
        .select(users::user_name)
        .first::<String>(&mut conn)
        .await
        .map_err(|e| {
            warn!(error = ?e, user_id = %user_id, "Failed to resolve live chat username");
            e
        })
        .ok();

    drop(conn);
    user_name
}

async fn handle_live_chat_socket(mut socket: WebSocket, state: Arc<ServerState>, actor: ChatActor) {
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

    let recent_messages = state
        .live_chat_cache
        .get_recent_chat_messages(LIVE_CHAT_INITIAL_MESSAGES)
        .await;
    let hello = LiveChatServerEvent::Hello {
        actor: actor.clone(),
        recent_messages,
    };
    if send_event(&mut socket, &hello).await.is_err() {
        cleanup_live_chat_connection(state, connection_id, &actor).await;
        return;
    }

    state
        .live_chat_cache
        .broadcast(LiveChatServerEvent::Presence {
            connected_count: state.live_chat_cache.connected_count(),
        });

    let mut broadcast_rx = state.live_chat_cache.subscribe();
    let mut last_send_at: Option<Instant> = None;

    loop {
        tokio::select! {
            socket_message = socket.recv() => {
                let should_continue = match socket_message {
                    Some(Ok(message)) => {
                        handle_client_message(
                            &mut socket,
                            state.clone(),
                            actor.clone(),
                            message,
                            &mut last_send_at,
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
                        if send_event(&mut socket, &event).await.is_err() {
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
    message: Message,
    last_send_at: &mut Option<Instant>,
) -> bool {
    if matches!(message, Message::Close(_)) {
        return false;
    }

    let text = match message.to_text() {
        Ok(text) => text,
        Err(e) => {
            let event = LiveChatServerEvent::Error {
                code: "invalid_frame".to_string(),
                message: "Expected UTF-8 text frame".to_string(),
            };
            error!(error = ?e, "Failed to parse live chat WebSocket frame as text");
            let _ = send_event(socket, &event).await;
            return true;
        }
    };

    let client_event = match serde_json::from_str::<LiveChatClientEvent>(text) {
        Ok(event) => event,
        Err(e) => {
            let event = LiveChatServerEvent::Error {
                code: "invalid_json".to_string(),
                message: "Invalid live chat event payload".to_string(),
            };
            warn!(error = ?e, "Failed to parse live chat client event");
            let _ = send_event(socket, &event).await;
            return true;
        }
    };

    match client_event {
        LiveChatClientEvent::SendMessage {
            client_message_id,
            body,
        } => {
            handle_send_message(socket, state, actor, client_message_id, body, last_send_at).await;
        }
        LiveChatClientEvent::Typing { is_typing } => {
            handle_typing(state, actor, is_typing).await;
        }
        LiveChatClientEvent::Heartbeat { nonce } => {
            let event = LiveChatServerEvent::HeartbeatAck { nonce };
            let _ = send_event(socket, &event).await;
        }
    }

    true
}

async fn handle_send_message(
    socket: &mut WebSocket,
    state: Arc<ServerState>,
    actor: ChatActor,
    client_message_id: String,
    body: String,
    last_send_at: &mut Option<Instant>,
) {
    let now = Instant::now();
    if let Some(previous) = *last_send_at
        && now.duration_since(previous) < Duration::from_millis(LIVE_CHAT_MIN_SEND_INTERVAL_MS)
    {
        let event = LiveChatServerEvent::Error {
            code: "rate_limited".to_string(),
            message: "Please slow down before sending another message.".to_string(),
        };
        let _ = send_event(socket, &event).await;
        return;
    }

    let body = body.trim().to_string();
    if body.is_empty() {
        let event = LiveChatServerEvent::Error {
            code: "empty_message".to_string(),
            message: "Message cannot be empty.".to_string(),
        };
        let _ = send_event(socket, &event).await;
        return;
    }

    if body.len() > LIVE_CHAT_MAX_MESSAGE_BYTES {
        let event = LiveChatServerEvent::Error {
            code: "message_too_large".to_string(),
            message: "Message is too large.".to_string(),
        };
        let _ = send_event(socket, &event).await;
        return;
    }

    let persisted = match persist_message(state.clone(), &actor, body).await {
        Some(message) => message,
        None => {
            let event = LiveChatServerEvent::Error {
                code: "persist_failed".to_string(),
                message: "Message could not be saved.".to_string(),
            };
            let _ = send_event(socket, &event).await;
            return;
        }
    };

    *last_send_at = Some(now);
    state
        .live_chat_cache
        .append_persisted_chat_message(persisted.clone())
        .await;
    state.live_chat_cache.clear_typing(&actor.actor_key).await;

    let ack = LiveChatServerEvent::MessageAck {
        client_message_id,
        message: persisted.clone(),
    };
    let _ = send_event(socket, &ack).await;

    state
        .live_chat_cache
        .broadcast(LiveChatServerEvent::Message { message: persisted });
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
    persisted.map(CachedChatMessage::from)
}

async fn handle_typing(state: Arc<ServerState>, actor: ChatActor, is_typing: bool) {
    let expires_at = Utc::now() + ChronoDuration::seconds(LIVE_CHAT_TYPING_TTL_SECONDS);
    if is_typing {
        state
            .live_chat_cache
            .set_typing(TypingState {
                actor: actor.clone(),
                room_key: DEFAULT_LIVE_CHAT_ROOM.to_string(),
                expires_at,
            })
            .await;
    } else {
        state.live_chat_cache.clear_typing(&actor.actor_key).await;
    }

    state.live_chat_cache.clear_expired_typing(Utc::now()).await;
    state
        .live_chat_cache
        .broadcast(LiveChatServerEvent::Typing {
            actor,
            is_typing,
            expires_at,
        });
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
    state.live_chat_cache.clear_typing(&actor.actor_key).await;
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
) -> Result<(), axum::Error> {
    let payload = match serde_json::to_string(event) {
        Ok(payload) => payload,
        Err(e) => {
            error!(error = ?e, "Failed to serialize live chat server event");
            return Ok(());
        }
    };

    socket.send(Message::Text(payload.into())).await
}
