//! Per-connection RTC signaling: dispatch client signals to the room/peer and
//! relay the peer's outbound signals onto the connection's writer queue.
//!
//! The SFU mechanics live in `domain::live_chat::rtc`; this module is the glue
//! between a live-chat WebSocket connection and that peer.

use std::net::IpAddr;
use std::sync::Arc;

use tokio::sync::{Mutex, mpsc};
use tracing::error;
use uuid::Uuid;

use crate::domain::live_chat::cache::{ChatActor, DEFAULT_LIVE_CHAT_ROOM, LiveChatServerEvent};
use crate::domain::live_chat::rtc::{
    RtcClientSignal, RtcPeer, RtcPeerPhase, RtcRoom, RtcRoomAcquire, RtcServerSignal,
};
use crate::init::state::ServerState;

use super::LiveChatWireProtocol;
use super::protocol::{OutboundSender, encode_event, send_event};

/// Bound on the per-peer outbound signal channel (SDP/ICE to one client).
const RTC_SIGNAL_QUEUE: usize = 64;

/// Mutable per-connection RTC state, guarded by a mutex on [`RtcSession`].
#[derive(Default)]
struct RtcSessionInner {
    room: Option<Arc<RtcRoom>>,
    peer: Option<Arc<RtcPeer>>,
    participant_id: Option<Uuid>,
}

/// RTC state bound to a single live-chat WebSocket connection.
pub(super) struct RtcSession {
    state: Arc<ServerState>,
    connection_id: Uuid,
    actor: ChatActor,
    client_ip: IpAddr,
    out: OutboundSender,
    wire_protocol: LiveChatWireProtocol,
    inner: Mutex<RtcSessionInner>,
}

impl RtcSession {
    pub(super) fn new(
        state: Arc<ServerState>,
        connection_id: Uuid,
        actor: ChatActor,
        client_ip: IpAddr,
        out: OutboundSender,
        wire_protocol: LiveChatWireProtocol,
    ) -> Self {
        Self {
            state,
            connection_id,
            actor,
            client_ip,
            out,
            wire_protocol,
            inner: Mutex::new(RtcSessionInner::default()),
        }
    }

    /// Route one inbound client signal.
    pub(super) async fn dispatch(&self, signal: RtcClientSignal) {
        match signal {
            RtcClientSignal::Join {
                sdp,
                want_audio,
                want_video,
            } => self.handle_join(sdp, want_audio, want_video).await,
            RtcClientSignal::Answer { sdp } => {
                if let Some(peer) = self.current_peer().await {
                    peer.accept_answer(sdp).await;
                }
            }
            RtcClientSignal::Ice(candidate) => {
                if let Some(peer) = self.current_peer().await {
                    peer.add_ice(candidate).await;
                }
            }
            RtcClientSignal::Leave => self.leave().await,
            RtcClientSignal::MediaState { mic_on, cam_on } => {
                let (room, peer) = self.current_room_peer().await;
                if let (Some(room), Some(peer)) = (room, peer) {
                    peer.set_media_state(mic_on, cam_on);
                    room.broadcast_peer_state(&peer.participant(), RtcPeerPhase::Joined);
                }
            }
        }
    }

    /// Authoritative teardown on WS disconnect: leave the call if joined.
    pub(super) async fn teardown(&self) {
        self.leave().await;
    }

    async fn handle_join(&self, sdp: String, want_audio: bool, want_video: bool) {
        if self.current_peer().await.is_some() {
            // Already in the call; ignore duplicate join.
            return;
        }

        let engine = match self.state.rtc_engine() {
            Some(engine) => engine,
            None => {
                self.send_error("rtc_disabled", "Calls are not available.")
                    .await;
                return;
            }
        };

        if self
            .state
            .live_chat_cache
            .is_banned(self.actor.user_id, self.client_ip)
            .await
        {
            self.send_error("banned", "Live chat access denied.").await;
            return;
        }

        // Acquire reserves a participant slot atomically (enforces the cap and
        // keeps the room alive against concurrent GC). Every failure path below
        // must release_slot() so the reservation is not leaked.
        let room = match self.state.acquire_rtc_room(DEFAULT_LIVE_CHAT_ROOM).await {
            RtcRoomAcquire::Acquired(room) => room,
            RtcRoomAcquire::Full => {
                self.send_error("room_full", "The call is full.").await;
                return;
            }
            RtcRoomAcquire::Unavailable => {
                self.send_error("rtc_unavailable", "Call could not be started.")
                    .await;
                return;
            }
        };

        let pc = match engine.new_peer_connection().await {
            Ok(pc) => pc,
            Err(e) => {
                error!(error = %e, "Failed to create RTC peer connection");
                self.send_error("rtc_unavailable", "Call could not be started.")
                    .await;
                room.release_slot();
                self.state.remove_rtc_room_if_empty(&room.room_key).await;
                return;
            }
        };

        let participant_id = match self
            .state
            .record_call_participant_join(room.call_id, &self.actor, want_audio, want_video)
            .await
        {
            Some(id) => id,
            None => {
                if let Err(e) = pc.close().await {
                    error!(error = %e, "Failed to close peer connection after persist failure");
                }
                self.send_error("persist_failed", "Call could not be started.")
                    .await;
                room.release_slot();
                self.state.remove_rtc_room_if_empty(&room.room_key).await;
                return;
            }
        };

        let (rtc_signal_tx, rtc_signal_rx) = mpsc::channel::<RtcServerSignal>(RTC_SIGNAL_QUEUE);
        self.spawn_signal_relay(rtc_signal_rx);

        let peer = RtcPeer::new(
            self.connection_id,
            self.actor.clone(),
            participant_id,
            pc,
            rtc_signal_tx,
            want_audio,
            want_video,
        );
        peer.attach_handlers(Arc::downgrade(&room));

        let answer = match peer.answer_join_offer(sdp).await {
            Some(answer) => answer,
            None => {
                peer.close().await;
                self.state
                    .record_call_participant_leave(participant_id)
                    .await;
                self.send_error("sdp_failed", "Could not negotiate the call.")
                    .await;
                room.release_slot();
                self.state.remove_rtc_room_if_empty(&room.room_key).await;
                return;
            }
        };
        peer.send_signal(RtcServerSignal::Answer { sdp: answer })
            .await;

        room.register_peer(peer.clone()).await;
        {
            let mut inner = self.inner.lock().await;
            inner.room = Some(room.clone());
            inner.peer = Some(peer.clone());
            inner.participant_id = Some(participant_id);
        }

        let participants = room.roster().await;
        peer.send_signal(RtcServerSignal::Roster { participants })
            .await;
        room.broadcast_peer_state(&peer.participant(), RtcPeerPhase::Joined);

        // Deliver existing publishers to the newcomer (SFU-offered renegotiation).
        room.subscribe_new_peer(peer).await;
    }

    async fn leave(&self) {
        // Take ownership out of the session so the peer Arc can drop after teardown.
        let (room, _peer, participant_id) = {
            let mut inner = self.inner.lock().await;
            (
                inner.room.take(),
                inner.peer.take(),
                inner.participant_id.take(),
            )
        };

        // teardown_peer broadcasts Left, closes the PC, and releases the slot
        // exactly once (idempotent with the connection-failed path).
        if let Some(room) = room.as_ref() {
            room.teardown_peer(self.connection_id).await;
        }

        if let Some(participant_id) = participant_id {
            self.state
                .record_call_participant_leave(participant_id)
                .await;
        }

        if let Some(room) = room {
            self.state.remove_rtc_room_if_empty(&room.room_key).await;
        }
    }

    /// Spawn the task that encodes the peer's outbound signals and enqueues them
    /// on this connection's writer queue. Ends when the peer drops its sender.
    fn spawn_signal_relay(&self, mut rtc_signal_rx: mpsc::Receiver<RtcServerSignal>) {
        let out = self.out.clone();
        let wire_protocol = self.wire_protocol;
        tokio::spawn(async move {
            while let Some(signal) = rtc_signal_rx.recv().await {
                let event = LiveChatServerEvent::Rtc(signal);
                match encode_event(&event, wire_protocol) {
                    Some(message) => {
                        if out.send(message).await.is_err() {
                            break;
                        }
                    }
                    None => continue,
                }
            }
        });
    }

    async fn current_peer(&self) -> Option<Arc<RtcPeer>> {
        self.inner.lock().await.peer.clone()
    }

    async fn current_room_peer(&self) -> (Option<Arc<RtcRoom>>, Option<Arc<RtcPeer>>) {
        let inner = self.inner.lock().await;
        (inner.room.clone(), inner.peer.clone())
    }

    async fn send_error(&self, code: &str, message: &str) {
        let event = LiveChatServerEvent::Rtc(RtcServerSignal::Error {
            code: code.to_string(),
            message: message.to_string(),
        });
        send_event(&self.out, &event, self.wire_protocol).await;
    }
}
