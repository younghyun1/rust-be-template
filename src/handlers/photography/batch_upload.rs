//! `POST /api/photographs/batch-upload` — accept many photographs at once.
//!
//! Each file part is streamed to a temp file (bounding memory; closes the
//! `upload_photograph.rs` line-52 TODO), aligned by index to a single `meta`
//! JSON sidecar field (`[{comment, lat, lon}, ...]`). The handler mints a batch
//! id, registers an in-memory session, spawns the background pipeline, and
//! replies **202** immediately with the batch id and per-item ids. Status is
//! polled separately via `GET /api/photographs/batch/{batch_id}`.

use std::sync::Arc;

use axum::{
    Extension,
    extract::{Multipart, State},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::Utc;
use serde_derive::Deserialize;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::{
    domain::photography::{
        batch::session::{BatchItem, BatchSession},
        batch::status::ProcessingStatus,
        photographs::PhotographContext,
    },
    dto::responses::{
        photography::batch_status_response::{BatchUploadItem, BatchUploadResponse},
        response_data::http_resp,
    },
    errors::code_error::{CodeError, CodeErrorResp, HandlerResponse, code_err},
    init::state::ServerState,
    util::{
        image::batch_pipeline::{
            BatchPipelineItem, append_chunk, batch_temp_dir, open_staging_file, spawn_batch,
        },
        time::now::tokio_now,
    },
};

/// Per-file size cap (matches the single-upload limit). The whole-request cap is
/// applied by the route-scoped `DefaultBodyLimit` in `main_router.rs`.
const MAX_FILE_SIZE_BYTES: u64 = 1024 * 1024 * 150; // 150MB
/// Upper bound on files per batch.
const MAX_FILES_PER_BATCH: usize = 50;

const ALLOWED_MIME_TYPES: [&str; 16] = [
    "image/png",
    "image/jpeg",
    "image/gif",
    "image/webp",
    "image/x-portable-anymap",
    "image/tiff",
    "image/x-tga",
    "image/vnd-ms.dds",
    "image/bmp",
    "image/vnd.microsoft.icon",
    "image/vnd.radiance",
    "image/x-exr",
    "image/farbfeld",
    "image/avif",
    "image/qoi",
    "image/vnd.zbrush.pcx",
];

/// Per-file metadata supplied in the `meta` JSON sidecar, aligned to file order.
#[derive(Debug, Deserialize)]
struct BatchMetaEntry {
    comment: Option<String>,
    lat: Option<f64>,
    lon: Option<f64>,
}

/// A file successfully staged to disk during multipart parsing.
struct StagedFile {
    item_id: Uuid,
    file_name: Option<String>,
    content_type: Option<String>,
    size_bytes: u64,
}

#[utoipa::path(
    post,
    path = "/api/photographs/batch-upload",
    tag = "photography",
    request_body(content_type = "multipart/form-data"),
    responses(
        (status = 202, description = "Batch accepted; processing started", body = BatchUploadResponse),
        (status = 400, description = "Invalid batch payload", body = CodeErrorResp),
        (status = 401, description = "Unauthorized", body = CodeErrorResp),
        (status = 403, description = "Forbidden (not superuser)", body = CodeErrorResp),
        (status = 500, description = "Internal server error", body = CodeErrorResp)
    )
)]
pub async fn batch_upload(
    Extension(user_id): Extension<Uuid>,
    State(state): State<Arc<ServerState>>,
    mut multipart: Multipart,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();
    let batch_id = Uuid::now_v7();
    let dir = batch_temp_dir(batch_id);

    let mut staged: Vec<StagedFile> = Vec::new();
    let mut meta: Option<Vec<BatchMetaEntry>> = None;
    let mut context = PhotographContext::Photography;

    while let Some(mut field) = multipart.next_field().await.map_err(|e| {
        error!(error = ?e, user_id = %user_id, "Failed to fetch next multipart field");
        code_err(CodeError::FILE_UPLOAD_ERROR, e)
    })? {
        match field.name().map(str::to_owned).as_deref() {
            Some("files") | Some("file") => {
                if staged.len() >= MAX_FILES_PER_BATCH {
                    let _ = tokio::fs::remove_dir_all(&dir).await;
                    warn!(user_id = %user_id, max = MAX_FILES_PER_BATCH, "Batch exceeds maximum file count");
                    return Err(code_err(
                        CodeError::BATCH_TOO_MANY_FILES,
                        format!("maximum {MAX_FILES_PER_BATCH} files per batch"),
                    ));
                }

                let file_name = field.file_name().map(|n| n.to_string());
                let content_type = field.content_type().map(|c| c.to_string());

                if !content_type
                    .as_deref()
                    .map(|c| ALLOWED_MIME_TYPES.contains(&c))
                    .unwrap_or(false)
                {
                    let _ = tokio::fs::remove_dir_all(&dir).await;
                    warn!(user_id = %user_id, content_type = ?content_type, "Unsupported image type in batch");
                    return Err(code_err(
                        CodeError::FILE_UPLOAD_ERROR,
                        "Unsupported image type in batch",
                    ));
                }

                let item_id = Uuid::now_v7();
                let mut file = open_staging_file(batch_id, item_id).await.map_err(|e| {
                    error!(error = ?e, user_id = %user_id, "Failed to create staging file");
                    code_err(CodeError::FILE_UPLOAD_ERROR, e)
                })?;

                let mut size_bytes: u64 = 0;
                loop {
                    let chunk = match field.chunk().await {
                        Ok(Some(chunk)) => chunk,
                        Ok(None) => break,
                        Err(e) => {
                            let _ = tokio::fs::remove_dir_all(&dir).await;
                            error!(error = ?e, user_id = %user_id, "Failed reading batch file chunk");
                            return Err(code_err(CodeError::FILE_UPLOAD_ERROR, e));
                        }
                    };
                    size_bytes += chunk.len() as u64;
                    if size_bytes > MAX_FILE_SIZE_BYTES {
                        let _ = tokio::fs::remove_dir_all(&dir).await;
                        warn!(user_id = %user_id, limit = MAX_FILE_SIZE_BYTES, "Batch file exceeds maximum size");
                        return Err(code_err(
                            CodeError::FILE_UPLOAD_ERROR,
                            "A file exceeds the maximum allowed size",
                        ));
                    }
                    if let Err(e) = append_chunk(&mut file, &chunk).await {
                        let _ = tokio::fs::remove_dir_all(&dir).await;
                        error!(error = ?e, user_id = %user_id, "Failed writing batch file chunk");
                        return Err(code_err(CodeError::FILE_UPLOAD_ERROR, e));
                    }
                }
                drop(file);

                staged.push(StagedFile {
                    item_id,
                    file_name,
                    content_type,
                    size_bytes,
                });
            }

            Some("meta") => {
                let text = field.text().await.map_err(|e| {
                    error!(error = ?e, user_id = %user_id, "Failed reading meta field");
                    code_err(CodeError::FILE_UPLOAD_ERROR, e)
                })?;
                match serde_json::from_str::<Vec<BatchMetaEntry>>(&text) {
                    Ok(parsed) => meta = Some(parsed),
                    Err(e) => {
                        let _ = tokio::fs::remove_dir_all(&dir).await;
                        warn!(error = ?e, user_id = %user_id, "Invalid meta JSON in batch");
                        return Err(code_err(CodeError::INVALID_REQUEST, "Invalid meta JSON"));
                    }
                }
            }

            Some("context") | Some("photograph_context") => {
                let text = field.text().await.map_err(|e| {
                    error!(error = ?e, user_id = %user_id, "Failed reading context field");
                    code_err(CodeError::FILE_UPLOAD_ERROR, e)
                })?;
                match PhotographContext::from_str(&text) {
                    Some(ctx) => context = ctx,
                    None => {
                        let _ = tokio::fs::remove_dir_all(&dir).await;
                        warn!(user_id = %user_id, value = %text, "Invalid photograph context");
                        return Err(code_err(
                            CodeError::INVALID_REQUEST,
                            "Invalid photograph context",
                        ));
                    }
                }
            }

            Some(other) => {
                warn!(user_id = %user_id, field = other, "Unexpected batch multipart field");
            }
            None => {
                warn!(user_id = %user_id, "Unnamed batch multipart field; ignoring");
            }
        }
    }

    if staged.is_empty() {
        let _ = tokio::fs::remove_dir_all(&dir).await;
        warn!(user_id = %user_id, "Batch contained no files");
        return Err(code_err(CodeError::BATCH_EMPTY, "No files in batch"));
    }

    let meta = match meta {
        Some(meta) => meta,
        None => {
            let _ = tokio::fs::remove_dir_all(&dir).await;
            warn!(user_id = %user_id, "Batch missing meta field");
            return Err(code_err(CodeError::INVALID_REQUEST, "Missing meta field"));
        }
    };

    if meta.len() != staged.len() {
        let _ = tokio::fs::remove_dir_all(&dir).await;
        warn!(
            user_id = %user_id,
            meta_len = meta.len(),
            files_len = staged.len(),
            "Batch meta count does not match file count"
        );
        return Err(code_err(
            CodeError::INVALID_REQUEST,
            "meta count must match file count",
        ));
    }

    // Resolve per-file metadata per context, mirroring the single-upload rules.
    let now = Utc::now();
    let total = staged.len();
    let mut pipeline_items: Vec<BatchPipelineItem> = Vec::with_capacity(total);
    let mut session_items: Vec<BatchItem> = Vec::with_capacity(total);
    let mut response_items: Vec<BatchUploadItem> = Vec::with_capacity(total);

    for (file, entry) in staged.into_iter().zip(meta) {
        let (comments, lat, lon) = match context {
            PhotographContext::Photography => {
                let comments = match entry.comment {
                    Some(c) if !c.trim().is_empty() => c,
                    _ => {
                        let _ = tokio::fs::remove_dir_all(&dir).await;
                        warn!(user_id = %user_id, "Batch item missing comment");
                        return Err(code_err(
                            CodeError::INVALID_REQUEST,
                            "Each photo requires a comment",
                        ));
                    }
                };
                let lat = match entry.lat {
                    Some(v) => v,
                    None => {
                        let _ = tokio::fs::remove_dir_all(&dir).await;
                        warn!(user_id = %user_id, "Batch item missing latitude");
                        return Err(code_err(
                            CodeError::INVALID_REQUEST,
                            "Each photo requires a location",
                        ));
                    }
                };
                let lon = match entry.lon {
                    Some(v) => v,
                    None => {
                        let _ = tokio::fs::remove_dir_all(&dir).await;
                        warn!(user_id = %user_id, "Batch item missing longitude");
                        return Err(code_err(
                            CodeError::INVALID_REQUEST,
                            "Each photo requires a location",
                        ));
                    }
                };
                (comments, lat, lon)
            }
            PhotographContext::Post => {
                let fallback = file
                    .file_name
                    .clone()
                    .unwrap_or_else(|| "post image".to_string());
                let comments = match entry.comment {
                    Some(c) if !c.trim().is_empty() => c,
                    _ => fallback,
                };
                (comments, entry.lat.unwrap_or(0.0), entry.lon.unwrap_or(0.0))
            }
        };

        response_items.push(BatchUploadItem {
            item_id: file.item_id,
            file_name: file.file_name.clone(),
        });
        session_items.push(BatchItem {
            item_id: file.item_id,
            original_file_name: file.file_name.clone(),
            original_size_bytes: file.size_bytes,
            status: ProcessingStatus::Queued,
            created_at: now,
            updated_at: now,
        });
        pipeline_items.push(BatchPipelineItem {
            item_id: file.item_id,
            file_name: file.file_name,
            content_type: file.content_type,
            comments,
            lat,
            lon,
        });
    }

    let batch = Arc::new(BatchSession::new(batch_id, user_id, total, now));
    for item in session_items {
        batch.register_item(item).await;
    }
    state.register_batch(Arc::clone(&batch)).await;

    spawn_batch(
        Arc::clone(&state),
        Arc::clone(&batch),
        pipeline_items,
        user_id,
        context,
    );

    info!(user_id = %user_id, batch_id = %batch_id, total, "Accepted batch upload; processing started");

    let resp = BatchUploadResponse {
        batch_id,
        total,
        items: response_items,
    };
    Ok((StatusCode::ACCEPTED, http_resp(resp, (), start)))
}
