use chrono::{DateTime, Utc};
use diesel::{Insertable, Queryable, Selectable};
use ipnet::IpNet;
use uuid::Uuid;

use crate::schema::live_chat_bans;

pub const LIVE_CHAT_BAN_SOURCE_ABNORMAL_MESSAGING: &str = "abnormal_messaging";

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = live_chat_bans)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct LiveChatBan {
    pub live_chat_ban_id: Uuid,
    pub user_id: Option<Uuid>,
    pub banned_ip: Option<IpNet>,
    pub reason: String,
    pub ban_source: String,
    pub banned_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = live_chat_bans)]
pub struct LiveChatBanInsertable {
    pub live_chat_ban_id: Uuid,
    pub user_id: Option<Uuid>,
    pub banned_ip: Option<IpNet>,
    pub reason: String,
    pub ban_source: String,
    pub banned_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}
