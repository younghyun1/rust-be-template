use chrono::{DateTime, Utc};
use diesel::{Insertable, Queryable, QueryableByName, Selectable};
use uuid::Uuid;

use crate::schema::live_chat_messages;

pub const LIVE_CHAT_SENDER_KIND_GUEST: i16 = 0;
pub const LIVE_CHAT_SENDER_KIND_USER: i16 = 1;

#[derive(Debug, Clone, QueryableByName, Queryable, Selectable)]
#[diesel(table_name = live_chat_messages)]
pub struct LiveChatMessage {
    pub live_chat_message_id: Uuid,
    pub room_key: String,
    pub user_id: Option<Uuid>,
    pub guest_ip: Option<ipnet::IpNet>,
    pub sender_kind: i16,
    pub sender_display_name: String,
    pub message_body: String,
    pub message_created_at: DateTime<Utc>,
    pub message_edited_at: Option<DateTime<Utc>>,
    pub message_deleted_at: Option<DateTime<Utc>>,
}

#[derive(Insertable)]
#[diesel(table_name = live_chat_messages)]
pub struct LiveChatMessageInsertable {
    pub live_chat_message_id: Uuid,
    pub room_key: String,
    pub user_id: Option<Uuid>,
    pub guest_ip: Option<ipnet::IpNet>,
    pub sender_kind: i16,
    pub sender_display_name: String,
    pub message_body: String,
    pub message_created_at: DateTime<Utc>,
}
