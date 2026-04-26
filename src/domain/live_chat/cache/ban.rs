use std::net::IpAddr;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::live_chat::ban::LiveChatBan;

#[derive(Debug, Clone)]
pub struct CachedLiveChatBan {
    pub live_chat_ban_id: Uuid,
    pub user_id: Option<Uuid>,
    pub banned_ip: Option<IpAddr>,
    pub reason: String,
    pub ban_source: String,
    pub banned_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

impl CachedLiveChatBan {
    pub fn is_active(&self, now: DateTime<Utc>) -> bool {
        match self.expires_at {
            Some(expires_at) => expires_at > now,
            None => true,
        }
    }
}

impl From<LiveChatBan> for CachedLiveChatBan {
    fn from(ban: LiveChatBan) -> Self {
        Self {
            live_chat_ban_id: ban.live_chat_ban_id,
            user_id: ban.user_id,
            banned_ip: ban.banned_ip.map(|ip| ip.addr()),
            reason: ban.reason,
            ban_source: ban.ban_source,
            banned_at: ban.banned_at,
            expires_at: ban.expires_at,
        }
    }
}
