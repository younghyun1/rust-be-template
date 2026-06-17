//! Background pipeline that drives a batch-upload session to completion.
//!
//! Files are staged to disk by the batch-upload handler; this module reads each
//! one back under a bounded semaphore permit, encodes the main image + thumbnail
//! (reusing [`process_uploaded_image`]), uploads both to S3 with the same
//! orphan-cleanup contract as the single-file `upload_photograph` handler,
//! inserts the row, and records per-item status on the shared [`BatchSession`].
//!
//! Bounded concurrency (`num_cpus`) caps blocking-pool pressure, resident image
//! bytes, and simultaneous pool connections. A reconciliation sweep after the
//! join set drains guarantees no item is left non-terminal even if a worker
//! task panics, so every batch eventually reaches `done`.

use std::path::PathBuf;
use std::sync::Arc;

use aws_sdk_s3::primitives::ByteStream;
use chrono::Utc;
use diesel_async::RunQueryDsl;
use tokio::io::AsyncWriteExt;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::domain::photography::batch::session::BatchSession;
use crate::domain::photography::batch::status::ProcessingStatus;
use crate::domain::photography::photographs::{
    Photograph, PhotographContext, PhotographInsertable,
};
use crate::init::state::ServerState;
use crate::schema::photographs;
use crate::util::image::exif_utils::extract_exif_shot_at;
use crate::util::image::map_image_format_to_db_enum::map_image_format_to_str;
use crate::util::image::process_uploaded_images::{
    CyhdevImageType, IMAGE_ENCODING_FORMAT, process_uploaded_image,
};

use crate::util::s3::AWS_S3_BUCKET_NAME;

/// Root directory under the system temp dir for all batch staging.
pub fn batch_root_dir() -> PathBuf {
    std::env::temp_dir().join("cyhdev-batch")
}

/// Per-batch staging directory.
pub fn batch_temp_dir(batch_id: Uuid) -> PathBuf {
    batch_root_dir().join(batch_id.to_string())
}

/// On-disk path for a single staged source file.
pub fn batch_item_path(batch_id: Uuid, item_id: Uuid) -> PathBuf {
    batch_temp_dir(batch_id).join(format!("{item_id}.orig"))
}

/// Stream a multipart chunk source to the staging path for `item_id`.
///
/// Returns the number of bytes written. The caller is responsible for size caps
/// (it owns the running byte budget across the whole request).
pub async fn open_staging_file(batch_id: Uuid, item_id: Uuid) -> std::io::Result<tokio::fs::File> {
    let dir = batch_temp_dir(batch_id);
    tokio::fs::create_dir_all(&dir).await?;
    tokio::fs::File::create(batch_item_path(batch_id, item_id)).await
}

/// One staged file plus its resolved per-file metadata.
pub struct BatchPipelineItem {
    pub item_id: Uuid,
    pub file_name: Option<String>,
    pub content_type: Option<String>,
    pub comments: String,
    pub lat: f64,
    pub lon: f64,
}

/// Spawn the background supervisor for a registered batch session.
///
/// Fire-and-forget: status is observed via the in-memory [`BatchSession`].
pub fn spawn_batch(
    state: Arc<ServerState>,
    batch: Arc<BatchSession>,
    items: Vec<BatchPipelineItem>,
    user_id: Uuid,
    context: PhotographContext,
) {
    tokio::spawn(async move {
        let batch_id = batch.batch_id;
        let s3_client = aws_sdk_s3::Client::new(&state.aws_profile_picture_config);
        let region = state
            .aws_profile_picture_config
            .region()
            .map(|r| r.to_string())
            .unwrap_or_else(|| "us-west-1".to_string());

        let permits = num_cpus::get().max(1);
        let semaphore = Arc::new(Semaphore::new(permits));
        let mut join_set: JoinSet<()> = JoinSet::new();

        for item in items {
            let permit = match Arc::clone(&semaphore).acquire_owned().await {
                Ok(permit) => permit,
                Err(e) => {
                    error!(
                        batch_id = %batch_id,
                        item_id = %item.item_id,
                        error = %e,
                        "Failed to acquire batch semaphore permit; failing item"
                    );
                    batch
                        .fail_item(
                            item.item_id,
                            "internal scheduling error".to_string(),
                            Utc::now(),
                        )
                        .await;
                    continue;
                }
            };

            let state = Arc::clone(&state);
            let batch = Arc::clone(&batch);
            let s3_client = s3_client.clone();
            let region = region.clone();

            join_set.spawn(async move {
                process_batch_item(
                    state, s3_client, region, batch, batch_id, item, user_id, context,
                )
                .await;
                drop(permit);
            });
        }

        while let Some(res) = join_set.join_next().await {
            if let Err(e) = res {
                error!(batch_id = %batch_id, error = %e, "Batch item task join error");
            }
        }

        // Reconciliation sweep: any item left non-terminal (e.g. a panicked
        // worker) is failed here so the batch can never get stuck below `done`.
        let now = Utc::now();
        for item in batch.snapshot_items().await {
            if !item.status.is_terminal() {
                warn!(
                    batch_id = %batch_id,
                    item_id = %item.item_id,
                    "Batch item left non-terminal after processing; marking failed"
                );
                batch
                    .fail_item(item.item_id, "processing did not complete".to_string(), now)
                    .await;
            }
        }

        let dir = batch_temp_dir(batch_id);
        if let Err(e) = tokio::fs::remove_dir_all(&dir).await
            && e.kind() != std::io::ErrorKind::NotFound
        {
            warn!(batch_id = %batch_id, error = %e, path = %dir.display(), "Failed to remove batch temp dir");
        }

        info!(
            batch_id = %batch_id,
            completed = batch.completed_count(),
            failed = batch.failed_count(),
            "Batch processing finished"
        );
    });
}

/// Removes a staged temp file when dropped, covering every exit path of item
/// processing (success or any error) so the staging dir never accumulates
/// orphaned source files between the per-item TTL sweeps.
struct StagedFileGuard {
    path: PathBuf,
}

impl Drop for StagedFileGuard {
    fn drop(&mut self) {
        let path = std::mem::take(&mut self.path);
        tokio::spawn(async move {
            if let Err(e) = tokio::fs::remove_file(&path).await
                && e.kind() != std::io::ErrorKind::NotFound
            {
                warn!(path = %path.display(), error = %e, "Failed to remove staged batch file");
            }
        });
    }
}

/// Process a single staged file end-to-end, recording status transitions.
#[allow(clippy::too_many_arguments)]
async fn process_batch_item(
    state: Arc<ServerState>,
    s3_client: aws_sdk_s3::Client,
    region: String,
    batch: Arc<BatchSession>,
    batch_id: Uuid,
    item: BatchPipelineItem,
    user_id: Uuid,
    context: PhotographContext,
) {
    let item_id = item.item_id;
    let path = batch_item_path(batch_id, item_id);
    // Drop guard removes the staged file on every return path (success or error).
    let _staged_guard = StagedFileGuard { path: path.clone() };

    batch
        .set_status(item_id, ProcessingStatus::Encoding, Utc::now())
        .await;

    // Read the staged source back into memory only now that we hold a permit,
    // bounding resident bytes to roughly `permits * file_size`.
    let bits = match tokio::fs::read(&path).await {
        Ok(bits) => bits,
        Err(e) => {
            error!(batch_id = %batch_id, item_id = %item_id, error = %e, "Failed to read staged batch file");
            batch
                .fail_item(
                    item_id,
                    format!("failed to read staged file: {e}"),
                    Utc::now(),
                )
                .await;
            return;
        }
    };

    // EXIF parse on the blocking pool; non-fatal (mirrors upload_photograph).
    let exif_bytes = bits.clone();
    let photograph_shot_at = match tokio::task::spawn_blocking(move || {
        extract_exif_shot_at(&exif_bytes)
    })
    .await
    {
        Ok(Ok(dt_opt)) => dt_opt,
        Ok(Err(e)) => {
            warn!(batch_id = %batch_id, item_id = %item_id, error = ?e, "Failed to parse EXIF; continuing");
            None
        }
        Err(e) => {
            warn!(batch_id = %batch_id, item_id = %item_id, error = ?e, "EXIF blocking task panicked; continuing");
            None
        }
    };

    let bits_clone = bits.clone();
    let (main_res, thumb_res) = tokio::join!(
        process_uploaded_image(bits, None, CyhdevImageType::Photograph),
        process_uploaded_image(bits_clone, None, CyhdevImageType::Thumbnail),
    );

    let processed_image = match main_res {
        Ok(bytes) => bytes,
        Err(e) => {
            error!(batch_id = %batch_id, item_id = %item_id, error = ?e, "Failed to encode photograph");
            batch
                .fail_item(item_id, format!("failed to encode image: {e}"), Utc::now())
                .await;
            return;
        }
    };
    let processed_thumbnail = match thumb_res {
        Ok(bytes) => bytes,
        Err(e) => {
            error!(batch_id = %batch_id, item_id = %item_id, error = ?e, "Failed to encode thumbnail");
            batch
                .fail_item(
                    item_id,
                    format!("failed to encode thumbnail: {e}"),
                    Utc::now(),
                )
                .await;
            return;
        }
    };

    let (extension, image_type_db_id) = map_image_format_to_str(IMAGE_ENCODING_FORMAT);
    let image_path = format!("images/{item_id}.{extension}");
    let thumbnail_path = format!("thumbnails/{item_id}.{extension}");
    let content_type = item
        .content_type
        .clone()
        .unwrap_or_else(|| "application/octet-stream".to_string());

    batch
        .set_status(item_id, ProcessingStatus::Uploading, Utc::now())
        .await;

    // Upload main image.
    if let Err(e) = s3_client
        .put_object()
        .bucket(AWS_S3_BUCKET_NAME)
        .key(&image_path)
        .content_type(&content_type)
        .body(ByteStream::from(processed_image))
        .send()
        .await
    {
        error!(batch_id = %batch_id, item_id = %item_id, key = %image_path, error = ?e, "Failed to upload main image to S3");
        batch
            .fail_item(item_id, format!("failed to upload image: {e}"), Utc::now())
            .await;
        return;
    }

    // Upload thumbnail; on failure delete the orphaned main object.
    if let Err(e) = s3_client
        .put_object()
        .bucket(AWS_S3_BUCKET_NAME)
        .key(&thumbnail_path)
        .content_type(&content_type)
        .body(ByteStream::from(processed_thumbnail))
        .send()
        .await
    {
        error!(batch_id = %batch_id, item_id = %item_id, key = %thumbnail_path, error = ?e, "Failed to upload thumbnail to S3");
        if let Err(cleanup_err) = s3_client
            .delete_object()
            .bucket(AWS_S3_BUCKET_NAME)
            .key(&image_path)
            .send()
            .await
        {
            error!(batch_id = %batch_id, item_id = %item_id, key = %image_path, error = ?cleanup_err, "Failed to clean up orphaned main image after thumbnail failure");
        }
        batch
            .fail_item(
                item_id,
                format!("failed to upload thumbnail: {e}"),
                Utc::now(),
            )
            .await;
        return;
    }

    let object_url = format!(
        "https://{}.s3.{}.amazonaws.com/{}",
        AWS_S3_BUCKET_NAME, region, image_path
    );
    let thumbnail_url = format!(
        "https://{}.s3.{}.amazonaws.com/{}",
        AWS_S3_BUCKET_NAME, region, thumbnail_path
    );

    batch
        .set_status(item_id, ProcessingStatus::Persisting, Utc::now())
        .await;

    let mut conn = match state.get_conn().await {
        Ok(conn) => conn,
        Err(e) => {
            error!(batch_id = %batch_id, item_id = %item_id, error = ?e, "Failed to get DB connection for batch item");
            delete_both_objects(&s3_client, &image_path, &thumbnail_path).await;
            batch
                .fail_item(
                    item_id,
                    format!("database connection error: {e}"),
                    Utc::now(),
                )
                .await;
            return;
        }
    };

    let insert_res: Result<Photograph, diesel::result::Error> =
        diesel::insert_into(photographs::table)
            .values(PhotographInsertable {
                user_id,
                photograph_shot_at,
                photograph_image_type: image_type_db_id,
                photograph_context: context,
                photograph_is_on_cloud: true,
                photograph_link: object_url.clone(),
                photograph_comments: item.comments.clone(),
                photograph_lat: item.lat,
                photograph_lon: item.lon,
                photograph_thumbnail_link: thumbnail_url.clone(),
            })
            .get_result(&mut conn)
            .await;
    drop(conn);

    let photograph = match insert_res {
        Ok(photograph) => photograph,
        Err(e) => {
            error!(batch_id = %batch_id, item_id = %item_id, error = ?e, "Failed to insert photograph row");
            delete_both_objects(&s3_client, &image_path, &thumbnail_path).await;
            batch
                .fail_item(
                    item_id,
                    format!("failed to persist photograph: {e}"),
                    Utc::now(),
                )
                .await;
            return;
        }
    };

    batch
        .complete_item(
            item_id,
            photograph.photograph_id,
            object_url,
            thumbnail_url,
            Utc::now(),
        )
        .await;
}

/// Delete both S3 objects, logging individual failures. Used when a DB insert
/// fails after both uploads succeeded (orphan cleanup).
async fn delete_both_objects(client: &aws_sdk_s3::Client, image_path: &str, thumbnail_path: &str) {
    for key in [image_path, thumbnail_path] {
        if let Err(e) = client
            .delete_object()
            .bucket(AWS_S3_BUCKET_NAME)
            .key(key)
            .send()
            .await
        {
            error!(bucket = AWS_S3_BUCKET_NAME, key = %key, error = ?e, "Failed to delete orphaned S3 object");
        }
    }
}

/// Convenience used by the handler while streaming: append a chunk to a file.
pub async fn append_chunk(file: &mut tokio::fs::File, chunk: &[u8]) -> std::io::Result<()> {
    file.write_all(chunk).await
}
