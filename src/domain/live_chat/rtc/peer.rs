//! A single participant's server-side peer connection in the SFU.
//!
//! Each `RtcPeer` owns one `RTCPeerConnection`, the local fan-out tracks built
//! from the media it publishes, and a per-peer renegotiation state machine. The
//! SFU is always the offerer for renegotiations (peer join/leave); the
//! coalescing `NegotiationState` prevents overlapping offers (glare).

use std::sync::Arc;
use std::sync::Weak;
use std::sync::atomic::{AtomicBool, Ordering};

use tokio::sync::{Mutex, mpsc};
use tokio::time::Instant;
use tracing::{debug, warn};
use uuid::Uuid;
use webrtc::ice_transport::ice_candidate::{RTCIceCandidate, RTCIceCandidateInit};
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::rtp_transceiver::RTCRtpTransceiver;
use webrtc::rtp_transceiver::rtp_codec::RTPCodecType;
use webrtc::rtp_transceiver::rtp_receiver::RTCRtpReceiver;
use webrtc::track::track_local::track_local_static_rtp::TrackLocalStaticRTP;
use webrtc::track::track_remote::TrackRemote;

use super::publication::{RtcPublication, spawn_rtp_forward};
use super::room::RtcRoom;
use super::signal::{MediaKind, RtcIceCandidate, RtcParticipant, RtcServerSignal};
use crate::domain::live_chat::cache::{ChatActor, ChatActorKey};

/// SDP/renegotiation methods live in the child module; they need access to this
/// type's private fields, which descendant modules are permitted.
mod negotiation;
/// Subscription/keyframe methods live in a child module for the same reason.
mod subscription;

/// Stable per-publisher stream id so a browser groups a publisher's audio and
/// video into one `MediaStream` and the frontend can map it back to an actor.
pub fn actor_stream_id(actor: &ChatActor) -> String {
    match &actor.actor_key {
        ChatActorKey::User(user_id) => format!("user:{user_id}"),
        ChatActorKey::Guest(ip) => format!("guest:{ip}"),
    }
}

/// Coalescing renegotiation state. `making_offer` is set while an SFU offer is
/// outstanding (awaiting the client's answer); a renegotiation requested in that
/// window sets `pending` and is replayed once the answer arrives. `offer_at`
/// timestamps the outstanding offer so a never-answered offer (backgrounded tab,
/// lost answer) goes stale and is replaced on the next renegotiation rather than
/// wedging the peer forever.
#[derive(Default)]
struct NegotiationState {
    making_offer: bool,
    pending: bool,
    offer_at: Option<Instant>,
}

/// One participant's peer connection and forwarding state.
pub struct RtcPeer {
    pub connection_id: Uuid,
    pub actor: ChatActor,
    pub participant_id: Uuid,
    pc: Arc<RTCPeerConnection>,
    signal_tx: mpsc::Sender<RtcServerSignal>,
    /// This peer's published media, as fan-out publications others subscribe to.
    publications: scc::HashMap<MediaKind, Arc<RtcPublication>>,
    /// Track ids this peer is already subscribed to, so a fan-out racing the
    /// join-time subscribe cannot `add_track` the same source track twice.
    subscribed: scc::HashSet<String>,
    /// Video publications bound since the last renegotiation answer; each gets
    /// a keyframe request once the answer lands (see `peer/subscription.rs`).
    pending_keyframe_requests: Mutex<Vec<Arc<RtcPublication>>>,
    mic_on: AtomicBool,
    cam_on: AtomicBool,
    negotiation: Mutex<NegotiationState>,
    /// Set once when teardown begins, so the Left broadcast and `pc.close()`
    /// happen exactly once across the WS-disconnect and connection-failed paths.
    torn_down: AtomicBool,
}

impl RtcPeer {
    /// Construct a peer wrapper. Handlers are attached separately so the
    /// callbacks can hold a `Weak` to the constructed `Arc<Self>`.
    pub fn new(
        connection_id: Uuid,
        actor: ChatActor,
        participant_id: Uuid,
        pc: Arc<RTCPeerConnection>,
        signal_tx: mpsc::Sender<RtcServerSignal>,
        want_audio: bool,
        want_video: bool,
    ) -> Arc<Self> {
        Arc::new(Self {
            connection_id,
            actor,
            participant_id,
            pc,
            signal_tx,
            publications: scc::HashMap::new(),
            subscribed: scc::HashSet::new(),
            pending_keyframe_requests: Mutex::new(Vec::new()),
            mic_on: AtomicBool::new(want_audio),
            cam_on: AtomicBool::new(want_video),
            negotiation: Mutex::new(NegotiationState::default()),
            torn_down: AtomicBool::new(false),
        })
    }

    /// Wire ICE, track, and connection-state callbacks. Must be called once on
    /// the `Arc<Self>` before negotiation begins.
    pub fn attach_handlers(self: &Arc<Self>, room: Weak<RtcRoom>) {
        let ice_signal_tx = self.signal_tx.clone();
        self.pc
            .on_ice_candidate(Box::new(move |candidate: Option<RTCIceCandidate>| {
                let ice_signal_tx = ice_signal_tx.clone();
                Box::pin(async move {
                    let candidate = match candidate {
                        Some(candidate) => candidate,
                        None => return,
                    };
                    match candidate.to_json() {
                        Ok(init) => {
                            let signal = RtcServerSignal::Ice(RtcIceCandidate {
                                candidate: init.candidate,
                                sdp_mid: init.sdp_mid,
                                sdp_mline_index: init.sdp_mline_index,
                            });
                            // Non-blocking: a client that stopped reading must not
                            // stall webrtc's ICE task. Trickle ICE tolerates the
                            // rare dropped candidate under backpressure.
                            if let Err(e) = ice_signal_tx.try_send(signal) {
                                debug!(error = %e, "Dropped local ICE candidate (signal queue full/closed)");
                            }
                        }
                        Err(e) => warn!(error = %e, "Failed to serialize local ICE candidate"),
                    }
                })
            }));

        let track_self = Arc::downgrade(self);
        let track_room = room.clone();
        self.pc.on_track(Box::new(
            move |remote: Arc<TrackRemote>,
                  _receiver: Arc<RTCRtpReceiver>,
                  _transceiver: Arc<RTCRtpTransceiver>| {
                let track_self = track_self.clone();
                let track_room = track_room.clone();
                Box::pin(async move {
                    let peer = match track_self.upgrade() {
                        Some(peer) => peer,
                        None => return,
                    };
                    let room = match track_room.upgrade() {
                        Some(room) => room,
                        None => return,
                    };
                    let kind = match remote.kind() {
                        RTPCodecType::Audio => MediaKind::Audio,
                        RTPCodecType::Video => MediaKind::Video,
                        _ => return,
                    };
                    let capability = remote.codec().capability;
                    let stream_id = actor_stream_id(&peer.actor);
                    let track_id = format!(
                        "{stream_id}:{}",
                        match kind {
                            MediaKind::Audio => "audio",
                            MediaKind::Video => "video",
                        }
                    );
                    let local = Arc::new(TrackLocalStaticRTP::new(capability, track_id, stream_id));
                    let publication = RtcPublication::new(
                        kind,
                        local.clone(),
                        Arc::downgrade(&peer.pc),
                        remote.ssrc(),
                    );
                    let _ = peer
                        .publications
                        .insert_async(kind, publication.clone())
                        .await;
                    spawn_rtp_forward(remote, local);
                    room.fan_out_track(peer.connection_id, publication).await;
                })
            },
        ));

        let state_room = room.clone();
        let state_connection_id = self.connection_id;
        self.pc
            .on_peer_connection_state_change(Box::new(move |state: RTCPeerConnectionState| {
                let state_room = state_room.clone();
                Box::pin(async move {
                    if matches!(state, RTCPeerConnectionState::Failed)
                        && let Some(room) = state_room.upgrade()
                    {
                        room.handle_peer_dropped(state_connection_id).await;
                    }
                })
            }));
    }

    /// Add a remote ICE candidate received from the client.
    pub async fn add_ice(&self, candidate: RtcIceCandidate) {
        let init = RTCIceCandidateInit {
            candidate: candidate.candidate,
            sdp_mid: candidate.sdp_mid,
            sdp_mline_index: candidate.sdp_mline_index,
            username_fragment: None,
        };
        if let Err(e) = self.pc.add_ice_candidate(init).await {
            debug!(error = %e, "add_ice_candidate failed");
        }
    }

    /// Record a microphone/camera state change (no renegotiation).
    pub fn set_media_state(&self, mic_on: bool, cam_on: bool) {
        self.mic_on.store(mic_on, Ordering::SeqCst);
        self.cam_on.store(cam_on, Ordering::SeqCst);
    }

    /// Current microphone-enabled flag.
    pub fn mic_on(&self) -> bool {
        self.mic_on.load(Ordering::SeqCst)
    }

    /// Current camera-enabled flag.
    pub fn cam_on(&self) -> bool {
        self.cam_on.load(Ordering::SeqCst)
    }

    /// Claim teardown for this peer. Returns true exactly once (the first caller
    /// across the WS-disconnect and connection-failed paths); later callers get
    /// false, so the Left broadcast and `pc.close()` run a single time.
    pub fn begin_teardown(&self) -> bool {
        self.torn_down
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }

    /// Roster entry for this peer.
    pub fn participant(&self) -> RtcParticipant {
        RtcParticipant {
            actor: self.actor.clone(),
            mic_on: self.mic_on(),
            cam_on: self.cam_on(),
        }
    }

    /// Send a unicast signal to this peer's client.
    pub async fn send_signal(&self, signal: RtcServerSignal) {
        let _ = self.signal_tx.send(signal).await;
    }

    /// Close the underlying peer connection.
    pub async fn close(&self) {
        if let Err(e) = self.pc.close().await {
            debug!(error = %e, "peer connection close failed");
        }
    }
}
