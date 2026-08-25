//! `webrtc` driver events for one peer connection.

use std::sync::{Arc, Weak};

use rtc::media_stream::MediaStreamTrack;
use rtc::rtp_transceiver::rtp_sender::RtpCodecKind;
use tokio::sync::Mutex;
use tracing::{debug, warn};
use webrtc::media_stream::track_remote::TrackRemote;
use webrtc::peer_connection::{
    PeerConnectionEventHandler, RTCPeerConnectionIceEvent, RTCPeerConnectionState,
};

use super::{RtcPeer, actor_stream_id};
use crate::domain::live_chat::rtc::publication::{RtcPublication, spawn_rtp_publish};
use crate::domain::live_chat::rtc::room::RtcRoom;
use crate::domain::live_chat::rtc::signal::{MediaKind, RtcIceCandidate, RtcServerSignal};

#[derive(Clone)]
struct EventContext {
    peer: Weak<RtcPeer>,
    room: Weak<RtcRoom>,
}

/// Construction-time event handler whose peer context is attached before SDP exchange.
#[derive(Default)]
pub(crate) struct RtcPeerEventHandler {
    context: Mutex<Option<EventContext>>,
}

impl RtcPeerEventHandler {
    pub(crate) fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub(crate) async fn attach(&self, peer: Weak<RtcPeer>, room: Weak<RtcRoom>) {
        *self.context.lock().await = Some(EventContext { peer, room });
    }

    async fn context(&self) -> Option<EventContext> {
        self.context.lock().await.clone()
    }
}

#[async_trait::async_trait]
impl PeerConnectionEventHandler for RtcPeerEventHandler {
    async fn on_ice_candidate(&self, event: RTCPeerConnectionIceEvent) {
        let Some(context) = self.context().await else {
            warn!("Received ICE candidate before RTC peer context was attached");
            return;
        };
        let Some(peer) = context.peer.upgrade() else {
            return;
        };
        match event.candidate.to_json() {
            Ok(init) => {
                let signal = RtcServerSignal::Ice(RtcIceCandidate {
                    candidate: init.candidate,
                    sdp_mid: init.sdp_mid,
                    sdp_mline_index: init.sdp_mline_index,
                });
                if let Err(error) = peer.signal_tx.try_send(signal) {
                    debug!(error = %error, "Dropped local ICE candidate");
                }
            }
            Err(error) => warn!(error = %error, "Failed to serialize local ICE candidate"),
        }
    }

    async fn on_track(&self, remote: Arc<dyn TrackRemote>) {
        let Some(context) = self.context().await else {
            warn!("Received remote track before RTC peer context was attached");
            return;
        };
        let Some(peer) = context.peer.upgrade() else {
            return;
        };
        let Some(room) = context.room.upgrade() else {
            return;
        };

        let rtp_kind = remote.kind().await;
        let kind = match rtp_kind {
            RtpCodecKind::Audio => MediaKind::Audio,
            RtpCodecKind::Video => MediaKind::Video,
            _ => return,
        };
        let media_ssrc = match remote.ssrcs().await.into_iter().next() {
            Some(ssrc) => ssrc,
            None => {
                warn!(?kind, "Remote RTC track has no SSRC");
                return;
            }
        };
        let codings = remote.codings().await;
        if codings.is_empty() {
            warn!(?kind, media_ssrc, "Remote RTC track has no RTP coding");
            return;
        }

        let stream_id = actor_stream_id(&peer.actor);
        let track_id = format!(
            "{stream_id}:{}",
            match kind {
                MediaKind::Audio => "audio",
                MediaKind::Video => "video",
            }
        );
        let local_track =
            MediaStreamTrack::new(stream_id, track_id.clone(), track_id, rtp_kind, codings);
        let publication =
            RtcPublication::new(kind, local_track, Arc::downgrade(&remote), media_ssrc);
        if peer
            .publications
            .insert_async(kind, publication.clone())
            .await
            .is_err()
        {
            warn!(?kind, "Ignored duplicate RTC publication");
            return;
        }

        spawn_rtp_publish(remote, publication.clone());
        room.fan_out_track(peer.connection_id, publication).await;
    }

    async fn on_connection_state_change(&self, state: RTCPeerConnectionState) {
        if state != RTCPeerConnectionState::Failed {
            return;
        }
        let Some(context) = self.context().await else {
            return;
        };
        let Some(peer) = context.peer.upgrade() else {
            return;
        };
        if let Some(room) = context.room.upgrade() {
            room.handle_peer_dropped(peer.connection_id).await;
        }
    }
}
