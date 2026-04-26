use std::net::IpAddr;

use serde_derive::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::live_chat::{
    guest_nickname::guest_nickname_for_ip,
    message::{LIVE_CHAT_SENDER_KIND_GUEST, LIVE_CHAT_SENDER_KIND_USER},
};

#[derive(Debug, Clone, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum ChatActorKey {
    User(Uuid),
    Guest(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatActor {
    pub actor_key: ChatActorKey,
    pub sender_kind: i16,
    pub user_id: Option<Uuid>,
    pub guest_ip: Option<IpAddr>,
    pub display_name: String,
    pub country_flag: Option<String>,
    pub user_profile_picture_url: Option<String>,
}

impl ChatActor {
    pub fn guest(ip: IpAddr, country_flag: Option<String>) -> Self {
        let display_name = guest_nickname_for_ip(ip);
        Self {
            actor_key: ChatActorKey::Guest(ip.to_string()),
            sender_kind: LIVE_CHAT_SENDER_KIND_GUEST,
            user_id: None,
            guest_ip: Some(ip),
            display_name,
            country_flag,
            user_profile_picture_url: None,
        }
    }

    pub fn user(
        user_id: Uuid,
        display_name: String,
        country_flag: Option<String>,
        user_profile_picture_url: Option<String>,
    ) -> Self {
        Self {
            actor_key: ChatActorKey::User(user_id),
            sender_kind: LIVE_CHAT_SENDER_KIND_USER,
            user_id: Some(user_id),
            guest_ip: None,
            display_name,
            country_flag,
            user_profile_picture_url,
        }
    }
}
