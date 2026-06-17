//! Periodic eviction of terminal/stuck photograph batch sessions.
//!
//! `photograph_batches` would otherwise grow one `Arc<BatchSession>` per upload
//! for the process lifetime (an unbounded runtime cache). `prune_terminal_batches`
//! drops batches that finished long enough ago for any poller to have observed
//! the result, plus a hard cap for wedged sessions, and removes their temp dirs.

use std::sync::Arc;

use chrono::Utc;

use crate::init::state::ServerState;

pub async fn prune_photograph_batches(state: Arc<ServerState>) {
    let now = Utc::now();
    let evicted = state.prune_terminal_batches(now).await;
    if evicted > 0 {
        tracing::info!(
            evicted_batches = evicted,
            "Pruned terminal/stuck photograph batch sessions"
        );
    }
}
