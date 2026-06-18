//! WebRTC signaling value types exchanged over the live-chat WebSocket.
//!
//! These are transport-agnostic: the binary codec (`binary_codec.rs`) and the
//! JSON fallback (serde) both encode/decode them. Client signals are inbound
//! only; server signals are outbound. SDP and ICE strings are opaque to the
//! SFU, which forwards media without inspecting codec payloads.

use serde_derive::{Deserialize, Serialize};

use crate::domain::live_chat::cache::ChatActor;

/// Kind of a media track, used for per-publisher track bookkeeping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaKind {
    Audio,
    Video,
}

/// A trickled ICE candidate, mirroring the browser `RTCIceCandidateInit` shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RtcIceCandidate {
    pub candidate: String,
    pub sdp_mid: Option<String>,
    pub sdp_mline_index: Option<u16>,
}

/// Whether a peer-state update marks a join/update or a departure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RtcPeerPhase {
    /// Peer joined the call or updated its media state.
    Joined,
    /// Peer left the call.
    Left,
}

/// One entry in the call roster: who is present and their media state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RtcParticipant {
    pub actor: ChatActor,
    pub mic_on: bool,
    pub cam_on: bool,
}

/// Inbound signaling from a client.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RtcClientSignal {
    /// Join the room call with an initial SDP offer.
    Join {
        sdp: String,
        want_audio: bool,
        want_video: bool,
    },
    /// Answer to an SFU-initiated renegotiation offer.
    Answer { sdp: String },
    /// A trickled ICE candidate for the client's peer connection.
    Ice(RtcIceCandidate),
    /// Leave the call.
    Leave,
    /// Microphone/camera enabled flags changed (no renegotiation).
    MediaState { mic_on: bool, cam_on: bool },
}

/// Outbound signaling to a client.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RtcServerSignal {
    /// SFU answer to the client's initial join offer. Unicast.
    Answer { sdp: String },
    /// SFU-initiated renegotiation offer. Unicast.
    Offer { sdp: String },
    /// A trickled ICE candidate from the SFU. Unicast.
    Ice(RtcIceCandidate),
    /// A peer joined/updated/left the call. Broadcast room-wide.
    PeerState {
        actor: ChatActor,
        phase: RtcPeerPhase,
        mic_on: bool,
        cam_on: bool,
    },
    /// Full roster snapshot sent to a peer on join. Unicast.
    Roster { participants: Vec<RtcParticipant> },
    /// A signaling-level error for the client. Unicast.
    Error { code: String, message: String },
}
