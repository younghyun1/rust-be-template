use axum::{
    body::Bytes,
    extract::ws::{Message, WebSocket},
};
use tracing::{error, warn};

use crate::{
    domain::live_chat::{
        binary_codec::{LiveChatBinaryClientEvent, decode_client_event, encode_server_event},
        cache::LiveChatServerEvent,
    },
    dto::requests::live_chat::LiveChatClientEvent,
};

use super::{LIVE_CHAT_MAX_FRAME_BYTES, LiveChatWireProtocol};

pub(super) enum DecodedLiveChatClientEvent {
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

pub(super) async fn decode_json_client_event(
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

pub(super) async fn decode_binary_client_event(
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

pub(super) async fn send_event(
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
