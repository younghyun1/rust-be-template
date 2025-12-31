use std::sync::Arc;

use axum::{
    Extension,
    extract::{Multipart, State},
    response::IntoResponse,
};
use diesel_async::RunQueryDsl;
use tracing::{error, info};
use uuid::Uuid;

use crate::{
    domain::photography::photographs::{Photograph, PhotographInsertable},
    dto::responses::response_data::http_resp,
    errors::code_error::{CodeError, CodeErrorResp, HandlerResponse, code_err},
    init::state::ServerState,
    schema::photographs,
    util::{
        auth::is_superuser::is_superuser,
        image::{
            exif_utils::extract_exif_shot_at,
            map_image_format_to_db_enum::map_image_format_to_str,
            process_uploaded_images::{
                CyhdevImageType, IMAGE_ENCODING_FORMAT, format_size, process_uploaded_image,
            },
        },
        time::now::tokio_now,
    },
};

// TODO: This is not actually checked. Need to check that; also see what's going on with the profile pic size check.
//
const MAX_SIZE_OF_UPLOADABLE_PHOTOGRPAH: usize = 1024 * 1024 * 150; // 150MB
const ALLOWED_MIME_TYPES: [&str; 16] = [
    "image/png",                // PNG
    "image/jpeg",               // JPEG
    "image/gif",                // GIF
    "image/webp",               // WebP
    "image/x-portable-anymap",  // PNM (general format including PBM, PGM, PPM)
    "image/tiff",               // TIFF
    "image/x-tga",              // TGA
    "image/vnd-ms.dds",         // DDS
    "image/bmp",                // BMP
    "image/vnd.microsoft.icon", // ICO
    "image/vnd.radiance",       // HDR
    "image/x-exr",              // OpenEXR
    "image/farbfeld",           // Farbfeld
    "image/avif",               // AVIF
    "image/qoi",                // QOI
    "image/vnd.zbrush.pcx",     // PCX
];

const AWS_S3_BUCKET_NAME: &str = "cyhdev-img";

// TODO: STREAM to file, don't keep the whole damn thing around
#[utoipa::path(
    post,
    path = "/api/photographs/upload",
    request_body(content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "Photograph uploaded successfully", body = Photograph),
        (status = 400, description = "Invalid upload payload", body = CodeErrorResp),
        (status = 401, description = "Unauthorized", body = CodeErrorResp),
        (status = 403, description = "Forbidden (not superuser)", body = CodeErrorResp),
        (status = 500, description = "Internal server error", body = CodeErrorResp)
    )
)]
pub async fn upload_photograph(
    Extension(user_id): Extension<Uuid>,
    State(state): State<Arc<ServerState>>,
    mut multipart: Multipart,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    let mut uploaded_file: Vec<u8> = Vec::with_capacity(MAX_SIZE_OF_UPLOADABLE_PHOTOGRPAH);

    let mut mime: Option<String> = None;

    let mut _extension: String = String::new();

    // Additional metadata fields provided in the multipart body (all required)

    let mut photograph_comments: Option<String> = None;

    let mut photograph_lat: Option<f64> = None;

    let mut photograph_lon: Option<f64> = None;

    let is_superuser = match is_superuser(state.clone(), user_id).await {
        Ok(is_superuser) => is_superuser,
        Err(e) => {
            error!(error = ?e, user_id = %user_id, "Failed to check if user is superuser");
            return Err(code_err(CodeError::DB_QUERY_ERROR, e));
        }
    };

    if !is_superuser {
        error!(user_id = %user_id, "User is not a superuser; cannot upload photograph");
        return Err(code_err(
            CodeError::IS_NOT_SUPERUSER,
            "User is not a superuser; cannot upload photograph",
        ));
    }

    // Process the multipart fields
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        error!(error = ?e, user_id = %user_id, "Failed to fetch next multipart field");
        code_err(CodeError::FILE_UPLOAD_ERROR, e)
    })? {
        let name = field.name().map(str::to_owned);

        match name.as_deref() {
            // Image file field (default / "file")
            Some("file") | None => {
                // For the first file field, extract metadata (file name and MIME type)
                if uploaded_file.is_empty() {
                    _extension = field
                        .file_name()
                        .and_then(|name| name.rsplit('.').next().map(|ext| ext.to_string()))
                        .ok_or_else(|| {
                            error!(user_id = %user_id, "Missing file extension in uploaded filename");
                            code_err(
                                CodeError::FILE_UPLOAD_ERROR,
                                "No extensions, that's illegal!",
                            )
                        })?;
                    mime = Some(
                        field
                            .content_type()
                            .map(|mime| mime.to_string())
                            .ok_or_else(|| {
                                error!(user_id = %user_id, "No MIME content type on uploaded file");
                                code_err(
                                    CodeError::FILE_UPLOAD_ERROR,
                                    "No MIME extensions, that's illegal!",
                                )
                            })?,
                    );
                    if !mime
                        .as_ref()
                        .map(|m| ALLOWED_MIME_TYPES.contains(&m.as_str()))
                        .unwrap_or(false)
                    {
                        error!(
                            user_id = %user_id,
                            mime = ?mime,
                            "Unsupported image type; rejecting upload"
                        );
                        return Err(code_err(
                            CodeError::FILE_UPLOAD_ERROR,
                            "Unsupported image type; no PSDs!",
                        ));
                    }
                }
                // Read and accumulate the file bytes.
                let bytes = field.bytes().await.map_err(|e| {
                    error!(error = ?e, user_id = %user_id, "Failed reading multipart field bytes");
                    code_err(CodeError::FILE_UPLOAD_ERROR, e)
                })?;
                uploaded_file.extend_from_slice(&bytes);
            }

            // Comments field (required)
            Some("comments") => {
                let text = field.text().await.map_err(|e| {
                    error!(error = ?e, user_id = %user_id, "Failed reading comments field");

                    code_err(CodeError::FILE_UPLOAD_ERROR, e)
                })?;

                photograph_comments = Some(text);
            }

            // Latitude field (required)
            Some("lat") => {
                let text = field.text().await.map_err(|e| {
                    error!(error = ?e, user_id = %user_id, "Failed reading lat field");

                    code_err(CodeError::FILE_UPLOAD_ERROR, e)
                })?;

                match text.parse::<f64>() {
                    Ok(v) => photograph_lat = Some(v),

                    Err(_) => {
                        error!(user_id = %user_id, value = %text, "Invalid lat value");
                        return Err(code_err(
                            CodeError::FILE_UPLOAD_ERROR,
                            "Invalid latitude value",
                        ));
                    }
                }
            }

            // Longitude field (required)
            Some("lon") => {
                let text = field.text().await.map_err(|e| {
                    error!(error = ?e, user_id = %user_id, "Failed reading lon field");
                    code_err(CodeError::FILE_UPLOAD_ERROR, e)
                })?;
                match text.parse::<f64>() {
                    Ok(v) => photograph_lon = Some(v),
                    Err(_) => {
                        error!(user_id = %user_id, value = %text, "Invalid lon value");
                        return Err(code_err(
                            CodeError::FILE_UPLOAD_ERROR,
                            "Invalid longitude value",
                        ));
                    }
                }
            }

            // Unknown fields: log and ignore
            Some(other) => {
                error!(user_id = %user_id, field = other, "Unexpected multipart field");
            }
        }
    }

    if uploaded_file.is_empty() {
        error!(user_id = %user_id, "Uploaded file is empty");

        return Err(code_err(CodeError::FILE_UPLOAD_ERROR, "File is empty!"));
    }

    let original_size_bytes = uploaded_file.len() as u64;
    info!(
        user_id = %user_id,
        original_size_bytes,
        "Received uploaded photograph bytes"
    );

    // Ensure required metadata fields are present
    let photograph_comments = match photograph_comments {
        Some(c) if !c.is_empty() => c,
        _ => {
            error!(user_id = %user_id, "Missing required comments field");
            return Err(code_err(
                CodeError::FILE_UPLOAD_ERROR,
                "Missing required comments field",
            ));
        }
    };

    let photograph_lat = match photograph_lat {
        Some(v) => v,
        None => {
            error!(user_id = %user_id, "Missing required lat field");
            return Err(code_err(
                CodeError::FILE_UPLOAD_ERROR,
                "Missing required latitude field",
            ));
        }
    };

    let photograph_lon = match photograph_lon {
        Some(v) => v,
        None => {
            error!(user_id = %user_id, "Missing required lon field");
            return Err(code_err(
                CodeError::FILE_UPLOAD_ERROR,
                "Missing required longitude field",
            ));
        }
    };

    // Try to extract EXIF shot date from the original bytes.
    // This may return None if there is no EXIF or no DateTimeOriginal.
    let photograph_shot_at = match extract_exif_shot_at(&uploaded_file) {
        Ok(dt_opt) => dt_opt,
        Err(e) => {
            error!(
                error = ?e,
                user_id = %user_id,
                "Failed to parse EXIF shot-at datetime from uploaded photograph"
            );
            None
        }
    };

    // compress and process image here in a blocking thread
    let uploaded_file_clone = uploaded_file.clone();

    let process_photograph_future =
        process_uploaded_image(uploaded_file, None, CyhdevImageType::Photograph);

    let process_thumbnail_future =
        process_uploaded_image(uploaded_file_clone, None, CyhdevImageType::Thumbnail);

    let (processed_image_res, processed_thumbnail_res) =
        tokio::join!(process_photograph_future, process_thumbnail_future);

    let processed_image: Vec<u8> = processed_image_res.map_err(|e| {
        error!(error = ?e, user_id = %user_id, "Failed to process uploaded photograph");
        code_err(CodeError::COULD_NOT_PROCESS_IMAGE, e)
    })?;

    let processed_thumbnail: Vec<u8> = processed_thumbnail_res.map_err(|e| {
        error!(error = ?e, user_id = %user_id, "Failed to process uploaded thumbnail");
        code_err(CodeError::COULD_NOT_PROCESS_IMAGE, e)
    })?;

    // Sizes of processed images for logging
    let main_size_bytes: usize = processed_image.len();
    let thumb_size_bytes: usize = processed_thumbnail.len();

    // store in filesystem or S3
    let image_id: Uuid = uuid::Uuid::new_v4();
    let (extension, image_type_db_id) = map_image_format_to_str(IMAGE_ENCODING_FORMAT);

    let image_path = format!("images/{image_id}.{extension}");
    let thumbnail_path = format!("thumbnails/{image_id}.{extension}");

    // upload to S3 here
    // Initialize AWS S3 client from environment and upload the image
    let s3_client = aws_sdk_s3::Client::new(&state.aws_profile_picture_config);

    // Upload main photograph
    s3_client
        .put_object()
        .bucket(AWS_S3_BUCKET_NAME)
        .key(&image_path)
        .content_type(mime.as_deref().unwrap_or("application/octet-stream"))
        .body(aws_sdk_s3::primitives::ByteStream::from(processed_image))
        .send()
        .await
        .map_err(|e| {
            error!(
                error = ?e,
                user_id = %user_id,
                bucket = AWS_S3_BUCKET_NAME,
                key = %image_path,
                "Failed to upload profile picture to S3"
            );
            code_err(CodeError::FILE_UPLOAD_ERROR, e)
        })?;

    info!(
        user_id = %user_id,
        bucket = AWS_S3_BUCKET_NAME,
        key = %image_path,
        main_size_bytes,
        main_size_human = %format_size(main_size_bytes),
        "Uploaded main photograph to S3"
    );

    // Upload thumbnail

    s3_client
        .put_object()
        .bucket(AWS_S3_BUCKET_NAME)
        .key(&thumbnail_path)
        .content_type(mime.as_deref().unwrap_or("application/octet-stream"))
        .body(aws_sdk_s3::primitives::ByteStream::from(
            processed_thumbnail,
        ))
        .send()
        .await
        .map_err(|e| {
            error!(
                error = ?e,
                user_id = %user_id,
                bucket = AWS_S3_BUCKET_NAME,
                key = %thumbnail_path,
                "Failed to upload thumbnail to S3"
            );

            code_err(CodeError::FILE_UPLOAD_ERROR, e)
        })?;

    info!(
        user_id = %user_id,
        bucket = AWS_S3_BUCKET_NAME,
        key = %thumbnail_path,
        thumb_size_bytes,
        thumb_size_human = %format_size(thumb_size_bytes),
        "Uploaded thumbnail photograph to S3"
    );

    // Assemble the public S3 object URL
    // Replace `<region>` below with your actual AWS region as appropriate
    let s3_region: String = state
        .aws_profile_picture_config
        .region()
        .map(|r| r.to_string())
        .unwrap_or_else(|| "us-west-1".to_string());

    let object_url: String = format!(
        "https://{}.s3.{}.amazonaws.com/{}",
        AWS_S3_BUCKET_NAME, s3_region, image_path
    );

    let thumbnail_url: String = format!(
        "https://{}.s3.{}.amazonaws.com/{}",
        AWS_S3_BUCKET_NAME, s3_region, thumbnail_path
    );

    let mut conn = state.get_conn().await.map_err(|e| {
        error!(error = ?e, user_id = %user_id, "Failed to get DB connection from pool");
        code_err(CodeError::POOL_ERROR, e)
    })?;

    let db_result: Result<Photograph, diesel::result::Error> =
        diesel::insert_into(photographs::table)
            .values(PhotographInsertable {
                user_id,
                photograph_shot_at,
                photograph_image_type: image_type_db_id,
                photograph_is_on_cloud: true,
                photograph_link: object_url.clone(),
                photograph_comments,
                photograph_lat,
                photograph_lon,
                photograph_thumbnail_link: thumbnail_url.clone(),
            })
            .get_result(&mut conn)
            .await;

    let photograph: Photograph = match db_result {
        Err(e) => {
            error!(
                error = ?e,
                user_id = %user_id,
                key = %image_path,
                "Failed to insert photograph row into DB"
            );
            // Clean up the image file if DB insertion fails
            tokio::fs::remove_file(&image_path).await.ok();
            return Err(code_err(CodeError::DB_INSERTION_ERROR, e));
        }
        Ok(photograph) => photograph,
    };

    drop(conn);

    // TODO: define response dto later
    Ok(http_resp(photograph, (), start))
}
