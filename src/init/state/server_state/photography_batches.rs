//! `ServerState` accessors for the in-memory photograph batch-upload tracker.
//!
//! Privacy invariant: [`get_owned_batch`] returns `None` for a batch that is
//! absent *or* not owned by the requester. The status handler maps that single
//! `None` path to a 404 (never 403), because the frontend treats 401/403 as a
//! session failure and hard-redirects to `/login`; a 403 from the status poller
//! would nuke the user's session mid-upload.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::photography::batch::session::BatchSession;
use crate::util::image::batch_pipeline::batch_temp_dir;

use super::ServerState;

/// Batches in a terminal state are retained this long after their last activity
/// so a slow poller can still observe the final result.
const TERMINAL_BATCH_TTL_SECONDS: i64 = 30 * 60;
/// Hard cap: any batch idle beyond this is dropped even if not `done`, guarding
/// against a wedged session leaking memory + temp files forever.
const STUCK_BATCH_TTL_SECONDS: i64 = 6 * 60 * 60;

impl ServerState {
    /// Register a freshly-created batch session.
    pub async fn register_batch(&self, batch: Arc<BatchSession>) {
        let batch_id = batch.batch_id;
        let _ = self.photograph_batches.insert_async(batch_id, batch).await;
    }

    /// Fetch a batch only if it exists and is owned by `requester`.
    ///
    /// Returns `None` for both "absent" and "not owned" so callers cannot
    /// distinguish the two (no enumeration; single 404 path).
    pub async fn get_owned_batch(
        &self,
        batch_id: Uuid,
        requester: Uuid,
    ) -> Option<Arc<BatchSession>> {
        self.photograph_batches
            .read_async(&batch_id, |_, batch| {
                if batch.owner == requester {
                    Some(Arc::clone(batch))
                } else {
                    None
                }
            })
            .await
            .flatten()
    }

    /// All batches currently owned by `requester`.
    pub async fn list_owned_batches(&self, requester: Uuid) -> Vec<Arc<BatchSession>> {
        let mut batches = Vec::new();
        self.photograph_batches
            .iter_async(|_, batch| {
                if batch.owner == requester {
                    batches.push(Arc::clone(batch));
                }
                true
            })
            .await;
        batches
    }

    /// Evict terminal batches idle past the terminal TTL, and any batch idle past
    /// the hard cap. Temp dirs for evicted batches are removed fire-and-forget.
    /// Returns the number of batches evicted.
    pub async fn prune_terminal_batches(&self, now: DateTime<Utc>) -> usize {
        let now_unix = now.timestamp();
        let mut evicted: Vec<Uuid> = Vec::new();

        self.photograph_batches
            .retain_async(|batch_id, batch| {
                let idle = now_unix - batch.last_activity_unix();
                let drop_terminal = batch.is_done() && idle > TERMINAL_BATCH_TTL_SECONDS;
                let drop_stuck = idle > STUCK_BATCH_TTL_SECONDS;
                if drop_terminal || drop_stuck {
                    evicted.push(*batch_id);
                    false
                } else {
                    true
                }
            })
            .await;

        for batch_id in &evicted {
            let dir = batch_temp_dir(*batch_id);
            tokio::spawn(async move {
                let _ = tokio::fs::remove_dir_all(&dir).await;
            });
        }

        evicted.len()
    }
}
