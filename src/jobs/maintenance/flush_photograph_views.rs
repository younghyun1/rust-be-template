//! Periodic flush of buffered photograph view counts to the database.
//!
//! Detail-page opens accumulate view deltas in `ServerState::photograph_view_buffer`
//! (see `init::state::server_state::photograph_views`). This job folds them into
//! `photographs.photograph_view_count` so the read path never writes per view.

use std::sync::Arc;

use tracing::error;

use crate::init::state::ServerState;

pub async fn flush_photograph_views(state: Arc<ServerState>) {
    match state.flush_photograph_views().await {
        Ok(_) => {}
        Err(e) => {
            error!(error = ?e, "Failed to flush photograph view counts");
        }
    }
}
