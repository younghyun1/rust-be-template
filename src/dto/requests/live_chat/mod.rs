pub mod get_live_chat_messages_request;

pub use get_live_chat_messages_request::GetLiveChatMessagesRequest;

use serde_derive::Deserialize;

use crate::domain::live_chat::rtc::RtcClientSignal;

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LiveChatClientEvent {
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
    /// WebRTC signaling from the client. Inner enum is tagged with "kind".
    Rtc(RtcClientSignal),
}
