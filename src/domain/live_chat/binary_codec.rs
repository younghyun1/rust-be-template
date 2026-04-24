use std::net::IpAddr;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::live_chat::cache::{
    CachedChatMessage, ChatActor, ChatActorKey, LiveChatServerEvent,
};

pub const LIVE_CHAT_BINARY_PROTOCOL: &str = "livechat.bin.v1";

const CLIENT_SEND_MESSAGE: u8 = 0x01;
const CLIENT_TYPING_START: u8 = 0x02;
const CLIENT_TYPING_STOP: u8 = 0x03;
const CLIENT_PING: u8 = 0x04;

const SERVER_HELLO: u8 = 0x81;
const SERVER_MESSAGE: u8 = 0x82;
const SERVER_MESSAGE_ACK: u8 = 0x83;
const SERVER_TYPING_SET: u8 = 0x84;
const SERVER_PRESENCE: u8 = 0x85;
const SERVER_PONG: u8 = 0x86;
const SERVER_ERROR: u8 = 0x87;

const ACTOR_USER: u8 = 0x01;
const ACTOR_GUEST: u8 = 0x02;

const IP_NONE: u8 = 0x00;
const IP_V4: u8 = 0x04;
const IP_V6: u8 = 0x06;

const MESSAGE_FLAG_EDITED_AT: u8 = 0x01;
const MESSAGE_FLAG_DELETED_AT: u8 = 0x02;
const NONE_STRING_LEN: u16 = u16::MAX;

#[derive(Debug)]
pub enum LiveChatBinaryClientEvent {
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
}

pub fn decode_client_event(bytes: &[u8]) -> anyhow::Result<LiveChatBinaryClientEvent> {
    let mut reader = BinaryReader::new(bytes);
    let event_type = reader.read_u8()?;
    match event_type {
        CLIENT_SEND_MESSAGE => {
            let client_message_id = reader.read_uuid()?.to_string();
            let body = reader.read_string()?;
            reader.finish()?;
            Ok(LiveChatBinaryClientEvent::SendMessage {
                client_message_id,
                body,
            })
        }
        CLIENT_TYPING_START => {
            reader.finish()?;
            Ok(LiveChatBinaryClientEvent::Typing { is_typing: true })
        }
        CLIENT_TYPING_STOP => {
            reader.finish()?;
            Ok(LiveChatBinaryClientEvent::Typing { is_typing: false })
        }
        CLIENT_PING => {
            let nonce = reader.read_u64()?.to_string();
            reader.finish()?;
            Ok(LiveChatBinaryClientEvent::Heartbeat { nonce })
        }
        _ => Err(anyhow::anyhow!(
            "Unknown live chat binary client event type"
        )),
    }
}

pub fn encode_server_event(event: &LiveChatServerEvent) -> anyhow::Result<Vec<u8>> {
    let mut writer = BinaryWriter::default();
    match event {
        LiveChatServerEvent::Hello {
            actor,
            recent_messages,
            connected_count,
        } => {
            writer.write_u8(SERVER_HELLO);
            writer.write_actor(actor)?;
            writer.write_u32(saturating_u32(*connected_count));
            writer.write_u16(saturating_u16(recent_messages.len()));
            for message in recent_messages.iter().take(u16::MAX as usize) {
                writer.write_message(message)?;
            }
        }
        LiveChatServerEvent::Message { message } => {
            writer.write_u8(SERVER_MESSAGE);
            writer.write_message(message)?;
        }
        LiveChatServerEvent::MessageAck {
            client_message_id,
            message,
        } => {
            writer.write_u8(SERVER_MESSAGE_ACK);
            writer.write_uuid_string(client_message_id)?;
            writer.write_message(message)?;
        }
        LiveChatServerEvent::TypingSet { actors, expires_at } => {
            writer.write_u8(SERVER_TYPING_SET);
            writer.write_time(*expires_at);
            writer.write_u8(saturating_u8(actors.len()));
            for actor in actors.iter().take(u8::MAX as usize) {
                writer.write_actor(actor)?;
            }
        }
        LiveChatServerEvent::Typing {
            actor,
            is_typing,
            expires_at,
        } => {
            writer.write_u8(SERVER_TYPING_SET);
            writer.write_time(*expires_at);
            writer.write_u8(if *is_typing { 1 } else { 0 });
            if *is_typing {
                writer.write_actor(actor)?;
            }
        }
        LiveChatServerEvent::Presence { connected_count } => {
            writer.write_u8(SERVER_PRESENCE);
            writer.write_u32(saturating_u32(*connected_count));
        }
        LiveChatServerEvent::HeartbeatAck { nonce } => {
            writer.write_u8(SERVER_PONG);
            let parsed_nonce = nonce.parse::<u64>().map_or(0, |value| value);
            writer.write_u64(parsed_nonce);
        }
        LiveChatServerEvent::Error { code, message } => {
            writer.write_u8(SERVER_ERROR);
            writer.write_string(code)?;
            writer.write_string(message)?;
        }
    }

    Ok(writer.into_inner())
}

#[derive(Default)]
struct BinaryWriter {
    bytes: Vec<u8>,
}

impl BinaryWriter {
    fn into_inner(self) -> Vec<u8> {
        self.bytes
    }

    fn write_u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    fn write_u16(&mut self, value: u16) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn write_u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn write_u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn write_i64(&mut self, value: i64) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn write_time(&mut self, value: DateTime<Utc>) {
        self.write_i64(value.timestamp_millis());
    }

    fn write_uuid(&mut self, value: Uuid) {
        self.bytes.extend_from_slice(value.as_bytes());
    }

    fn write_uuid_string(&mut self, value: &str) -> anyhow::Result<()> {
        let uuid = Uuid::parse_str(value)?;
        self.write_uuid(uuid);
        Ok(())
    }

    fn write_string(&mut self, value: &str) -> anyhow::Result<()> {
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

    fn write_actor(&mut self, actor: &ChatActor) -> anyhow::Result<()> {
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

    fn write_message(&mut self, message: &CachedChatMessage) -> anyhow::Result<()> {
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

struct BinaryReader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> BinaryReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn finish(&self) -> anyhow::Result<()> {
        if self.offset == self.bytes.len() {
            return Ok(());
        }
        Err(anyhow::anyhow!("Trailing bytes in live chat binary frame"))
    }

    fn read_exact(&mut self, len: usize) -> anyhow::Result<&'a [u8]> {
        let end = self.offset.saturating_add(len);
        if end > self.bytes.len() {
            return Err(anyhow::anyhow!("Truncated live chat binary frame"));
        }
        let slice = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(slice)
    }

    fn read_u8(&mut self) -> anyhow::Result<u8> {
        let bytes = self.read_exact(1)?;
        Ok(bytes[0])
    }

    fn read_u16(&mut self) -> anyhow::Result<u16> {
        let bytes = self.read_exact(2)?;
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    fn read_u64(&mut self) -> anyhow::Result<u64> {
        let bytes = self.read_exact(8)?;
        Ok(u64::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    fn read_uuid(&mut self) -> anyhow::Result<Uuid> {
        let bytes = self.read_exact(16)?;
        let mut raw = [0u8; 16];
        raw.copy_from_slice(bytes);
        Ok(Uuid::from_bytes(raw))
    }

    fn read_string(&mut self) -> anyhow::Result<String> {
        let len = self.read_u16()?;
        if len == NONE_STRING_LEN {
            return Err(anyhow::anyhow!(
                "Unexpected null string in live chat binary frame"
            ));
        }
        let bytes = self.read_exact(len as usize)?;
        String::from_utf8(bytes.to_vec()).map_err(|e| anyhow::anyhow!(e))
    }
}

fn saturating_u8<T>(value: T) -> u8
where
    T: TryInto<u8>,
{
    value.try_into().map_or(u8::MAX, |value| value)
}

fn saturating_u16<T>(value: T) -> u16
where
    T: TryInto<u16>,
{
    value.try_into().map_or(u16::MAX, |value| value)
}

fn saturating_u32<T>(value: T) -> u32
where
    T: TryInto<u32>,
{
    value.try_into().map_or(u32::MAX, |value| value)
}

#[cfg(test)]
mod tests {
    use std::{net::IpAddr, str::FromStr};

    use chrono::{TimeZone, Utc};
    use uuid::Uuid;

    use super::*;
    use crate::domain::live_chat::{
        cache::{CachedChatMessage, ChatActor},
        guest_nickname::guest_nickname_for_ip,
        message::LIVE_CHAT_SENDER_KIND_GUEST,
    };

    const CLIENT_MESSAGE_ID: &str = "018f3f7d-5a76-7d8f-8123-456789abcdef";
    const MESSAGE_ID: &str = "018f3f7d-5a76-7d8f-8123-456789abc001";

    #[test]
    fn decodes_send_message_client_frame() {
        let mut frame = Vec::new();
        frame.push(CLIENT_SEND_MESSAGE);
        frame.extend_from_slice(uuid(CLIENT_MESSAGE_ID).as_bytes());
        frame.extend_from_slice(&5u16.to_be_bytes());
        frame.extend_from_slice("hello".as_bytes());

        let event = match decode_client_event(&frame) {
            Ok(event) => event,
            Err(e) => panic!("expected client frame to decode: {e}"),
        };

        match event {
            LiveChatBinaryClientEvent::SendMessage {
                client_message_id,
                body,
            } => {
                assert_eq!(client_message_id, CLIENT_MESSAGE_ID);
                assert_eq!(body, "hello");
            }
            _ => panic!("expected send message event"),
        }
    }

    #[test]
    fn decodes_typing_and_ping_client_frames() {
        let start = match decode_client_event(&[CLIENT_TYPING_START]) {
            Ok(event) => event,
            Err(e) => panic!("expected typing start to decode: {e}"),
        };
        let stop = match decode_client_event(&[CLIENT_TYPING_STOP]) {
            Ok(event) => event,
            Err(e) => panic!("expected typing stop to decode: {e}"),
        };
        let mut ping_frame = vec![CLIENT_PING];
        ping_frame.extend_from_slice(&42u64.to_be_bytes());
        let ping = match decode_client_event(&ping_frame) {
            Ok(event) => event,
            Err(e) => panic!("expected ping to decode: {e}"),
        };

        assert!(matches!(
            start,
            LiveChatBinaryClientEvent::Typing { is_typing: true }
        ));
        assert!(matches!(
            stop,
            LiveChatBinaryClientEvent::Typing { is_typing: false }
        ));
        assert!(matches!(
            ping,
            LiveChatBinaryClientEvent::Heartbeat { nonce } if nonce == "42"
        ));
    }

    #[test]
    fn rejects_trailing_and_truncated_client_frames() {
        assert!(decode_client_event(&[CLIENT_TYPING_START, 0]).is_err());
        assert!(decode_client_event(&[CLIENT_SEND_MESSAGE, 1, 2, 3]).is_err());
        assert!(decode_client_event(&[0xff]).is_err());
    }

    #[test]
    fn encodes_presence_and_error_server_frames() {
        let presence = match encode_server_event(&LiveChatServerEvent::Presence {
            connected_count: 513,
        }) {
            Ok(frame) => frame,
            Err(e) => panic!("expected presence frame to encode: {e}"),
        };
        assert_eq!(presence, vec![SERVER_PRESENCE, 0, 0, 2, 1]);

        let error = match encode_server_event(&LiveChatServerEvent::Error {
            code: "bad".to_string(),
            message: "nope".to_string(),
        }) {
            Ok(frame) => frame,
            Err(e) => panic!("expected error frame to encode: {e}"),
        };
        assert_eq!(error[0], SERVER_ERROR);
        assert_eq!(&error[1..3], &3u16.to_be_bytes());
        assert_eq!(&error[3..6], b"bad");
        assert_eq!(&error[6..8], &4u16.to_be_bytes());
        assert_eq!(&error[8..12], b"nope");
    }

    #[test]
    fn encodes_binary_ipv4_and_ipv6_addresses() {
        let ipv4_message = cached_guest_message(ip("203.0.113.9"));
        let ipv4_frame = match encode_server_event(&LiveChatServerEvent::Message {
            message: ipv4_message,
        }) {
            Ok(frame) => frame,
            Err(e) => panic!("expected IPv4 message frame to encode: {e}"),
        };
        assert!(contains_subsequence(&ipv4_frame, &[IP_V4, 203, 0, 113, 9]));

        let ipv6_message = cached_guest_message(ip("2001:db8::1"));
        let ipv6_frame = match encode_server_event(&LiveChatServerEvent::Message {
            message: ipv6_message,
        }) {
            Ok(frame) => frame,
            Err(e) => panic!("expected IPv6 message frame to encode: {e}"),
        };
        assert!(contains_subsequence(
            &ipv6_frame,
            &[IP_V6, 0x20, 0x01, 0x0d, 0xb8]
        ));
    }

    #[test]
    fn encodes_hello_with_actor_and_recent_messages() {
        let actor = ChatActor::guest(ip("198.51.100.7"), Some("🇺🇸".to_string()));
        let message = cached_guest_message(ip("198.51.100.7"));
        let frame = match encode_server_event(&LiveChatServerEvent::Hello {
            actor,
            recent_messages: vec![message],
            connected_count: 3,
        }) {
            Ok(frame) => frame,
            Err(e) => panic!("expected hello frame to encode: {e}"),
        };

        assert_eq!(frame[0], SERVER_HELLO);
        assert!(contains_subsequence(&frame, &[IP_V4, 198, 51, 100, 7]));
        assert!(contains_subsequence(&frame, &3u32.to_be_bytes()));
        assert!(contains_subsequence(&frame, &1u16.to_be_bytes()));
    }

    fn cached_guest_message(guest_ip: IpAddr) -> CachedChatMessage {
        CachedChatMessage {
            live_chat_message_id: uuid(MESSAGE_ID),
            room_key: "main".to_string(),
            user_id: None,
            guest_ip: Some(guest_ip),
            sender_kind: LIVE_CHAT_SENDER_KIND_GUEST,
            sender_display_name: guest_nickname_for_ip(guest_ip),
            sender_country_flag: Some("🇺🇸".to_string()),
            user_profile_picture_url: None,
            message_body: "hello".to_string(),
            message_created_at: match Utc.timestamp_millis_opt(1_700_000_000_000).single() {
                Some(value) => value,
                None => panic!("valid timestamp expected"),
            },
            message_edited_at: None,
            message_deleted_at: None,
        }
    }

    fn uuid(value: &str) -> Uuid {
        match Uuid::parse_str(value) {
            Ok(uuid) => uuid,
            Err(e) => panic!("valid UUID expected: {e}"),
        }
    }

    fn ip(value: &str) -> IpAddr {
        match IpAddr::from_str(value) {
            Ok(ip) => ip,
            Err(e) => panic!("valid IP expected: {e}"),
        }
    }

    fn contains_subsequence(haystack: &[u8], needle: &[u8]) -> bool {
        if needle.is_empty() {
            return true;
        }
        haystack
            .windows(needle.len())
            .any(|window| window == needle)
    }
}
