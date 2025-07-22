use std::sync::Arc;

use axum::{
    Extension,
    extract::{Multipart, State},
    response::IntoResponse,
};
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::{
    domain::user::UserProfilePictureInsertable,
    dto::responses::response_data::http_resp,
    errors::code_error::{CodeError, HandlerResponse, code_err},
    init::state::ServerState,
    schema::user_profile_pictures,
    util::{
        image::{
            map_image_format_to_db_enum::map_image_format_to_str,
            process_uploaded_images::{
                CyhdevImageType, IMAGE_ENCODING_FORMAT, process_uploaded_image,
            },
        },
        time::now::tokio_now,
    },
};

const MAX_SIZE_OF_UPLOADABLE_PROFILE_PICTURE: usize = 1024 * 1024 * 10; // 10MB
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
pub async fn upload_profile_picture(
    Extension(user_id): Extension<Uuid>,
    State(state): State<Arc<ServerState>>,
    mut multipart: Multipart,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();
    let mut uploaded_file: Vec<u8> = Vec::with_capacity(MAX_SIZE_OF_UPLOADABLE_PROFILE_PICTURE);
    let mut mime: String;
    let mut _extension: String = String::new();

    // Process the multipart fields
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| code_err(CodeError::FILE_UPLOAD_ERROR, e))?
    {
        // For the first field, extract metadata (file name and MIME type)
        if uploaded_file.is_empty() {
            _extension = field
                .file_name()
                .and_then(|name| name.rsplit('.').next().map(|ext| ext.to_string()))
                .ok_or_else(|| {
                    code_err(
                        CodeError::FILE_UPLOAD_ERROR,
                        "No extensions, that's illegal!",
                    )
                })?;
            mime = field
                .content_type()
                .map(|mime| mime.to_string())
                .ok_or_else(|| {
                    code_err(
                        CodeError::FILE_UPLOAD_ERROR,
                        "No MIME extensions, that's illegal!",
                    )
                })?;
            if !ALLOWED_MIME_TYPES.contains(&mime.as_ref()) {
                return Err(code_err(
                    CodeError::FILE_UPLOAD_ERROR,
                    "Unsupported image type; no PSDs!",
                ));
            }
        }
        // Read and accumulate the field's bytes.
        let bytes = field
            .bytes()
            .await
            .map_err(|e| code_err(CodeError::FILE_UPLOAD_ERROR, e))?;
        uploaded_file.extend_from_slice(&bytes);
    }

    if uploaded_file.is_empty() {
        return Err(code_err(CodeError::FILE_UPLOAD_ERROR, "File is empty!"));
    }

    // compress and process image here in a blocking thread
    let processed_image: Vec<u8> =
        process_uploaded_image(uploaded_file, None, CyhdevImageType::ProfilePicture)
            .await
            .map_err(|e| code_err(CodeError::COULD_NOT_PROCESS_IMAGE, e))?;

    // store in filesystem or S3
    let image_id: Uuid = uuid::Uuid::new_v4();
    let (extension, image_type_db_id) = map_image_format_to_str(IMAGE_ENCODING_FORMAT);

    let image_path = format!("images/{image_id}.{extension}");

    // upload to S3 here
    
    
    let mut conn = state
        .get_conn()
        .await
        .map_err(|e| code_err(CodeError::POOL_ERROR, e))?;

    let db_result: Result<Uuid, diesel::result::Error> =
        diesel::insert_into(user_profile_pictures::table)
            .values(UserProfilePictureInsertable {
                user_id,
                user_profile_picture_image_type: image_type_db_id,
                user_profile_picture_is_on_cloud: true,
                user_profile_picture_link: None,
            })
            .returning(user_profile_pictures::user_profile_picture_id)
            .get_result(&mut conn)
            .await;

    match db_result {
        Err(e) => {
            // Clean up the image file if DB insertion fails
            tokio::fs::remove_file(&image_path).await.ok();
            return Err(code_err(CodeError::DB_INSERTION_ERROR, e));
        }
        Ok(user_profile_picture_id) => {
            let _user_profile_picture_id = user_profile_picture_id;
        }
    }

    drop(conn);

    // define response dto later
    Ok(http_resp((), (), start))
}
