//! SDP exchange and the coalescing renegotiation state machine for [`RtcPeer`].
//!
//! Split out of `peer.rs` to keep that file under the size limit. As a descendant
//! of `rtc::peer` this module may access `RtcPeer`'s private fields.

use std::sync::Arc;
use std::time::Duration;

use tokio::time::Instant;
use tracing::{error, warn};
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

use super::super::signal::RtcServerSignal;
use super::RtcPeer;

/// How long an unanswered SFU offer stays authoritative. Past this the offer is
/// treated as stale and replaced on the next renegotiation, so a client that
/// never returns an answer (backgrounded tab, lost answer) cannot permanently
/// wedge the peer's negotiation.
const STALE_OFFER_TIMEOUT: Duration = Duration::from_secs(10);

impl RtcPeer {
    /// Apply the client's join offer and produce the SFU answer SDP.
    pub async fn answer_join_offer(&self, offer_sdp: String) -> Option<String> {
        let offer = match RTCSessionDescription::offer(offer_sdp) {
            Ok(offer) => offer,
            Err(e) => {
                error!(error = %e, "Invalid join offer SDP");
                return None;
            }
        };
        if let Err(e) = self.pc.set_remote_description(offer).await {
            error!(error = %e, "set_remote_description(offer) failed");
            return None;
        }
        let answer = match self.pc.create_answer(None).await {
            Ok(answer) => answer,
            Err(e) => {
                error!(error = %e, "create_answer failed");
                return None;
            }
        };
        if let Err(e) = self.pc.set_local_description(answer.clone()).await {
            error!(error = %e, "set_local_description(answer) failed");
            return None;
        }
        Some(answer.sdp)
    }

    /// Start (or coalesce) an SFU-initiated renegotiation offer. If an offer is
    /// already outstanding and still fresh, the request is marked pending and
    /// replayed when the answer arrives; if the outstanding offer has gone stale
    /// (never answered) it is replaced.
    pub async fn renegotiate(self: &Arc<Self>) {
        {
            let mut state = self.negotiation.lock().await;
            if state.making_offer {
                let stale = state
                    .offer_at
                    .map(|sent_at| sent_at.elapsed() >= STALE_OFFER_TIMEOUT)
                    .unwrap_or(true);
                if !stale {
                    state.pending = true;
                    return;
                }
                warn!(
                    connection_id = %self.connection_id,
                    "Replacing stale unanswered renegotiation offer"
                );
            }
            state.making_offer = true;
            state.offer_at = Some(Instant::now());
        }

        let offer = match self.pc.create_offer(None).await {
            Ok(offer) => offer,
            Err(e) => {
                error!(error = %e, "create_offer (renegotiation) failed");
                self.clear_making_offer().await;
                return;
            }
        };
        if let Err(e) = self.pc.set_local_description(offer.clone()).await {
            error!(error = %e, "set_local_description (renegotiation offer) failed");
            self.clear_making_offer().await;
            return;
        }
        let _ = self
            .signal_tx
            .send(RtcServerSignal::Offer { sdp: offer.sdp })
            .await;
    }

    /// Apply a client answer to an SFU renegotiation offer, then replay a
    /// pending renegotiation if one was requested while the offer was in flight.
    pub async fn accept_answer(self: &Arc<Self>, sdp: String) {
        match RTCSessionDescription::answer(sdp) {
            Ok(answer) => {
                if let Err(e) = self.pc.set_remote_description(answer).await {
                    error!(error = %e, "set_remote_description(answer) failed");
                } else {
                    // Subscriptions added in this negotiation round are live
                    // now; ask their publishers for the keyframes that make
                    // the video decodable.
                    self.request_pending_keyframes().await;
                }
            }
            Err(e) => error!(error = %e, "Invalid renegotiation answer SDP"),
        }

        let pending = {
            let mut state = self.negotiation.lock().await;
            state.making_offer = false;
            state.offer_at = None;
            std::mem::take(&mut state.pending)
        };
        if pending {
            self.renegotiate().await;
        }
    }

    async fn clear_making_offer(&self) {
        let mut state = self.negotiation.lock().await;
        state.making_offer = false;
        state.offer_at = None;
    }
}
