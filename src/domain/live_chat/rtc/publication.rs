//! Per-publisher media forwarding: copy RTP from a remote track into a local
//! fan-out track that every subscriber's peer connection is bound to.

use std::sync::Arc;

use tracing::debug;
use webrtc::rtp_transceiver::rtp_sender::RTCRtpSender;
use webrtc::track::track_local::TrackLocalWriter;
use webrtc::track::track_local::track_local_static_rtp::TrackLocalStaticRTP;
use webrtc::track::track_remote::TrackRemote;

/// Spawn a task that copies RTP packets from a remote (published) track into a
/// local fan-out track. The local track is added to every subscriber's peer
/// connection, so a single write reaches all of them. The task exits when the
/// remote track ends (publisher left/unpublished) or the local track closes.
pub fn spawn_rtp_forward(remote: Arc<TrackRemote>, local: Arc<TrackLocalStaticRTP>) {
    tokio::spawn(async move {
        loop {
            match remote.read_rtp().await {
                Ok((packet, _attributes)) => {
                    if let Err(e) = local.write_rtp(&packet).await {
                        debug!(error = %e, "RTP forward write ended");
                        break;
                    }
                }
                Err(e) => {
                    debug!(error = %e, "RTP forward read ended");
                    break;
                }
            }
        }
    });
}

/// Spawn a task that drains RTCP from an RTP sender. Reading is required so the
/// sender's interceptor pipeline runs (RTCP reports, retransmission, etc.); the
/// packets themselves are not acted on yet. Exits when the sender closes.
pub fn spawn_rtcp_drain(sender: Arc<RTCRtpSender>) {
    tokio::spawn(async move { while sender.read_rtcp().await.is_ok() {} });
}
