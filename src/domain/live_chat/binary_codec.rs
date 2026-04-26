use crate::domain::live_chat::cache::LiveChatServerEvent;

mod reader;
mod saturating;
mod writer;

use reader::BinaryReader;
use saturating::{saturating_u8, saturating_u16, saturating_u32};
use writer::BinaryWriter;

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
