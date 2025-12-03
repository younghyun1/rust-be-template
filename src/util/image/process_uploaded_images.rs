use anyhow::anyhow;
use fast_image_resize::{PixelType, ResizeOptions, Resizer, images::Image as FastImage};
use image::{
    DynamicImage, GenericImageView, ImageFormat, load_from_memory, load_from_memory_with_format,
};
use std::{io::Cursor, time::Instant};
use tracing::info;

pub const IMAGE_ENCODING_FORMAT: ImageFormat = ImageFormat::Avif;

#[repr(u8)]
pub enum CyhdevImageType {
    ProfilePicture,
    Photograph,
    Thumbnail,
}

impl CyhdevImageType {
    pub fn max_long_width(&self) -> u32 {
        match self {
            CyhdevImageType::ProfilePicture => 400,
            CyhdevImageType::Photograph => 6000,
            CyhdevImageType::Thumbnail => 800,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            CyhdevImageType::ProfilePicture => "profile_picture",
            CyhdevImageType::Photograph => "photograph",
            CyhdevImageType::Thumbnail => "thumbnail",
        }
    }
}

pub fn format_size(bytes: usize) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    let b = bytes as f64;
    if b < KB {
        format!("{bytes} B")
    } else if b < MB {
        format!("{:.2} KB", b / KB)
    } else if b < GB {
        format!("{:.2} MB", b / MB)
    } else {
        format!("{:.2} GB", b / GB)
    }
}

pub async fn process_uploaded_image(
    bits: Vec<u8>,
    format: Option<image::ImageFormat>,
    image_type: CyhdevImageType,
) -> anyhow::Result<Vec<u8>> {
    let image_type_label = image_type.as_str();
    let original_size = bits.len();
    let start = Instant::now();

    let result = tokio::task::spawn_blocking(move || {
        // Attempt to decode the image from memory, using the provided format if auto-detection fails.
        let img = match load_from_memory(&bits) {
            Ok(img) => img,
            Err(e) => {
                if let Some(fmt) = format {
                    load_from_memory_with_format(&bits, fmt).map_err(|e2| {
                        anyhow!("Failed to decode image with the provided format: {:?}", e2)
                    })?
                } else {
                    return Err(anyhow!("Failed to decode image: {:?}", e));
                }
            }
        };

        // Determine dimensions and resize if necessary.
        let (width, height) = img.dimensions();
        let max_edge = width.max(height);
        let resized_img = if max_edge > image_type.max_long_width() {
            let scale = image_type.max_long_width() as f64 / max_edge as f64;
            let new_width = (width as f64 * scale).round().max(1.0) as u32;
            let new_height = (height as f64 * scale).round().max(1.0) as u32;

            let src_data = img.to_rgba8().into_raw();
            let src_image = FastImage::from_vec_u8(width, height, src_data, PixelType::U8x4)
                .map_err(|_| anyhow!("Failed to create fast image from buffer"))?;

            let mut dst_image = FastImage::new(new_width, new_height, src_image.pixel_type());

            let mut resizer = Resizer::new();
            resizer
                .resize(&src_image, &mut dst_image, &ResizeOptions::default())
                .map_err(|_| anyhow!("Failed to resize image"))?;

            let dst_data = dst_image.into_vec();
            let dst_buffer =
                image::ImageBuffer::<image::Rgba<u8>, _>::from_raw(new_width, new_height, dst_data)
                    .ok_or(anyhow!("Failed to create image buffer"))?;

            DynamicImage::ImageRgba8(dst_buffer)
        } else {
            img
        };

        let mut output_buffer = Vec::new();
        {
            let mut cursor = Cursor::new(&mut output_buffer);
            resized_img
                .write_to(&mut cursor, IMAGE_ENCODING_FORMAT)
                .map_err(|e| anyhow!("Failed to encode image as AVIF: {:?}", e))?;
        }
        Ok(output_buffer)
    })
    .await
    .map_err(|e| anyhow!("Blocking image processing task panicked: {:?}", e))?;

    let elapsed = start.elapsed();
    if let Ok(ref processed) = result {
        let processed_size = processed.len();
        info!(
            image_type = image_type_label,
            original_size_bytes = original_size,
            original_size_human = %format_size(original_size),
            processed_size_bytes = processed_size,
            processed_size_human = %format_size(processed_size),
            elapsed_ms = %elapsed.as_millis(),
            "Completed image processing and AVIF encoding"
        );
    }

    result
}
