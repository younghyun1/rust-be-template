use image::ImageFormat;

pub const fn map_image_format_to_str(image_format: ImageFormat) -> (&'static str, i32) {
    match image_format {
        ImageFormat::Png => ("png", 2),
        ImageFormat::Jpeg => ("jpg", 1),
        ImageFormat::Gif => ("gif", 6),
        ImageFormat::WebP => ("webp", 5),
        ImageFormat::Tiff => ("tiff", 3),
        ImageFormat::Bmp => ("bmp", 7),
        ImageFormat::Avif => ("avif", 4),
        _ => ("", 0),
    }
}
