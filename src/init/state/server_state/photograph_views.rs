//! `ServerState` accessors for the in-memory photograph view-count cache.
//!
//! Views are a high-frequency, low-value write: a DB `UPDATE` per detail open is
//! wasteful. Instead each view increments an in-RAM delta under a `tokio::RwLock`
//! ([`record_view`]) and a periodic job ([`flush_photograph_views`]) folds the
//! accumulated deltas into `photographs.photograph_view_count`. The buffer is
//! drained to empty on every flush, so it stays bounded.
//!
//! Loss policy: on a DB *error* the deltas are merged back and retried next
//! window (no loss). Deltas whose `UPDATE` matches zero rows are dropped, not
//! requeued: that only happens once the photograph has been deleted, and
//! requeueing a never-matching id would leak the buffer forever. A process
//! crash loses at most one flush window, acceptable for naive view counts.

use std::collections::HashMap;

use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use tracing::warn;
use uuid::Uuid;

use crate::schema::photographs;

use super::ServerState;

impl ServerState {
    /// Record a single view for `photograph_id` in the RAM buffer and return the
    /// running pending delta (including this view), so the caller can present a
    /// live count of `persisted_base + pending` without a DB round trip.
    pub async fn record_view(&self, photograph_id: Uuid) -> i64 {
        let mut buffer = self.photograph_view_buffer.write().await;
        let entry = buffer.entry(photograph_id).or_insert(0);
        *entry += 1;
        *entry
    }

    /// Drain buffered view deltas and fold them into the persisted counters.
    /// Returns the total number of views flushed.
    pub async fn flush_photograph_views(&self) -> anyhow::Result<u64> {
        let pending: HashMap<Uuid, i64> = {
            let mut buffer = self.photograph_view_buffer.write().await;
            std::mem::take(&mut *buffer)
        };
        if pending.is_empty() {
            return Ok(0);
        }

        let mut conn = match self.get_conn().await {
            Ok(conn) => conn,
            Err(e) => {
                self.requeue_photograph_views(pending).await;
                return Err(e);
            }
        };

        let mut flushed: u64 = 0;
        let mut failed: HashMap<Uuid, i64> = HashMap::new();
        for (photograph_id, delta) in pending {
            if delta <= 0 {
                continue;
            }
            let res = diesel::update(
                photographs::table.filter(photographs::photograph_id.eq(photograph_id)),
            )
            .set(photographs::photograph_view_count.eq(photographs::photograph_view_count + delta))
            .execute(&mut conn)
            .await;
            match res {
                Ok(_) => flushed = flushed.saturating_add(delta as u64),
                Err(e) => {
                    warn!(
                        photograph_id = %photograph_id,
                        error = ?e,
                        "Failed to flush photograph view delta; requeueing"
                    );
                    failed.insert(photograph_id, delta);
                }
            }
        }
        drop(conn);

        if !failed.is_empty() {
            self.requeue_photograph_views(failed).await;
        }

        Ok(flushed)
    }

    /// Merge deltas back into the buffer after a failed flush so no views are lost.
    async fn requeue_photograph_views(&self, pending: HashMap<Uuid, i64>) {
        let mut buffer = self.photograph_view_buffer.write().await;
        for (photograph_id, delta) in pending {
            *buffer.entry(photograph_id).or_insert(0) += delta;
        }
    }
}
