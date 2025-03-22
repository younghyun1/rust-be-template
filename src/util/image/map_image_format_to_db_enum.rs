use image::ImageFormat;

pub fn map_image_format_to_str(image_format: ImageFormat) -> i32 {
    match image_format {
        ImageFormat::Png => 2,
        ImageFormat::Jpeg => 1,
        ImageFormat::Gif => 6,
        ImageFormat::WebP => 5,
        ImageFormat::Tiff => 3,
        ImageFormat::Bmp => 7,
        ImageFormat::Avif => 4,
        _ => 0,
    }
}
