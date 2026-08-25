//! Per-publisher media forwarding and keyframe feedback.
//!
//! `webrtc` 0.20 binds one local track to one peer connection. Each subscriber
//! therefore receives its own local track fed from a bounded RTP broadcast
//! channel, while PLI/FIR feedback is relayed to the publisher's remote track.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Weak};

use rtc::media_stream::MediaStreamTrack;
use rtc::rtcp::payload_feedbacks::full_intra_request::FullIntraRequest;
use rtc::rtcp::payload_feedbacks::picture_loss_indication::PictureLossIndication;
use rtc::rtp::Packet;
use tokio::sync::broadcast;
use tracing::debug;
use webrtc::media_stream::track_local::static_rtp::TrackLocalStaticRTP;
use webrtc::media_stream::track_local::{TrackLocal, TrackLocalEvent};
use webrtc::media_stream::track_remote::{TrackRemote, TrackRemoteEvent};

use super::signal::MediaKind;

/// Number of RTP packets retained for each subscriber before old packets drop.
const RTP_FORWARD_QUEUE: usize = 512;
/// Minimum spacing between keyframe requests forwarded to one publication.
const MIN_KEYFRAME_REQUEST_INTERVAL_MS: u64 = 500;

/// One published track and the route back to its publisher.
pub struct RtcPublication {
    pub kind: MediaKind,
    track: MediaStreamTrack,
    rtp_tx: broadcast::Sender<Packet>,
    publisher_track: Weak<dyn TrackRemote>,
    media_ssrc: u32,
    created_at: tokio::time::Instant,
    last_keyframe_request_ms: AtomicU64,
}

impl RtcPublication {
    /// Create a publication from an inbound remote track.
    pub fn new(
        kind: MediaKind,
        track: MediaStreamTrack,
        publisher_track: Weak<dyn TrackRemote>,
        media_ssrc: u32,
    ) -> Arc<Self> {
        let (rtp_tx, _) = broadcast::channel(RTP_FORWARD_QUEUE);
        Arc::new(Self {
            kind,
            track,
            rtp_tx,
            publisher_track,
            media_ssrc,
            created_at: tokio::time::Instant::now(),
            last_keyframe_request_ms: AtomicU64::new(0),
        })
    }

    /// Stable source track id used to deduplicate subscriptions.
    pub fn track_id(&self) -> &str {
        self.track.track_id()
    }

    /// Build the connection-local track and bounded packet receiver for a subscriber.
    pub fn subscribe(&self) -> (Arc<TrackLocalStaticRTP>, broadcast::Receiver<Packet>) {
        (
            Arc::new(TrackLocalStaticRTP::new(self.track.clone())),
            self.rtp_tx.subscribe(),
        )
    }

    /// Ask the publisher for a keyframe, throttled per publication.
    pub async fn request_keyframe(&self) {
        if self.kind != MediaKind::Video {
            return;
        }
        let publisher_track = match self.publisher_track.upgrade() {
            Some(track) => track,
            None => return,
        };
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
            return;
        }
        let pli = PictureLossIndication {
            sender_ssrc: 0,
            media_ssrc: self.media_ssrc,
        };
        if let Err(error) = publisher_track.write_rtcp(vec![Box::new(pli)]).await {
            debug!(error = %error, "Keyframe request to publisher failed");
        }
    }
}

fn keyframe_request_due(now_ms: u64, last_ms: u64) -> bool {
    last_ms == 0 || now_ms.saturating_sub(last_ms) >= MIN_KEYFRAME_REQUEST_INTERVAL_MS
}

/// Poll one publisher track and distribute its RTP packets to subscribers.
pub fn spawn_rtp_publish(remote: Arc<dyn TrackRemote>, publication: Arc<RtcPublication>) {
    tokio::spawn(async move {
        while let Some(event) = remote.poll().await {
            match event {
                TrackRemoteEvent::OnRtpPacket(packet) => {
                    let _ = publication.rtp_tx.send(packet);
                }
                TrackRemoteEvent::OnEnded | TrackRemoteEvent::OnError => break,
                _ => {}
            }
        }
    });
}

/// Forward packets from a publication into one subscriber-local track.
pub fn spawn_rtp_forward(
    mut packets: broadcast::Receiver<Packet>,
    local: Arc<TrackLocalStaticRTP>,
    publication: Arc<RtcPublication>,
) {
    tokio::spawn(async move {
        let mut forwarding_started = false;
        loop {
            match packets.recv().await {
                Ok(packet) => {
                    if let Err(error) = local.write_rtp(packet).await {
                        if forwarding_started {
                            debug!(error = %error, "Subscriber RTP forwarding ended");
                            break;
                        }
                        debug!(error = %error, "Dropped RTP packet before subscriber track bound");
                        continue;
                    }
                    if !forwarding_started {
                        forwarding_started = true;
                        spawn_rtcp_listen(local.clone(), publication.clone());
                    }
                }
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    debug!(skipped, "Subscriber RTP queue lagged");
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

/// Relay subscriber PLI/FIR feedback to the publisher.
fn spawn_rtcp_listen(local: Arc<TrackLocalStaticRTP>, publication: Arc<RtcPublication>) {
    tokio::spawn(async move {
        while let Some(TrackLocalEvent::OnRtcpPacket(packets)) = local.poll().await {
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
