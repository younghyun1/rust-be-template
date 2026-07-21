//! Per-publisher media forwarding: the fan-out track subscribers bind to, the
//! RTP copy task feeding it, and the keyframe-request (PLI) path back to the
//! publisher. Browsers emit video keyframes only at encoder start or on
//! PLI/FIR, and a subscriber always binds mid-stream (after a renegotiation
//! round-trip), so without the request path subscribed video never becomes
//! decodable.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Weak};

use tracing::debug;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::rtcp::payload_feedbacks::full_intra_request::FullIntraRequest;
use webrtc::rtcp::payload_feedbacks::picture_loss_indication::PictureLossIndication;
use webrtc::rtp_transceiver::rtp_sender::RTCRtpSender;
use webrtc::track::track_local::TrackLocalWriter;
use webrtc::track::track_local::track_local_static_rtp::TrackLocalStaticRTP;
use webrtc::track::track_remote::TrackRemote;

use super::signal::MediaKind;

/// Minimum spacing between keyframe requests forwarded to one publication, so
/// several subscribers requesting at once do not stampede the encoder.
const MIN_KEYFRAME_REQUEST_INTERVAL_MS: u64 = 500;

/// One published track: the fan-out RTP track subscribers bind to plus the
/// route back to the publisher used to request keyframes.
pub struct RtcPublication {
    pub kind: MediaKind,
    /// Fan-out track added to every subscriber's peer connection.
    pub track: Arc<TrackLocalStaticRTP>,
    /// Publisher's peer connection; keyframe requests are written to it.
    publisher_pc: Weak<RTCPeerConnection>,
    /// SSRC of the publisher's inbound track, named in the PLI.
    media_ssrc: u32,
    /// Throttle baseline (publication creation time).
    created_at: tokio::time::Instant,
    /// Milliseconds after `created_at` of the last keyframe request; 0 = none.
    last_keyframe_request_ms: AtomicU64,
}

impl RtcPublication {
    /// Wrap a fan-out track with the publisher route used for PLI.
    pub fn new(
        kind: MediaKind,
        track: Arc<TrackLocalStaticRTP>,
        publisher_pc: Weak<RTCPeerConnection>,
        media_ssrc: u32,
    ) -> Arc<Self> {
        Arc::new(Self {
            kind,
            track,
            publisher_pc,
            media_ssrc,
            created_at: tokio::time::Instant::now(),
            last_keyframe_request_ms: AtomicU64::new(0),
        })
    }

    /// Ask the publisher for a keyframe (PLI), throttled per publication.
    /// No-op for audio (Opus frames are independently decodable) and after the
    /// publisher's peer connection is gone.
    pub async fn request_keyframe(&self) {
        if self.kind != MediaKind::Video {
            return;
        }
        let pc = match self.publisher_pc.upgrade() {
            Some(pc) => pc,
            None => return,
        };
        // Millisecond resolution is ample for the gate; max(1) keeps a
        // first-millisecond request from re-reading as "never sent".
        let now_ms = (self.created_at.elapsed().as_millis() as u64).max(1);
        let last = self.last_keyframe_request_ms.load(Ordering::SeqCst);
        if !keyframe_request_due(now_ms, last) {
            return;
        }
        if self
            .last_keyframe_request_ms
            .compare_exchange(last, now_ms, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            // Lost the race to a concurrent request; that one covers us.
            return;
        }
        let pli = PictureLossIndication {
            sender_ssrc: 0,
            media_ssrc: self.media_ssrc,
        };
        if let Err(e) = pc.write_rtcp(&[Box::new(pli)]).await {
            debug!(error = %e, "Keyframe request (PLI) to publisher failed");
        }
    }
}

/// Whether a keyframe request at `now_ms` may proceed given the previous one
/// at `last_ms` (0 meaning "never"). Pure so the gate is unit-testable.
fn keyframe_request_due(now_ms: u64, last_ms: u64) -> bool {
    last_ms == 0 || now_ms.saturating_sub(last_ms) >= MIN_KEYFRAME_REQUEST_INTERVAL_MS
}

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

/// Spawn a task that reads RTCP arriving from one subscriber for a forwarded
/// track. Reading drives the sender's interceptor pipeline, and PLI/FIR from
/// the subscriber is relayed to the publisher as a keyframe request so the
/// subscriber can start (or resume) decoding. Exits when the sender closes.
pub fn spawn_rtcp_listen(sender: Arc<RTCRtpSender>, publication: Arc<RtcPublication>) {
    tokio::spawn(async move {
        while let Ok((packets, _attributes)) = sender.read_rtcp().await {
            let wants_keyframe = packets.iter().any(|packet| {
                packet
                    .as_any()
                    .downcast_ref::<PictureLossIndication>()
                    .is_some()
                    || packet.as_any().downcast_ref::<FullIntraRequest>().is_some()
            });
            if wants_keyframe {
                publication.request_keyframe().await;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_keyframe_request_is_due() {
        assert!(keyframe_request_due(1, 0));
    }

    #[test]
    fn keyframe_requests_are_throttled() {
        assert!(!keyframe_request_due(400, 1));
        assert!(keyframe_request_due(501, 1));
    }

    #[test]
    fn throttle_tolerates_clock_equalities() {
        assert!(!keyframe_request_due(1, 1));
        assert!(keyframe_request_due(
            MIN_KEYFRAME_REQUEST_INTERVAL_MS + 1,
            1
        ));
    }
}
