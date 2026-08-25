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
use tracing::debug;
use uuid::Uuid;
use webrtc::peer_connection::{PeerConnection, RTCIceCandidateInit};

use super::publication::RtcPublication;
use super::room::RtcRoom;
use super::signal::{MediaKind, RtcIceCandidate, RtcParticipant, RtcServerSignal};
use crate::domain::live_chat::cache::{ChatActor, ChatActorKey};

/// Driver event handling lives separately because `webrtc` 0.20 installs the
/// handler while constructing the peer connection.
mod events;
/// SDP/renegotiation methods live in the child module; they need access to this
/// type's private fields, which descendant modules are permitted.
mod negotiation;
/// Subscription/keyframe methods live in a child module for the same reason.
mod subscription;

pub(crate) use events::RtcPeerEventHandler;

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
    pc: Arc<dyn PeerConnection>,
    signal_tx: mpsc::Sender<RtcServerSignal>,
    /// This peer's published media, as fan-out publications others subscribe to.
    publications: scc::HashMap<MediaKind, Arc<RtcPublication>>,
    /// Track ids this peer is already subscribed to, so a fan-out racing the
    /// join-time subscribe cannot `add_track` the same source track twice.
    subscribed: scc::HashSet<String>,
    /// Connection-local tracks waiting for the renegotiation answer that binds them.
    pending_subscriptions: Mutex<Vec<subscription::PendingSubscription>>,
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
        pc: Arc<dyn PeerConnection>,
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
            pending_subscriptions: Mutex::new(Vec::new()),
            mic_on: AtomicBool::new(want_audio),
            cam_on: AtomicBool::new(want_video),
            negotiation: Mutex::new(NegotiationState::default()),
            torn_down: AtomicBool::new(false),
        })
    }

    /// Attach the context used by the construction-time event handler.
    pub(crate) async fn attach_handlers(
        self: &Arc<Self>,
        handler: &RtcPeerEventHandler,
        room: Weak<RtcRoom>,
    ) {
        handler.attach(Arc::downgrade(self), room).await;
    }

    /// Add a remote ICE candidate received from the client.
    pub async fn add_ice(&self, candidate: RtcIceCandidate) {
        let init = RTCIceCandidateInit {
            candidate: candidate.candidate,
            sdp_mid: candidate.sdp_mid,
            sdp_mline_index: candidate.sdp_mline_index,
            username_fragment: None,
            url: None,
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
