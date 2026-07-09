//! A room's SFU state: the set of connected peers plus the broadcast handle
//! used to publish roster/peer-state changes to every live-chat connection.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use tokio::sync::broadcast;
use tracing::debug;
use uuid::Uuid;
use webrtc::track::track_local::track_local_static_rtp::TrackLocalStaticRTP;

use super::peer::RtcPeer;
use super::signal::{RtcParticipant, RtcPeerPhase, RtcServerSignal};
use crate::domain::live_chat::cache::LiveChatServerEvent;

/// Result of acquiring a room slot for a join: a reserved room, or a refusal.
pub enum RtcRoomAcquire {
    /// A room with a reserved participant slot (release it on every failure or
    /// on leave).
    Acquired(Arc<RtcRoom>),
    /// The room is at `max_participants`.
    Full,
    /// The SFU is disabled or a backing call row could not be opened.
    Unavailable,
}

/// Per-room SFU state. Created on the first join (which opens a `live_chat_calls`
/// row) and removed when empty (which closes it). `call_id` is the backing row.
///
/// `occupancy` is the count of reserved slots (incremented before the expensive
/// join work, decremented on teardown/failure). It is the authority for both the
/// participant cap and room GC, so a slot reserved before a peer is registered
/// keeps the room alive and counted against the cap across `.await` points.
pub struct RtcRoom {
    pub room_key: String,
    pub call_id: Uuid,
    peers: scc::HashMap<Uuid, Arc<RtcPeer>>,
    broadcast_tx: broadcast::Sender<LiveChatServerEvent>,
    occupancy: AtomicUsize,
    max_participants: usize,
    removed: AtomicBool,
}

impl RtcRoom {
    /// Construct an empty room backed by the `live_chat_calls` row `call_id`.
    pub fn new(
        room_key: String,
        call_id: Uuid,
        broadcast_tx: broadcast::Sender<LiveChatServerEvent>,
        max_participants: usize,
    ) -> Arc<Self> {
        Arc::new(Self {
            room_key,
            call_id,
            peers: scc::HashMap::new(),
            broadcast_tx,
            occupancy: AtomicUsize::new(0),
            max_participants,
            removed: AtomicBool::new(false),
        })
    }

    /// Try to reserve a participant slot, failing if the room is at capacity.
    /// Atomic against concurrent joins, so the cap is enforced (not advisory).
    pub fn try_reserve(&self) -> bool {
        let mut current = self.occupancy.load(Ordering::SeqCst);
        loop {
            if current >= self.max_participants {
                return false;
            }
            match self.occupancy.compare_exchange(
                current,
                current + 1,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => return true,
                Err(observed) => current = observed,
            }
        }
    }

    /// Release a previously reserved slot. Saturating so it never underflows.
    pub fn release_slot(&self) {
        let _ = self
            .occupancy
            .try_update(Ordering::SeqCst, Ordering::SeqCst, |value| {
                value.checked_sub(1)
            });
    }

    /// Current reserved-slot count (peers plus in-flight joins).
    pub fn occupancy(&self) -> usize {
        self.occupancy.load(Ordering::SeqCst)
    }

    /// Mark the room as removed from the registry, so an in-flight acquirer that
    /// already holds this `Arc` releases its slot and retries instead of joining
    /// a GC'd room whose call row was closed.
    pub fn mark_removed(&self) {
        self.removed.store(true, Ordering::SeqCst);
    }

    /// Whether the room has been removed from the registry.
    pub fn is_removed(&self) -> bool {
        self.removed.load(Ordering::SeqCst)
    }

    /// Number of peers currently registered in the room.
    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }

    /// True when no peers remain registered.
    pub fn is_empty(&self) -> bool {
        self.peers.is_empty()
    }

    /// Register a joined peer under its connection id.
    pub async fn register_peer(&self, peer: Arc<RtcPeer>) {
        let _ = self.peers.insert_async(peer.connection_id, peer).await;
    }

    /// Look up a peer by its connection id.
    pub async fn get_peer(&self, connection_id: Uuid) -> Option<Arc<RtcPeer>> {
        self.peers
            .read_async(&connection_id, |_, peer| peer.clone())
            .await
    }

    /// Roster snapshot of all current participants.
    pub async fn roster(&self) -> Vec<RtcParticipant> {
        let mut participants = Vec::new();
        self.peers
            .iter_async(|_, peer| {
                participants.push(peer.participant());
                true
            })
            .await;
        participants
    }

    /// Subscribe a newly joined peer to every existing publisher's current
    /// tracks, then renegotiate the new peer once if anything was added.
    pub async fn subscribe_new_peer(&self, new_peer: Arc<RtcPeer>) {
        let others = self.other_peers(new_peer.connection_id).await;
        let mut added_any = false;
        for other in others {
            for track in other.local_tracks_snapshot().await {
                if new_peer.subscribe_to(track).await {
                    added_any = true;
                }
            }
        }
        if added_any {
            new_peer.renegotiate().await;
        }
    }

    /// Fan a publisher's newly arrived track out to all other peers, then
    /// renegotiate each subscriber that accepted it.
    pub async fn fan_out_track(&self, publisher_id: Uuid, track: Arc<TrackLocalStaticRTP>) {
        let subscribers = self.other_peers(publisher_id).await;
        for subscriber in subscribers {
            if subscriber.subscribe_to(track.clone()).await {
                subscriber.renegotiate().await;
            }
        }
    }

    /// Broadcast a peer-state change (join/update/leave) to every connection.
    pub fn broadcast_peer_state(&self, participant: &RtcParticipant, phase: RtcPeerPhase) {
        let _ = self
            .broadcast_tx
            .send(LiveChatServerEvent::Rtc(RtcServerSignal::PeerState {
                actor: participant.actor.clone(),
                phase,
                mic_on: participant.mic_on,
                cam_on: participant.cam_on,
            }));
    }

    /// Remove a peer from the registry, returning it if it was present.
    pub async fn remove_peer(&self, connection_id: Uuid) -> Option<Arc<RtcPeer>> {
        self.peers
            .remove_async(&connection_id)
            .await
            .map(|(_, peer)| peer)
    }

    /// Remove a peer and run its teardown exactly once: broadcast Left, close the
    /// peer connection, and release its reserved slot. Idempotent across the
    /// WS-disconnect and connection-failed paths (the `begin_teardown` guard and
    /// the single registry removal ensure the broadcast/close/release run once).
    pub async fn teardown_peer(&self, connection_id: Uuid) {
        if let Some(peer) = self.remove_peer(connection_id).await
            && peer.begin_teardown()
        {
            self.broadcast_peer_state(&peer.participant(), RtcPeerPhase::Left);
            peer.close().await;
            self.release_slot();
        }
    }

    /// In-memory teardown for a peer whose connection failed. The authoritative
    /// teardown (with DB participant-leave and room GC) runs on WS disconnect;
    /// this keeps the registry correct if media dies while the socket lingers.
    pub async fn handle_peer_dropped(self: &Arc<Self>, connection_id: Uuid) {
        self.teardown_peer(connection_id).await;
        debug!(
            connection_id = %connection_id,
            room_key = %self.room_key,
            "RTC peer dropped (connection failed)"
        );
    }

    async fn other_peers(&self, exclude: Uuid) -> Vec<Arc<RtcPeer>> {
        let mut peers = Vec::new();
        self.peers
            .iter_async(|_, peer| {
                if peer.connection_id != exclude {
                    peers.push(peer.clone());
                }
                true
            })
            .await;
        peers
    }
}
