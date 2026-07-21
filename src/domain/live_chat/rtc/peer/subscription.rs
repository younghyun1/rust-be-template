//! Subscription management for [`RtcPeer`]: binding other publishers' fan-out
//! tracks onto this peer's connection and requesting the keyframes that make
//! subscribed video decodable.
//!
//! Split out of `peer.rs` to keep that file under the size limit. As a
//! descendant of `rtc::peer` this module may access `RtcPeer`'s private fields.

use std::sync::Arc;

use tracing::warn;
use webrtc::track::track_local::TrackLocal;

use super::super::publication::{RtcPublication, spawn_rtcp_listen};
use super::super::signal::MediaKind;
use super::RtcPeer;

impl RtcPeer {
    /// Subscribe this peer to a publication. Returns true if the track was
    /// newly added (the caller should then renegotiate this peer). Idempotent
    /// per source track id: a fan-out racing the join-time subscribe cannot
    /// `add_track` the same track twice onto this peer connection.
    pub async fn subscribe_to(&self, publication: Arc<RtcPublication>) -> bool {
        let track_id = publication.track.id().to_string();
        if self
            .subscribed
            .insert_async(track_id.clone())
            .await
            .is_err()
        {
            return false;
        }
        match self
            .pc
            .add_track(publication.track.clone() as Arc<dyn TrackLocal + Send + Sync>)
            .await
        {
            Ok(sender) => {
                // The listener relays this subscriber's PLI/FIR to the
                // publisher; without it a mid-stream subscriber never
                // receives a keyframe and video stays undecodable.
                spawn_rtcp_listen(sender, publication.clone());
                if publication.kind == MediaKind::Video {
                    self.pending_keyframe_requests
                        .lock()
                        .await
                        .push(publication);
                }
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

    /// Request a keyframe for every video publication bound since the last
    /// renegotiation answer. Runs once the client's answer applies, when the
    /// new transceivers are actually receiving; a request fired at `add_track`
    /// time would produce a keyframe that dies before the subscriber is bound.
    /// The RTCP PLI relay backstops any request lost to renegotiation
    /// coalescing.
    pub(super) async fn request_pending_keyframes(&self) {
        let pending = std::mem::take(&mut *self.pending_keyframe_requests.lock().await);
        for publication in pending {
            publication.request_keyframe().await;
        }
    }
}
