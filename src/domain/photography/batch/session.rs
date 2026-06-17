//! In-memory, per-user batch-upload session tracker.
//!
//! A [`BatchSession`] holds the live processing state for one multi-file upload.
//! It is stored in `ServerState::photograph_batches` and is intentionally
//! ephemeral: a process restart loses tracking, but any photograph that already
//! reached the `Persisting`/`Completed` stage is durable in S3 + Postgres.
//!
//! Counters are denormalized atomics so status aggregation is O(1); the per-item
//! map is consulted only when serializing the full item list. Terminal-counter
//! bumps are guarded on a real non-terminal -> terminal transition so repeated
//! calls (or a reconciliation sweep) stay idempotent.

use std::sync::atomic::{AtomicI64, AtomicUsize, Ordering};

use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::status::ProcessingStatus;

/// Tracked state for a single photograph within a batch.
#[derive(Debug, Clone)]
pub struct BatchItem {
    pub item_id: Uuid,
    pub original_file_name: Option<String>,
    pub original_size_bytes: u64,
    pub status: ProcessingStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A live batch-upload session owned by a single user.
pub struct BatchSession {
    pub batch_id: Uuid,
    pub owner: Uuid,
    pub created_at: DateTime<Utc>,
    pub total: usize,
    items: scc::HashMap<Uuid, BatchItem>,
    completed: AtomicUsize,
    failed: AtomicUsize,
    last_activity: AtomicI64,
}

impl BatchSession {
    /// Create an empty session. Items are registered via [`register_item`].
    pub fn new(batch_id: Uuid, owner: Uuid, total: usize, now: DateTime<Utc>) -> Self {
        Self {
            batch_id,
            owner,
            created_at: now,
            total,
            items: scc::HashMap::new(),
            completed: AtomicUsize::new(0),
            failed: AtomicUsize::new(0),
            last_activity: AtomicI64::new(now.timestamp()),
        }
    }

    fn touch(&self, now: DateTime<Utc>) {
        self.last_activity.store(now.timestamp(), Ordering::SeqCst);
    }

    /// Insert (or replace) an item record.
    pub async fn register_item(&self, item: BatchItem) {
        let _ = self.items.insert_async(item.item_id, item).await;
    }

    /// Move an item to a non-terminal status. No-op if the item is already
    /// terminal, so a late intermediate update can never overwrite a result.
    pub async fn set_status(&self, item_id: Uuid, status: ProcessingStatus, now: DateTime<Utc>) {
        let _ = self
            .items
            .update_async(&item_id, |_, item| {
                if !item.status.is_terminal() {
                    item.status = status.clone();
                    item.updated_at = now;
                }
            })
            .await;
        self.touch(now);
    }

    /// Mark an item completed. Bumps the completed counter only on a real
    /// non-terminal -> terminal transition (idempotent).
    pub async fn complete_item(
        &self,
        item_id: Uuid,
        photograph_id: Uuid,
        photograph_link: String,
        thumbnail_link: String,
        now: DateTime<Utc>,
    ) {
        let transitioned = self
            .items
            .update_async(&item_id, |_, item| {
                if item.status.is_terminal() {
                    false
                } else {
                    item.status = ProcessingStatus::Completed {
                        photograph_id,
                        photograph_link: photograph_link.clone(),
                        thumbnail_link: thumbnail_link.clone(),
                    };
                    item.updated_at = now;
                    true
                }
            })
            .await
            .unwrap_or(false);
        if transitioned {
            self.completed.fetch_add(1, Ordering::SeqCst);
        }
        self.touch(now);
    }

    /// Mark an item failed. Bumps the failed counter only on a real
    /// non-terminal -> terminal transition (idempotent).
    pub async fn fail_item(&self, item_id: Uuid, reason: String, now: DateTime<Utc>) {
        let transitioned = self
            .items
            .update_async(&item_id, |_, item| {
                if item.status.is_terminal() {
                    false
                } else {
                    item.status = ProcessingStatus::Failed {
                        reason: reason.clone(),
                    };
                    item.updated_at = now;
                    true
                }
            })
            .await
            .unwrap_or(false);
        if transitioned {
            self.failed.fetch_add(1, Ordering::SeqCst);
        }
        self.touch(now);
    }

    pub fn completed_count(&self) -> usize {
        self.completed.load(Ordering::SeqCst)
    }

    pub fn failed_count(&self) -> usize {
        self.failed.load(Ordering::SeqCst)
    }

    /// Items not yet in a terminal state.
    pub fn pending_count(&self) -> usize {
        self.total
            .saturating_sub(self.completed_count() + self.failed_count())
    }

    /// Whether every item has reached a terminal state.
    pub fn is_done(&self) -> bool {
        self.completed_count() + self.failed_count() >= self.total
    }

    /// Unix seconds of the last status mutation.
    pub fn last_activity_unix(&self) -> i64 {
        self.last_activity.load(Ordering::SeqCst)
    }

    /// Snapshot all item records (cloned). Order is unspecified; callers sort.
    pub async fn snapshot_items(&self) -> Vec<BatchItem> {
        let mut items = Vec::with_capacity(self.total);
        self.items
            .iter_async(|_, item| {
                items.push(item.clone());
                true
            })
            .await;
        items
    }
}
