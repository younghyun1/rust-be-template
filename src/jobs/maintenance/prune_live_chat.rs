use std::sync::Arc;

use chrono::Utc;

use crate::init::state::ServerState;

/// Periodic prune of per-actor live-chat in-memory state.
///
/// `message_rate_by_key` and `typing_by_actor` otherwise grow one entry per
/// distinct guest IP / user for the process lifetime (an unbounded runtime cache).
/// The rate window is 1s wide and typing entries carry their own expiry, so any
/// entry older than the sweep interval holds no live signal and is recreated on
/// demand. Running this once per minute bounds both maps to recently-active actors.
pub async fn prune_live_chat_state(state: Arc<ServerState>) {
    let now = Utc::now();
    state.live_chat_cache.clear_expired_rate_windows(now).await;
    state.live_chat_cache.clear_expired_typing(now).await;
    // Drop empty SFU rooms and close their dangling call rows, bounding the
    // `rtc_rooms` registry to rooms with live participants.
    state.prune_empty_rtc_rooms().await;
}
