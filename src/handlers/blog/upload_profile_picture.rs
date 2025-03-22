use std::sync::Arc;

use axum::{
    extract::{Multipart, State},
    response::IntoResponse,
};
use uuid::Uuid;

use crate::{
    dto::responses::response_data::http_resp,
    errors::code_error::{CodeError, HandlerResponse, code_err},
    init::state::ServerState,
    util::{image::process_uploaded_images::process_uploaded_image, time::now::tokio_now},
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
    let mut conn = state
        .get_conn()
        .await
        .map_err(|e| code_err(CodeError::POOL_ERROR, e))?;

    drop(conn);

    // define response dto later
    Ok(http_resp((), (), start))
}
