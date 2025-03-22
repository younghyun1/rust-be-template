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
            process_uploaded_images::{IMAGE_ENCODING_FORMAT, process_uploaded_image},
        },
        time::now::tokio_now,
    },
};

const MAX_SIZE_OF_UPLOADABLE_PROFILE_PICTURE: usize = 1024 * 1024 * 10; // 10MB
const ALLOWED_MIME_TYPES: [&'static str; 16] = [
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

// TODO: Unlimit upload size (each chunk should probably be bigger than 2MB?)
pub async fn upload_profile_picture(
    Extension(user_id): Extension<Uuid>,
    State(state): State<Arc<ServerState>>,
    mut multipart: Multipart,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();
    let mime: String;
    let _extension: String;

    // maximum profile picture image size of...10MB. : /
    let mut uploaded_file: Vec<u8> = Vec::with_capacity(MAX_SIZE_OF_UPLOADABLE_PROFILE_PICTURE);

    // grab the MIME type and file extension
    match multipart.next_field().await {
        Ok(Some(field)) => {
            _extension = if let Some(extension) = field.file_name() {
                match extension.rsplit('.').next() {
                    Some(ext) => ext.to_owned(),
                    None => extension.to_owned(), // If no period in filename, use the whole name
                }
            } else {
                return Err(code_err(
                    CodeError::FILE_UPLOAD_ERROR,
                    "No extensions, that's illegal!",
                ));
            };
            mime = if let Some(mime) = field.content_type() {
                mime.to_owned()
            } else {
                return Err(code_err(
                    CodeError::FILE_UPLOAD_ERROR,
                    "No MIME extensions, that's illegal!",
                ));
            };

            if !ALLOWED_MIME_TYPES.contains(&mime.as_ref()) {
                return Err(code_err(
                    CodeError::FILE_UPLOAD_ERROR,
                    "Unsupported image type; no PSDs!",
                ));
            }
        }
        Ok(None) => {
            return Err(code_err(CodeError::FILE_UPLOAD_ERROR, "File is empty!"));
        }
        Err(e) => {
            return Err(code_err(CodeError::FILE_UPLOAD_ERROR, e));
        }
    }

    // iterate through all the rest of the chunks in the multipart upload
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| code_err(CodeError::FILE_UPLOAD_ERROR, e))?
    {
        let bytes = field
            .bytes()
            .await
            .map_err(|e| code_err(CodeError::FILE_UPLOAD_ERROR, e))?;
        uploaded_file.extend_from_slice(&bytes);
    }

    // compress and process image here in a blocking thread
    let processed_image = process_uploaded_image(uploaded_file, None)
        .await
        .map_err(|e| code_err(CodeError::COULD_NOT_PROCESS_IMAGE, e))?;

    // store in filesystem or S3
    let image_id: Uuid = uuid::Uuid::new_v4();
    let (extension, image_type_db_id) = map_image_format_to_str(IMAGE_ENCODING_FORMAT);

    tokio::fs::create_dir_all("images")
        .await
        .map_err(|e| code_err(CodeError::COULD_NOT_CREATE_DIRECTORY, e))?;

    let image_path = format!("images/{}.{}", image_id, extension);

    tokio::fs::write(&image_path, processed_image)
        .await
        .map_err(|e| code_err(CodeError::COULD_NOT_WRITE_FILE, e))?;

    let mut conn = state
        .get_conn()
        .await
        .map_err(|e| code_err(CodeError::POOL_ERROR, e))?;

    let db_result = diesel::insert_into(user_profile_pictures::table)
        .values(UserProfilePictureInsertable {
            user_id,
            user_profile_picture_image_type: image_type_db_id,
            user_profile_picture_is_on_cloud: true,
            user_profile_picture_link: None,
        })
        .execute(&mut conn)
        .await;

    if let Err(e) = db_result {
        // Clean up the image file if DB insertion fails
        tokio::fs::remove_file(&image_path).await.ok();
        return Err(code_err(CodeError::DB_INSERTION_ERROR, e));
    }

    drop(conn);

    // define response dto later
    Ok(http_resp((), (), start))
}
