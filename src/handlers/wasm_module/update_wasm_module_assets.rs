use std::sync::Arc;

use axum::{
    Extension,
    extract::{Multipart, Path, State},
    response::IntoResponse,
};
use chrono::Utc;
use diesel::{AsChangeset, ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use tracing::{error, info};
use uuid::Uuid;

use crate::{
    domain::wasm_module::wasm_module::WasmModule,
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

#[derive(AsChangeset, Default)]
#[diesel(table_name = wasm_module)]
struct WasmModuleAssetsChangeset {
    wasm_module_title: Option<String>,
    wasm_module_description: Option<String>,
    wasm_module_thumbnail_link: Option<String>,
    wasm_module_bundle_gz: Option<Vec<u8>>,
    wasm_module_updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// POST /api/wasm-modules/{wasm_module_id}/assets
/// Superuser only - updates WASM module bundle/thumbnail and optional metadata.
#[utoipa::path(
    post,
    path = "/api/wasm-modules/{wasm_module_id}/assets",
    tag = "wasm_module",
    params(
        ("wasm_module_id" = Uuid, Path, description = "WASM module UUID")
    ),
    request_body(content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "WASM module updated", body = WasmModuleItem),
        (status = 400, description = "Invalid upload payload", body = CodeErrorResp),
        (status = 401, description = "Unauthorized", body = CodeErrorResp),
        (status = 403, description = "Forbidden (not superuser)", body = CodeErrorResp),
        (status = 404, description = "WASM module not found", body = CodeErrorResp),
        (status = 500, description = "Internal server error", body = CodeErrorResp)
    )
)]
pub async fn update_wasm_module_assets(
    Extension(user_id): Extension<Uuid>,
    State(state): State<Arc<ServerState>>,
    Path(wasm_module_id): Path<Uuid>,
    mut multipart: Multipart,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    // Check superuser status
    let is_su = is_superuser(state.clone(), user_id).await.map_err(|e| {
        error!(error = ?e, user_id = %user_id, "Failed to check superuser status");
        code_err(CodeError::DB_QUERY_ERROR, e)
    })?;

    if !is_su {
        error!(user_id = %user_id, "User is not superuser; cannot update WASM module");
        return Err(code_err(
            CodeError::IS_NOT_SUPERUSER,
            "Only superusers can update WASM modules",
        ));
    }

    let mut bundle_bytes: Option<Vec<u8>> = None;
    let mut bundle_is_gzipped = false;
    let mut bundle_is_html = false;
    let mut thumbnail_bytes: Option<Vec<u8>> = None;
    let mut title: Option<String> = None;
    let mut description: Option<String> = None;

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

                if !bundle_is_html
                    && !bundle_is_gzipped
                    && (bytes.len() < 4 || &bytes[0..4] != b"\x00asm")
                {
                    return Err(code_err(
                        CodeError::FILE_UPLOAD_ERROR,
                        "Invalid WASM file (missing magic number)",
                    ));
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
                if !text.trim().is_empty() {
                    title = Some(text);
                }
            }

            Some("description") | Some("wasm_module_description") => {
                let text = field.text().await.map_err(|e| {
                    error!(error = ?e, "Failed to read description field");
                    code_err(CodeError::FILE_UPLOAD_ERROR, e)
                })?;
                if !text.trim().is_empty() {
                    description = Some(text);
                }
            }

            Some(other) => {
                info!(field = other, "Ignoring unknown multipart field");
            }

            None => {}
        }
    }

    let mut bundle_gz_for_db: Option<Vec<u8>> = None;
    let mut bundle_cache_entry: Option<(Vec<u8>, &'static str)> = None;

    if let Some(bundle_bytes) = bundle_bytes {
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
            "Prepared updated WASM bundle for database storage"
        );

        bundle_gz_for_db = Some(normalized_bundle.gz_bytes.clone());
        bundle_cache_entry = Some((normalized_bundle.gz_bytes, normalized_bundle.content_type));
    }

    let mut thumbnail_url: Option<String> = None;
    if let Some(thumbnail_bytes) = thumbnail_bytes {
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

        thumbnail_url = Some(format!(
            "https://{}.s3.{}.amazonaws.com/{}",
            AWS_S3_BUCKET_NAME, s3_region, thumbnail_path
        ));
    }

    let mut conn = state.get_conn().await.map_err(|e| {
        error!(error = ?e, "Failed to get DB connection");
        code_err(CodeError::POOL_ERROR, e)
    })?;

    let changeset = WasmModuleAssetsChangeset {
        wasm_module_title: title,
        wasm_module_description: description,
        wasm_module_thumbnail_link: thumbnail_url,
        wasm_module_bundle_gz: bundle_gz_for_db,
        wasm_module_updated_at: Some(Utc::now()),
    };

    let updated: WasmModule = diesel::update(
        wasm_module::table.filter(wasm_module::wasm_module_id.eq(wasm_module_id)),
    )
    .set(&changeset)
    .get_result(&mut conn)
    .await
    .map_err(|e| {
        error!(error = ?e, wasm_module_id = %wasm_module_id, "Failed to update WASM module");
        match e {
            diesel::result::Error::NotFound => {
                code_err(CodeError::DB_QUERY_ERROR, "WASM module not found")
            }
            _ => code_err(CodeError::DB_UPDATE_ERROR, e),
        }
    })?;

    drop(conn);

    let (cache_bytes, content_type) = match bundle_cache_entry {
        Some((gz_bytes, content_type)) => (gz_bytes, content_type),
        None => {
            let content_type = crate::util::wasm_bundle::sniff_content_type_from_gzip_bytes(
                &updated.wasm_module_bundle_gz,
            )
            .map_err(|e| {
                error!(
                    error = ?e,
                    wasm_module_id = %wasm_module_id,
                    "Failed to detect bundle content type while refreshing WASM cache"
                );
                code_err(CodeError::DB_UPDATE_ERROR, e)
            })?;
            (updated.wasm_module_bundle_gz.clone(), content_type)
        }
    };
    state
        .upsert_wasm_module_cache(wasm_module_id, cache_bytes, content_type)
        .await;

    Ok(http_resp(WasmModuleItem::from(updated), (), start))
}
