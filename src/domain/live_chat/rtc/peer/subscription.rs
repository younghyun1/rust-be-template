//! Subscription management for [`RtcPeer`]: binding other publishers' fan-out
//! tracks onto this peer's connection and requesting the keyframes that make
//! subscribed video decodable.
//!
//! Split out of `peer.rs` to keep that file under the size limit. As a
//! descendant of `rtc::peer` this module may access `RtcPeer`'s private fields.

use std::sync::Arc;

use rtc::rtp::Packet;
use tokio::sync::broadcast;
use tracing::warn;
use webrtc::media_stream::track_local::TrackLocal;
use webrtc::media_stream::track_local::static_rtp::TrackLocalStaticRTP;

use super::super::publication::{RtcPublication, spawn_rtp_forward};
use super::RtcPeer;

/// A subscriber-local track waiting for its negotiation answer before forwarding starts.
pub(super) struct PendingSubscription {
    publication: Arc<RtcPublication>,
    local: Arc<TrackLocalStaticRTP>,
    packets: broadcast::Receiver<Packet>,
}

impl RtcPeer {
    /// Subscribe this peer to a publication. Returns true if the track was
    /// newly added (the caller should then renegotiate this peer). Idempotent
    /// per source track id: a fan-out racing the join-time subscribe cannot
    /// `add_track` the same track twice onto this peer connection.
    pub async fn subscribe_to(&self, publication: Arc<RtcPublication>) -> bool {
        let track_id = publication.track_id().to_owned();
        if self
            .subscribed
            .insert_async(track_id.clone())
            .await
            .is_err()
        {
            return false;
        }
        let (local, packets) = publication.subscribe();
        match self
            .pc
            .add_track(local.clone() as Arc<dyn TrackLocal>)
            .await
        {
            Ok(_) => {
                self.pending_subscriptions
                    .lock()
                    .await
                    .push(PendingSubscription {
                        publication,
                        local,
                        packets,
                    });
                true
            }
            Err(e) => {
                warn!(error = %e, "add_track (subscribe) failed");
                // Allow a later retry of the same track since the add failed.
                let _ = self.subscribed.remove_async(&track_id).await;
                false
            }
        }
    }

    /// Snapshot of this peer's publications.
    pub async fn publications_snapshot(&self) -> Vec<Arc<RtcPublication>> {
        let mut publications = Vec::new();
        self.publications
            .iter_async(|_, publication| {
                publications.push(publication.clone());
                true
            })
            .await;
        publications
    }

    /// Start forwarding for tracks bound by the latest negotiation answer.
    pub(super) async fn activate_pending_subscriptions(&self) {
        let pending = std::mem::take(&mut *self.pending_subscriptions.lock().await);
        for subscription in pending {
            spawn_rtp_forward(
                subscription.packets,
                subscription.local,
                subscription.publication.clone(),
            );
            subscription.publication.request_keyframe().await;
        }
    }
}
