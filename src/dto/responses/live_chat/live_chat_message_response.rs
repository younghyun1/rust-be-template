use chrono::{DateTime, Utc};
use serde_derive::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::live_chat::cache::CachedChatMessage;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct LiveChatMessageItem {
    pub live_chat_message_id: Uuid,
    pub room_key: String,
    pub user_id: Option<Uuid>,
    pub guest_ip: Option<String>,
    pub sender_kind: i16,
    pub sender_display_name: String,
    pub sender_country_flag: Option<String>,
    pub user_profile_picture_url: Option<String>,
    pub message_body: String,
    pub message_created_at: DateTime<Utc>,
    pub message_edited_at: Option<DateTime<Utc>>,
    pub message_deleted_at: Option<DateTime<Utc>>,
}

impl From<CachedChatMessage> for LiveChatMessageItem {
    fn from(message: CachedChatMessage) -> Self {
        Self {
            live_chat_message_id: message.live_chat_message_id,
            room_key: message.room_key,
            user_id: message.user_id,
            guest_ip: message.guest_ip.map(|ip| ip.to_string()),
            sender_kind: message.sender_kind,
            sender_display_name: message.sender_display_name,
            sender_country_flag: message.sender_country_flag,
            user_profile_picture_url: message.user_profile_picture_url,
            message_body: message.message_body,
            message_created_at: message.message_created_at,
            message_edited_at: message.message_edited_at,
            message_deleted_at: message.message_deleted_at,
        }
    }
}
