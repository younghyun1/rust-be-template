//! Response DTOs for the batch-upload + processing-tracker endpoints.
//!
//! Decoupled from the in-memory domain types so the wire shape is stable.

use chrono::{DateTime, Utc};
use serde_derive::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::photography::batch::status::ProcessingStatus;

/// One item as echoed back from the initial batch-upload acceptance.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BatchUploadItem {
    pub item_id: Uuid,
    pub file_name: Option<String>,
}

/// Immediate (202) response to `POST /api/photographs/batch-upload`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BatchUploadResponse {
    pub batch_id: Uuid,
    pub total: usize,
    pub items: Vec<BatchUploadItem>,
}

/// One item's live status.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BatchItemStatus {
    pub item_id: Uuid,
    pub file_name: Option<String>,
    pub original_size_bytes: u64,
    pub status: ProcessingStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Full status snapshot for a single batch.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BatchStatusResponse {
    pub batch_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub total: usize,
    pub completed: usize,
    pub failed: usize,
    pub pending: usize,
    pub done: bool,
    pub items: Vec<BatchItemStatus>,
}

/// All of the caller's tracked batches.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BatchListResponse {
    pub batches: Vec<BatchStatusResponse>,
}
