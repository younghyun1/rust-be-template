use chrono::{DateTime, Utc};
use serde_derive::{Deserialize, Serialize};

use super::{CachedChatMessage, ChatActor};
use crate::domain::live_chat::rtc::RtcServerSignal;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypingState {
    pub actor: ChatActor,
    pub room_key: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ChatConnectionState {
    pub actor: ChatActor,
    pub room_key: String,
    pub connected_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LiveChatServerEvent {
    Hello {
        actor: ChatActor,
        recent_messages: Vec<CachedChatMessage>,
        connected_count: u64,
    },
    Message {
        message: CachedChatMessage,
    },
    MessageAck {
        client_message_id: String,
        message: CachedChatMessage,
    },
    Typing {
        actor: ChatActor,
        is_typing: bool,
        expires_at: DateTime<Utc>,
    },
    TypingSet {
        actors: Vec<ChatActor>,
        expires_at: DateTime<Utc>,
    },
    Presence {
        connected_count: u64,
    },
    HeartbeatAck {
        nonce: String,
    },
    Error {
        code: String,
        message: String,
    },
    /// WebRTC signaling envelope. Unicast variants (answer/offer/ice/roster) are
    /// sent to a single connection; peer-state is broadcast room-wide. The inner
    /// enum is internally tagged with "kind".
    Rtc(RtcServerSignal),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveChatCacheStats {
    pub max_bytes: usize,
    pub used_bytes: usize,
    pub message_count: usize,
    pub oldest_cached_at: Option<DateTime<Utc>>,
    pub newest_cached_at: Option<DateTime<Utc>>,
    pub active_typing_count: usize,
    pub connected_count: u64,
}
