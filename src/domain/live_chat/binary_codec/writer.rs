use std::net::IpAddr;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::live_chat::cache::{CachedChatMessage, ChatActor, ChatActorKey};

use super::{
    ACTOR_GUEST, ACTOR_USER, IP_NONE, IP_V4, IP_V6, MESSAGE_FLAG_DELETED_AT,
    MESSAGE_FLAG_EDITED_AT, NONE_STRING_LEN, saturating_u8,
};

#[derive(Default)]
pub(super) struct BinaryWriter {
    bytes: Vec<u8>,
}

impl BinaryWriter {
    pub(super) fn into_inner(self) -> Vec<u8> {
        self.bytes
    }

    pub(super) fn write_u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    pub(super) fn write_u16(&mut self, value: u16) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    pub(super) fn write_u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    pub(super) fn write_u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn write_i64(&mut self, value: i64) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    pub(super) fn write_time(&mut self, value: DateTime<Utc>) {
        self.write_i64(value.timestamp_millis());
    }

    fn write_uuid(&mut self, value: Uuid) {
        self.bytes.extend_from_slice(value.as_bytes());
    }

    pub(super) fn write_uuid_string(&mut self, value: &str) -> anyhow::Result<()> {
        let uuid = Uuid::parse_str(value)?;
        self.write_uuid(uuid);
        Ok(())
    }

    pub(super) fn write_string(&mut self, value: &str) -> anyhow::Result<()> {
        let bytes = value.as_bytes();
        if bytes.len() > u16::MAX as usize - 1 {
            return Err(anyhow::anyhow!(
                "live chat binary string exceeds u16 length"
            ));
        }
        self.write_u16(bytes.len() as u16);
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }

    fn write_optional_string(&mut self, value: Option<&str>) -> anyhow::Result<()> {
        match value {
            Some(value) => self.write_string(value),
            None => {
                self.write_u16(NONE_STRING_LEN);
                Ok(())
            }
        }
    }

    fn write_ip(&mut self, value: Option<IpAddr>) {
        match value {
            Some(IpAddr::V4(ip)) => {
                self.write_u8(IP_V4);
                self.bytes.extend_from_slice(&ip.octets());
            }
            Some(IpAddr::V6(ip)) => {
                self.write_u8(IP_V6);
                self.bytes.extend_from_slice(&ip.octets());
            }
            None => self.write_u8(IP_NONE),
        }
    }

    pub(super) fn write_actor(&mut self, actor: &ChatActor) -> anyhow::Result<()> {
        match actor.actor_key {
            ChatActorKey::User(user_id) => {
                self.write_u8(ACTOR_USER);
                self.write_uuid(user_id);
                self.write_ip(None);
            }
            ChatActorKey::Guest(_) => {
                self.write_u8(ACTOR_GUEST);
                self.write_ip(actor.guest_ip);
            }
        }
        self.write_u8(saturating_u8(actor.sender_kind));
        self.write_string(&actor.display_name)?;
        self.write_optional_string(actor.country_flag.as_deref())?;
        self.write_optional_string(actor.user_profile_picture_url.as_deref())?;
        Ok(())
    }

    pub(super) fn write_message(&mut self, message: &CachedChatMessage) -> anyhow::Result<()> {
        self.write_uuid(message.live_chat_message_id);
        self.write_string(&message.room_key)?;
        self.write_u8(match message.user_id {
            Some(_) => ACTOR_USER,
            None => ACTOR_GUEST,
        });
        if let Some(user_id) = message.user_id {
            self.write_uuid(user_id);
            self.write_ip(None);
        } else {
            self.write_ip(message.guest_ip);
        }
        self.write_u8(saturating_u8(message.sender_kind));
        self.write_string(&message.sender_display_name)?;
        self.write_optional_string(message.sender_country_flag.as_deref())?;
        self.write_optional_string(message.user_profile_picture_url.as_deref())?;
        self.write_time(message.message_created_at);

        let mut flags = 0u8;
        if message.message_edited_at.is_some() {
            flags |= MESSAGE_FLAG_EDITED_AT;
        }
        if message.message_deleted_at.is_some() {
            flags |= MESSAGE_FLAG_DELETED_AT;
        }
        self.write_u8(flags);
        if let Some(edited_at) = message.message_edited_at {
            self.write_time(edited_at);
        }
        if let Some(deleted_at) = message.message_deleted_at {
            self.write_time(deleted_at);
        }
        self.write_string(&message.message_body)?;
        Ok(())
    }
}
