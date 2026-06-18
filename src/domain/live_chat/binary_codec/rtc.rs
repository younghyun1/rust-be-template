//! Binary codec for the WebRTC signaling sub-frames carried after the
//! `CLIENT_RTC` (0x05) and `SERVER_RTC` (0x90) opcodes. Split from the parent
//! codec module to keep file sizes within the project limit; the parent's
//! `decode_client_event` / `encode_server_event` dispatch into these.

use super::reader::BinaryReader;
use super::saturating::saturating_u8;
use super::writer::BinaryWriter;
use crate::domain::live_chat::rtc::{
    RtcClientSignal, RtcIceCandidate, RtcPeerPhase, RtcServerSignal,
};

// RTC client sub-opcodes (second byte after CLIENT_RTC).
const RTC_C_JOIN: u8 = 0x01;
const RTC_C_ANSWER: u8 = 0x02;
const RTC_C_ICE: u8 = 0x03;
const RTC_C_LEAVE: u8 = 0x04;
const RTC_C_MEDIA_STATE: u8 = 0x05;

// RTC server sub-opcodes (second byte after SERVER_RTC).
const RTC_S_ANSWER: u8 = 0x01;
const RTC_S_OFFER: u8 = 0x02;
const RTC_S_ICE: u8 = 0x03;
const RTC_S_PEER_STATE: u8 = 0x04;
const RTC_S_ROSTER: u8 = 0x05;
const RTC_S_ERROR: u8 = 0x06;

const RTC_PHASE_LEFT: u8 = 0x00;
const RTC_PHASE_JOINED: u8 = 0x01;

/// Decode an RTC client signal sub-frame (the bytes after `CLIENT_RTC`).
pub(super) fn decode_rtc_client_signal(
    reader: &mut BinaryReader,
) -> anyhow::Result<RtcClientSignal> {
    let sub_op = reader.read_u8()?;
    match sub_op {
        RTC_C_JOIN => {
            let want_audio = reader.read_u8()? != 0;
            let want_video = reader.read_u8()? != 0;
            let sdp = reader.read_string()?;
            Ok(RtcClientSignal::Join {
                sdp,
                want_audio,
                want_video,
            })
        }
        RTC_C_ANSWER => {
            let sdp = reader.read_string()?;
            Ok(RtcClientSignal::Answer { sdp })
        }
        RTC_C_ICE => {
            let candidate = reader.read_string()?;
            let sdp_mid = if reader.read_u8()? != 0 {
                Some(reader.read_string()?)
            } else {
                None
            };
            let sdp_mline_index = if reader.read_u8()? != 0 {
                Some(reader.read_u16()?)
            } else {
                None
            };
            Ok(RtcClientSignal::Ice(RtcIceCandidate {
                candidate,
                sdp_mid,
                sdp_mline_index,
            }))
        }
        RTC_C_LEAVE => Ok(RtcClientSignal::Leave),
        RTC_C_MEDIA_STATE => {
            let mic_on = reader.read_u8()? != 0;
            let cam_on = reader.read_u8()? != 0;
            Ok(RtcClientSignal::MediaState { mic_on, cam_on })
        }
        _ => Err(anyhow::anyhow!("Unknown RTC client sub-opcode")),
    }
}

/// Encode an RTC server signal sub-frame (written after `SERVER_RTC`).
pub(super) fn encode_rtc_server_signal(
    writer: &mut BinaryWriter,
    signal: &RtcServerSignal,
) -> anyhow::Result<()> {
    match signal {
        RtcServerSignal::Answer { sdp } => {
            writer.write_u8(RTC_S_ANSWER);
            writer.write_string(sdp)?;
        }
        RtcServerSignal::Offer { sdp } => {
            writer.write_u8(RTC_S_OFFER);
            writer.write_string(sdp)?;
        }
        RtcServerSignal::Ice(candidate) => {
            writer.write_u8(RTC_S_ICE);
            write_rtc_ice(writer, candidate)?;
        }
        RtcServerSignal::PeerState {
            actor,
            phase,
            mic_on,
            cam_on,
        } => {
            writer.write_u8(RTC_S_PEER_STATE);
            writer.write_actor(actor)?;
            writer.write_u8(encode_phase(*phase));
            writer.write_u8(u8::from(*mic_on));
            writer.write_u8(u8::from(*cam_on));
        }
        RtcServerSignal::Roster { participants } => {
            writer.write_u8(RTC_S_ROSTER);
            writer.write_u8(saturating_u8(participants.len()));
            for participant in participants.iter().take(u8::MAX as usize) {
                writer.write_actor(&participant.actor)?;
                writer.write_u8(u8::from(participant.mic_on));
                writer.write_u8(u8::from(participant.cam_on));
            }
        }
        RtcServerSignal::Error { code, message } => {
            writer.write_u8(RTC_S_ERROR);
            writer.write_string(code)?;
            writer.write_string(message)?;
        }
    }
    Ok(())
}

fn write_rtc_ice(writer: &mut BinaryWriter, candidate: &RtcIceCandidate) -> anyhow::Result<()> {
    writer.write_string(&candidate.candidate)?;
    match &candidate.sdp_mid {
        Some(sdp_mid) => {
            writer.write_u8(1);
            writer.write_string(sdp_mid)?;
        }
        None => writer.write_u8(0),
    }
    match candidate.sdp_mline_index {
        Some(index) => {
            writer.write_u8(1);
            writer.write_u16(index);
        }
        None => writer.write_u8(0),
    }
    Ok(())
}

fn encode_phase(phase: RtcPeerPhase) -> u8 {
    match phase {
        RtcPeerPhase::Joined => RTC_PHASE_JOINED,
        RtcPeerPhase::Left => RTC_PHASE_LEFT,
    }
}

#[cfg(test)]
mod tests {
    use std::net::IpAddr;
    use std::str::FromStr;

    use super::super::{
        CLIENT_RTC, LiveChatBinaryClientEvent, SERVER_RTC, decode_client_event, encode_server_event,
    };
    use super::*;
    use crate::domain::live_chat::cache::{ChatActor, LiveChatServerEvent};

    fn ip(value: &str) -> IpAddr {
        match IpAddr::from_str(value) {
            Ok(ip) => ip,
            Err(e) => panic!("valid IP expected: {e}"),
        }
    }

    #[test]
    fn decodes_rtc_join_client_frame() {
        let sdp = "v=0\r\no=- 1 1 IN IP4 0.0.0.0\r\n";
        let mut frame = vec![CLIENT_RTC, RTC_C_JOIN, 1, 0];
        frame.extend_from_slice(&(sdp.len() as u16).to_be_bytes());
        frame.extend_from_slice(sdp.as_bytes());

        let event = match decode_client_event(&frame) {
            Ok(event) => event,
            Err(e) => panic!("expected rtc join to decode: {e}"),
        };
        match event {
            LiveChatBinaryClientEvent::Rtc(RtcClientSignal::Join {
                sdp: decoded_sdp,
                want_audio,
                want_video,
            }) => {
                assert_eq!(decoded_sdp, sdp);
                assert!(want_audio);
                assert!(!want_video);
            }
            _ => panic!("expected rtc join event"),
        }
    }

    #[test]
    fn decodes_rtc_ice_client_frame_with_optional_fields() {
        let candidate = "candidate:1 1 udp 2122260223 192.0.2.1 54321 typ host";
        let sdp_mid = "0";
        let mut frame = vec![CLIENT_RTC, RTC_C_ICE];
        frame.extend_from_slice(&(candidate.len() as u16).to_be_bytes());
        frame.extend_from_slice(candidate.as_bytes());
        frame.push(1);
        frame.extend_from_slice(&(sdp_mid.len() as u16).to_be_bytes());
        frame.extend_from_slice(sdp_mid.as_bytes());
        frame.push(0);

        let event = match decode_client_event(&frame) {
            Ok(event) => event,
            Err(e) => panic!("expected rtc ice to decode: {e}"),
        };
        match event {
            LiveChatBinaryClientEvent::Rtc(RtcClientSignal::Ice(candidate_decoded)) => {
                assert_eq!(candidate_decoded.candidate, candidate);
                assert_eq!(candidate_decoded.sdp_mid.as_deref(), Some("0"));
                assert!(candidate_decoded.sdp_mline_index.is_none());
            }
            _ => panic!("expected rtc ice event"),
        }
    }

    #[test]
    fn encodes_rtc_answer_server_frame() {
        let sdp = "v=0\r\na=answer\r\n";
        let frame = match encode_server_event(&LiveChatServerEvent::Rtc(RtcServerSignal::Answer {
            sdp: sdp.to_string(),
        })) {
            Ok(frame) => frame,
            Err(e) => panic!("expected rtc answer to encode: {e}"),
        };
        assert_eq!(frame[0], SERVER_RTC);
        assert_eq!(frame[1], RTC_S_ANSWER);
        assert_eq!(&frame[2..4], &(sdp.len() as u16).to_be_bytes());
        assert_eq!(&frame[4..], sdp.as_bytes());
    }

    #[test]
    fn encodes_rtc_peer_state_server_frame() {
        let actor = ChatActor::guest(ip("203.0.113.5"), Some("🇺🇸".to_string()));
        let frame =
            match encode_server_event(&LiveChatServerEvent::Rtc(RtcServerSignal::PeerState {
                actor,
                phase: RtcPeerPhase::Joined,
                mic_on: true,
                cam_on: false,
            })) {
                Ok(frame) => frame,
                Err(e) => panic!("expected rtc peer state to encode: {e}"),
            };
        assert_eq!(frame[0], SERVER_RTC);
        assert_eq!(frame[1], RTC_S_PEER_STATE);
        // Trailing three bytes: phase (joined=1), mic_on (1), cam_on (0).
        let tail = &frame[frame.len() - 3..];
        assert_eq!(tail, &[RTC_PHASE_JOINED, 1, 0]);
    }
}
