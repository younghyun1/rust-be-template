use std::net::IpAddr;

use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub(super) enum LiveChatRateKey {
    User(Uuid),
    Ip(IpAddr),
}

#[derive(Debug, Clone)]
pub(super) struct LiveChatRateState {
    pub(super) window_started_at: DateTime<Utc>,
    pub(super) count: u32,
}
