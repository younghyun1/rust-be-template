use std::sync::Arc;

use axum::{
    Extension,
    extract::{Multipart, State},
    response::IntoResponse,
};
use chrono::Utc;
use diesel_async::RunQueryDsl;
use tracing::{error, info};
use uuid::Uuid;

use crate::{
    domain::wasm_module::wasm_module::{WasmModule, WasmModuleInsertable},
    dto::responses::{response_data::http_resp, wasm_module::WasmModuleItem},
    errors::code_error::{CodeError, CodeErrorResp, HandlerResponse, code_err},
    init::state::ServerState,
    schema::wasm_module,
    util::{
        auth::is_superuser::is_superuser,
        image::{
            map_image_format_to_db_enum::map_image_format_to_str,
            process_uploaded_images::{
                CyhdevImageType, IMAGE_ENCODING_FORMAT, process_uploaded_image,
            },
        },
        time::now::tokio_now,
        wasm_bundle::{looks_like_html, normalize_bundle_bytes},
    },
};

const MAX_BUNDLE_SIZE: usize = 1024 * 1024 * 50; // 50MB
const MAX_THUMBNAIL_SIZE: usize = 1024 * 1024 * 10; // 10MB
const AWS_S3_BUCKET_NAME: &str = "cyhdev-img";

/// POST /api/wasm-modules
/// Superuser only - uploads a new WASM module bundle with thumbnail
///
/// Multipart fields:
/// - `bundle_file` or `wasm_file`: The HTML bundle (.html or .html.gz) or WASM file (required)
/// - `thumbnail`: The thumbnail image (required)
/// - `title`: Module title (required)
/// - `description`: Module description (required)
#[utoipa::path(
    post,
    path = "/api/wasm-modules",
    tag = "wasm_module",
    request_body(content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "WASM module uploaded successfully", body = WasmModuleItem),
        (status = 400, description = "Invalid upload payload", body = CodeErrorResp),
        (status = 401, description = "Unauthorized", body = CodeErrorResp),
        (status = 403, description = "Forbidden (not superuser)", body = CodeErrorResp),
        (status = 500, description = "Internal server error", body = CodeErrorResp)
    )
)]
pub async fn upload_wasm_module(
    Extension(user_id): Extension<Uuid>,
    State(state): State<Arc<ServerState>>,
    mut multipart: Multipart,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    // Check superuser status
    let is_su = is_superuser(state.clone(), user_id).await.map_err(|e| {
        error!(error = ?e, user_id = %user_id, "Failed to check superuser status");
        code_err(CodeError::DB_QUERY_ERROR, e)
    })?;

    if !is_su {
        error!(user_id = %user_id, "User is not superuser; cannot upload WASM module");
        return Err(code_err(
            CodeError::IS_NOT_SUPERUSER,
            "Only superusers can upload WASM modules",
        ));
    }

    let mut bundle_bytes: Option<Vec<u8>> = None;
    let mut bundle_is_gzipped = false;
    let mut bundle_is_html = false;
    let mut thumbnail_bytes: Option<Vec<u8>> = None;
    let mut title: Option<String> = None;
    let mut description: Option<String> = None;

    // Process multipart fields
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        error!(error = ?e, "Failed to read multipart field");
        code_err(CodeError::FILE_UPLOAD_ERROR, e)
    })? {
        let name = field.name().map(str::to_owned);

        match name.as_deref() {
            Some("bundle_file") | Some("wasm_file") | Some("wasm") => {
                let file_name = field.file_name().map(|s| s.to_string());
                let content_type = field.content_type().map(|s| s.to_string());
                let bytes = field.bytes().await.map_err(|e| {
                    error!(error = ?e, "Failed to read bundle file bytes");
                    code_err(CodeError::FILE_UPLOAD_ERROR, e)
                })?;

                if bytes.len() > MAX_BUNDLE_SIZE {
                    return Err(code_err(
                        CodeError::FILE_UPLOAD_ERROR,
                        format!(
                            "Bundle file too large (max {}MB)",
                            MAX_BUNDLE_SIZE / 1024 / 1024
                        ),
                    ));
                }

                let gzip_magic = bytes.len() >= 2 && bytes[0] == 0x1f && bytes[1] == 0x8b;
                bundle_is_gzipped = gzip_magic
                    || file_name
                        .as_deref()
                        .map(|name| name.ends_with(".gz"))
                        .unwrap_or(false)
                    || content_type
                        .as_deref()
                        .map(|ct| ct.contains("gzip"))
                        .unwrap_or(false);

                let file_is_html = content_type
                    .as_deref()
                    .map(|ct| ct.starts_with("text/html"))
                    .unwrap_or(false)
                    || file_name
                        .as_deref()
                        .map(|name| {
                            name.ends_with(".html")
                                || name.ends_with(".htm")
                                || name.ends_with(".html.gz")
                                || name.ends_with(".htm.gz")
                        })
                        .unwrap_or(false);

                let file_is_wasm = content_type
                    .as_deref()
                    .map(|ct| ct.starts_with("application/wasm"))
                    .unwrap_or(false)
                    || file_name
                        .as_deref()
                        .map(|name| name.ends_with(".wasm") || name.ends_with(".wasm.gz"))
                        .unwrap_or(false);

                if file_is_html {
                    bundle_is_html = true;
                } else if file_is_wasm
                    || (!bundle_is_gzipped && bytes.len() >= 4 && &bytes[0..4] == b"\x00asm")
                {
                    bundle_is_html = false;
                } else if !bundle_is_gzipped && looks_like_html(&bytes) {
                    bundle_is_html = true;
                } else if bundle_is_gzipped {
                    return Err(code_err(
                        CodeError::FILE_UPLOAD_ERROR,
                        "Unable to determine gzipped bundle type; please name it .html.gz or .wasm.gz",
                    ));
                } else {
                    return Err(code_err(
                        CodeError::FILE_UPLOAD_ERROR,
                        "Unrecognized bundle type; expected .html/.html.gz or .wasm",
                    ));
                }

                if !bundle_is_html && !bundle_is_gzipped {
                    // Basic WASM magic number check (0x00 0x61 0x73 0x6D = \0asm)
                    if bytes.len() < 4 || &bytes[0..4] != b"\x00asm" {
                        return Err(code_err(
                            CodeError::FILE_UPLOAD_ERROR,
                            "Invalid WASM file (missing magic number)",
                        ));
                    }
                }

                bundle_bytes = Some(bytes.to_vec());
            }

            Some("thumbnail") | Some("thumbnail_file") => {
                let bytes = field.bytes().await.map_err(|e| {
                    error!(error = ?e, "Failed to read thumbnail bytes");
                    code_err(CodeError::FILE_UPLOAD_ERROR, e)
                })?;

                if bytes.len() > MAX_THUMBNAIL_SIZE {
                    return Err(code_err(
                        CodeError::FILE_UPLOAD_ERROR,
                        format!(
                            "Thumbnail too large (max {}MB)",
                            MAX_THUMBNAIL_SIZE / 1024 / 1024
                        ),
                    ));
                }

                thumbnail_bytes = Some(bytes.to_vec());
            }

            Some("title") | Some("wasm_module_title") => {
                let text = field.text().await.map_err(|e| {
                    error!(error = ?e, "Failed to read title field");
                    code_err(CodeError::FILE_UPLOAD_ERROR, e)
                })?;
                title = Some(text);
            }

            Some("description") | Some("wasm_module_description") => {
                let text = field.text().await.map_err(|e| {
                    error!(error = ?e, "Failed to read description field");
                    code_err(CodeError::FILE_UPLOAD_ERROR, e)
                })?;
                description = Some(text);
            }

            Some(other) => {
                info!(field = other, "Ignoring unknown multipart field");
            }

            None => {}
        }
    }

    // Validate required fields
    let bundle_bytes = bundle_bytes
        .ok_or_else(|| code_err(CodeError::FILE_UPLOAD_ERROR, "Missing bundle file"))?;

    let thumbnail_bytes = thumbnail_bytes
        .ok_or_else(|| code_err(CodeError::FILE_UPLOAD_ERROR, "Missing thumbnail image"))?;

    let title =
        title.ok_or_else(|| code_err(CodeError::FILE_UPLOAD_ERROR, "Missing title field"))?;

    let description = description
        .ok_or_else(|| code_err(CodeError::FILE_UPLOAD_ERROR, "Missing description field"))?;

    // Generate UUID for the module
    let wasm_module_id = Uuid::new_v4();

    let normalized_bundle = tokio::task::spawn_blocking(move || {
        normalize_bundle_bytes(
            &bundle_bytes,
            bundle_is_gzipped,
            bundle_is_html,
            MAX_BUNDLE_SIZE,
        )
    })
    .await
    .map_err(|e| {
        error!(error = ?e, "Failed to compress WASM bundle");
        code_err(CodeError::FILE_UPLOAD_ERROR, e)
    })?
    .map_err(|e| {
        error!(error = ?e, "Failed to normalize WASM bundle bytes");
        code_err(CodeError::FILE_UPLOAD_ERROR, e)
    })?;

    info!(
        wasm_module_id = %wasm_module_id,
        size_bytes = normalized_bundle.gz_bytes.len(),
        is_html = bundle_is_html,
        is_gzipped = true,
        "Prepared WASM bundle for database storage"
    );

    // Upload thumbnail to S3
    let processed_thumbnail =
        process_uploaded_image(thumbnail_bytes, None, CyhdevImageType::DemoThumbnail)
            .await
            .map_err(|e| {
                error!(error = ?e, "Failed to process WASM thumbnail image");
                code_err(CodeError::COULD_NOT_PROCESS_IMAGE, e)
            })?;

    let (thumb_ext, _) = map_image_format_to_str(IMAGE_ENCODING_FORMAT);
    let thumbnail_path = format!("wasm-thumbnails/{}.{}", wasm_module_id, thumb_ext);
    let s3_client = aws_sdk_s3::Client::new(&state.aws_profile_picture_config);

    s3_client
        .put_object()
        .bucket(AWS_S3_BUCKET_NAME)
        .key(&thumbnail_path)
        .content_type("image/avif")
        .body(aws_sdk_s3::primitives::ByteStream::from(
            processed_thumbnail,
        ))
        .send()
        .await
        .map_err(|e| {
            error!(error = ?e, "Failed to upload thumbnail to S3");
            code_err(CodeError::FILE_UPLOAD_ERROR, e)
        })?;

    let s3_region = state
        .aws_profile_picture_config
        .region()
        .map(|r| r.to_string())
        .unwrap_or_else(|| "us-west-1".to_string());

    let thumbnail_url = format!(
        "https://{}.s3.{}.amazonaws.com/{}",
        AWS_S3_BUCKET_NAME, s3_region, thumbnail_path
    );

    // The WASM link will be served by our backend route
    let wasm_link = format!("/api/wasm-modules/{}/wasm", wasm_module_id);

    let now = Utc::now();

    // Insert into database
    let mut conn = state.get_conn().await.map_err(|e| {
        error!(error = ?e, "Failed to get DB connection");
        code_err(CodeError::POOL_ERROR, e)
    })?;

    let module: WasmModule = diesel::insert_into(wasm_module::table)
        .values(WasmModuleInsertable {
            wasm_module_id,
            user_id,
            wasm_module_link: wasm_link,
            wasm_module_description: description,
            wasm_module_created_at: now,
            wasm_module_updated_at: now,
            wasm_module_thumbnail_link: thumbnail_url,
            wasm_module_title: title,
            wasm_module_bundle_gz: normalized_bundle.gz_bytes.clone(),
        })
        .get_result(&mut conn)
        .await
        .map_err(|e| {
            error!(error = ?e, "Failed to insert WASM module into DB");
            code_err(CodeError::DB_INSERTION_ERROR, e)
        })?;

    drop(conn);

    state
        .upsert_wasm_module_cache(
            wasm_module_id,
            normalized_bundle.gz_bytes,
            normalized_bundle.content_type,
        )
        .await;

    info!(
        wasm_module_id = %wasm_module_id,
        user_id = %user_id,
        title = %module.wasm_module_title,
        "WASM module uploaded successfully"
    );

    Ok(http_resp(WasmModuleItem::from(module), (), start))
}
