use axum::{body::Bytes, extract::ws::Message};
use tracing::{error, warn};

use crate::{
    domain::live_chat::{
        binary_codec::{LiveChatBinaryClientEvent, decode_client_event, encode_server_event},
        cache::LiveChatServerEvent,
    },
    dto::requests::live_chat::LiveChatClientEvent,
};

use super::{LIVE_CHAT_MAX_FRAME_BYTES, LiveChatWireProtocol};

/// Outbound frame channel to the per-connection writer task. Handlers enqueue
/// already-or-soon-encoded frames here instead of writing to the socket directly,
/// so a slow DB persist on the read side never stalls the broadcast drain.
pub(super) type OutboundSender = tokio::sync::mpsc::Sender<Message>;

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
    out: &OutboundSender,
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
            send_event(out, &event, LiveChatWireProtocol::Json).await;
            return None;
        }
    };

    if text.len() > LIVE_CHAT_MAX_FRAME_BYTES {
        let event = LiveChatServerEvent::Error {
            code: "frame_too_large".to_string(),
            message: "Live chat event payload is too large.".to_string(),
        };
        send_event(out, &event, LiveChatWireProtocol::Json).await;
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
            send_event(out, &event, LiveChatWireProtocol::Json).await;
            None
        }
    }
}

pub(super) async fn decode_binary_client_event(
    out: &OutboundSender,
    message: Message,
) -> Option<DecodedLiveChatClientEvent> {
    let bytes = match message {
        Message::Binary(bytes) => bytes,
        // Transport control frames (Ping/Pong) are auto-handled by tungstenite;
        // skip them silently instead of emitting an application-level error.
        Message::Ping(_) | Message::Pong(_) => return None,
        Message::Text(_) => {
            let event = LiveChatServerEvent::Error {
                code: "invalid_frame".to_string(),
                message: "Expected binary live chat frame".to_string(),
            };
            send_event(out, &event, LiveChatWireProtocol::Binary).await;
            return None;
        }
        Message::Close(_) => return None,
    };

    if bytes.len() > LIVE_CHAT_MAX_FRAME_BYTES {
        let event = LiveChatServerEvent::Error {
            code: "frame_too_large".to_string(),
            message: "Live chat event payload is too large.".to_string(),
        };
        send_event(out, &event, LiveChatWireProtocol::Binary).await;
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
            send_event(out, &event, LiveChatWireProtocol::Binary).await;
            None
        }
    }
}

/// Serialize a server event into a WebSocket frame for the negotiated wire
/// protocol. Returns `None` (and logs) on a serialization failure.
pub(super) fn encode_event(
    event: &LiveChatServerEvent,
    wire_protocol: LiveChatWireProtocol,
) -> Option<Message> {
    if matches!(wire_protocol, LiveChatWireProtocol::Binary) {
        match encode_server_event(event) {
            Ok(payload) => Some(Message::Binary(Bytes::from(payload))),
            Err(e) => {
                error!(error = ?e, "Failed to serialize binary live chat server event");
                None
            }
        }
    } else {
        match serde_json::to_string(event) {
            Ok(payload) => Some(Message::Text(payload.into())),
            Err(e) => {
                error!(error = ?e, "Failed to serialize live chat server event");
                None
            }
        }
    }
}

/// Enqueue a server event for the writer task. A closed channel means the
/// connection is already tearing down, so the frame is dropped silently.
pub(super) async fn send_event(
    out: &OutboundSender,
    event: &LiveChatServerEvent,
    wire_protocol: LiveChatWireProtocol,
) {
    if let Some(message) = encode_event(event, wire_protocol) {
        let _ = out.send(message).await;
    }
}
