use axum::{
    http::{HeaderMap, StatusCode, Uri, header},
    response::{IntoResponse, Response},
};
use mime_guess::from_path;
use rust_embed::Embed;

#[derive(Embed)]
#[folder = "fe/"]
struct EmbeddedAssets;

/// Serves static files embedded in the binary, prioritizing pre-compressed .zst files.
#[derive(Clone, Copy)]
enum ContentCodingPreference {
    Zstd,
    Gzip,
    Identity,
}

fn parse_quality(raw: &str) -> f32 {
    match raw.trim().parse::<f32>() {
        Ok(value) if (0.0..=1.0).contains(&value) => value,
        Ok(_) => 0.0,
        Err(_) => 0.0,
    }
}

fn set_max_quality(slot: &mut Option<f32>, quality: f32) {
    match *slot {
        Some(current) if current >= quality => {}
        _ => *slot = Some(quality),
    }
}

#[allow(
    clippy::manual_unwrap_or_default,
    clippy::manual_unwrap_or,
    clippy::needless_late_init
)]
fn select_static_encoding(headers: &HeaderMap) -> ContentCodingPreference {
    let accept_encoding = match headers.get(header::ACCEPT_ENCODING) {
        Some(value) => match value.to_str() {
            Ok(parsed) => parsed,
            Err(_) => return ContentCodingPreference::Identity,
        },
        None => return ContentCodingPreference::Identity,
    };

    let mut zstd_q: Option<f32> = None;
    let mut gzip_q: Option<f32> = None;
    let mut identity_q: Option<f32> = None;
    let mut wildcard_q: Option<f32> = None;

    for encoding_entry in accept_encoding.split(',') {
        let mut parts = encoding_entry.trim().split(';');
        let encoding_name = match parts.next() {
            Some(value) => value.trim().to_ascii_lowercase(),
            None => continue,
        };
        if encoding_name.is_empty() {
            continue;
        }

        let mut quality = 1.0_f32;
        for parameter in parts {
            let mut key_value = parameter.trim().splitn(2, '=');
            let key = match key_value.next() {
                Some(value) => value.trim(),
                None => "",
            };
            if key.eq_ignore_ascii_case("q") {
                let raw_quality: &str;
                match key_value.next() {
                    Some(value) => raw_quality = value,
                    None => raw_quality = "",
                }
                quality = parse_quality(raw_quality);
            }
        }

        match encoding_name.as_str() {
            "zstd" => set_max_quality(&mut zstd_q, quality),
            "gzip" | "x-gzip" => set_max_quality(&mut gzip_q, quality),
            "identity" => set_max_quality(&mut identity_q, quality),
            "*" => set_max_quality(&mut wildcard_q, quality),
            _ => {}
        }
    }

    let wildcard_default: f32;
    match wildcard_q {
        Some(value) => wildcard_default = value,
        None => wildcard_default = 0.0,
    }
    let zstd_effective = match zstd_q {
        Some(value) => value,
        None => wildcard_default,
    };
    let gzip_effective = match gzip_q {
        Some(value) => value,
        None => wildcard_default,
    };
    let identity_effective = match identity_q {
        Some(value) => value,
        None => match wildcard_q {
            Some(0.0) => 0.0,
            _ => 1.0,
        },
    };

    if zstd_effective > 0.0
        && zstd_effective >= gzip_effective
        && zstd_effective >= identity_effective
    {
        return ContentCodingPreference::Zstd;
    }

    if gzip_effective > 0.0 && gzip_effective >= identity_effective {
        return ContentCodingPreference::Gzip;
    }

    ContentCodingPreference::Identity
}

fn serve_compressed_asset(path: &str, coding: ContentCodingPreference) -> Option<Response> {
    let (extension, encoding_name) = match coding {
        ContentCodingPreference::Zstd => (".zst", "zstd"),
        ContentCodingPreference::Gzip => (".gz", "gzip"),
        ContentCodingPreference::Identity => return None,
    };

    let compressed_path = format!("{path}{extension}");
    match EmbeddedAssets::get(&compressed_path) {
        Some(content) => {
            let mime = from_path(path).first_or_octet_stream();
            Some(
                (
                    StatusCode::OK,
                    [
                        (header::CONTENT_TYPE, mime.as_ref()),
                        (header::CONTENT_ENCODING, encoding_name),
                        (header::VARY, "Accept-Encoding"),
                    ],
                    content.data,
                )
                    .into_response(),
            )
        }
        None => None,
    }
}

fn serve_uncompressed_asset(path: &str) -> Option<Response> {
    match EmbeddedAssets::get(path) {
        Some(content) => {
            let mime = from_path(path).first_or_octet_stream();
            Some(
                (
                    StatusCode::OK,
                    [
                        (header::CONTENT_TYPE, mime.as_ref()),
                        (header::VARY, "Accept-Encoding"),
                    ],
                    content.data,
                )
                    .into_response(),
            )
        }
        None => None,
    }
}

/// Serves static files embedded in the binary and negotiates zstd/gzip via Accept-Encoding.
pub(super) async fn static_asset_handler(uri: Uri, headers: HeaderMap) -> impl IntoResponse {
    let mut path = uri.path().trim_start_matches('/').to_string();
    if path.is_empty() {
        path = "index.html".to_string();
    }

    let selected_encoding = select_static_encoding(&headers);

    // 1. Try an encoded version matching client support.
    if let Some(response) = serve_compressed_asset(&path, selected_encoding) {
        return response;
    }

    // 2. Fallback to the uncompressed direct path.
    if let Some(response) = serve_uncompressed_asset(&path) {
        return response;
    }

    // 3. SPA fallback: serve encoded index.html first, then plain index.html.
    if let Some(response) = serve_compressed_asset("index.html", selected_encoding) {
        return response;
    }

    if let Some(response) = serve_uncompressed_asset("index.html") {
        return response;
    }

    // 4. If nothing is found, return an error.
    (
        StatusCode::NOT_FOUND,
        [(header::VARY, "Accept-Encoding")],
        "Not Found",
    )
        .into_response()
}
