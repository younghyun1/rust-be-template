//! Processing status for a single photograph inside a batch-upload session.
//!
//! Mirrors the serde-tagged-enum convention used by live-chat events
//! (`src/domain/live_chat/cache/event.rs`): the discriminant is serialized into
//! a `status` field, struct-variant fields are inlined alongside it.

use serde_derive::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Lifecycle of a single photograph as it moves through the batch pipeline.
///
/// `Queued` -> `Encoding` -> `Uploading` -> `Persisting` ->
/// (`Completed` | `Failed`). `Completed` and `Failed` are terminal.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ProcessingStatus {
    /// Staged on disk, awaiting a worker permit.
    Queued,
    /// Decoding + resizing + AVIF encoding (CPU work on the blocking pool).
    Encoding,
    /// Pushing the encoded main image and thumbnail to S3.
    Uploading,
    /// Inserting the photograph row into the database.
    Persisting,
    /// Finished successfully; carries the persisted identifiers/links.
    Completed {
        photograph_id: Uuid,
        photograph_link: String,
        thumbnail_link: String,
    },
    /// Terminated with an error; `reason` is a human-readable summary.
    Failed { reason: String },
}

impl ProcessingStatus {
    /// Whether this status is terminal (no further transitions expected).
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            ProcessingStatus::Completed { .. } | ProcessingStatus::Failed { .. }
        )
    }
}
