use anyhow::anyhow;
use image::{
    GenericImageView, ImageFormat, imageops::FilterType, load_from_memory,
    load_from_memory_with_format,
};
use std::io::Cursor;

pub const IMAGE_ENCODING_FORMAT: ImageFormat = ImageFormat::Avif;

pub async fn process_uploaded_image(
    bits: Vec<u8>,
    format: Option<image::ImageFormat>,
) -> anyhow::Result<Vec<u8>> {
    tokio::task::spawn_blocking(move || {
        // Attempt to decode the image from memory, using the provided format if auto-detection fails.
        let img = match load_from_memory(&bits) {
            Ok(img) => img,
            Err(e) => {
                if let Some(fmt) = format {
                    load_from_memory_with_format(&bits, fmt)
                        .expect("Failed to decode image with the provided format")
                } else {
                    return Err(anyhow!("Failed to decode image: {:?}", e));
                }
            }
        };

        // Determine dimensions and resize if necessary. The long edge is capped at 3840 pixels.
        let (width, height) = img.dimensions();
        let max_edge = width.max(height);
        let target_edge = 800;
        let resized_img = if max_edge > target_edge {
            let scale = target_edge as f64 / max_edge as f64;
            let new_width = (width as f64 * scale).round() as u32;
            let new_height = (height as f64 * scale).round() as u32;
            img.resize(new_width, new_height, FilterType::Lanczos3)
        } else {
            img
        };

        let mut output_buffer = Vec::new();
        {
            let mut cursor = Cursor::new(&mut output_buffer);
            resized_img
                .write_to(&mut cursor, IMAGE_ENCODING_FORMAT)
                .map_err(|e| anyhow!("Failed to encode image as WebP: {:?}", e))?;
        }
        Ok(output_buffer)
    })
    .await
    .map_err(|e| anyhow!("Blocking image processing task panicked: {:?}", e))?
}
