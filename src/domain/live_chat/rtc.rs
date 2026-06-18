//! In-process WebRTC SFU for live-chat group calls.
//!
//! One `RTCPeerConnection` per client publishes its mic/camera and subscribes
//! to every other participant; the SFU forwards RTP without transcoding. The
//! signaling rides the existing `/ws/live-chat` socket. See
//! `docs/architecture/be/rtc-sfu.md` for the full design and wire protocol.

mod config;
mod engine;
mod peer;
mod publication;
mod room;
mod signal;

pub use config::{MaxParticipants, RtcConfig, TurnConfig};
pub use engine::RtcEngine;
pub use peer::RtcPeer;
pub use room::{RtcRoom, RtcRoomAcquire};
pub use signal::{
    MediaKind, RtcClientSignal, RtcIceCandidate, RtcParticipant, RtcPeerPhase, RtcServerSignal,
};
