use chrono::{DateTime, Utc};
use serde_derive::Serialize;
use utoipa::ToSchema;

use crate::domain::live_chat::cache::LiveChatCacheStats;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct LiveChatCacheStatsResponse {
    pub max_bytes: usize,
    pub used_bytes: usize,
    pub message_count: usize,
    pub oldest_cached_at: Option<DateTime<Utc>>,
    pub newest_cached_at: Option<DateTime<Utc>>,
    pub active_typing_count: usize,
    pub connected_count: u64,
}

impl From<LiveChatCacheStats> for LiveChatCacheStatsResponse {
    fn from(stats: LiveChatCacheStats) -> Self {
        Self {
            max_bytes: stats.max_bytes,
            used_bytes: stats.used_bytes,
            message_count: stats.message_count,
            oldest_cached_at: stats.oldest_cached_at,
            newest_cached_at: stats.newest_cached_at,
            active_typing_count: stats.active_typing_count,
            connected_count: stats.connected_count,
        }
    }
}
