use std::io::{Read, Write};

use anyhow::anyhow;
use flate2::{Compression, read::GzDecoder, write::GzEncoder};

pub const HTML_CONTENT_TYPE: &str = "text/html; charset=utf-8";
pub const WASM_CONTENT_TYPE: &str = "application/wasm";

pub struct NormalizedBundle {
    pub gz_bytes: Vec<u8>,
    pub content_type: &'static str,
}

pub fn looks_like_html(data: &[u8]) -> bool {
    if data.is_empty() {
        return false;
    }

    let mut idx = 0;
    if data.len() >= 3 && data[0..3] == [0xef, 0xbb, 0xbf] {
        idx = 3;
    }

    while idx < data.len() && data[idx].is_ascii_whitespace() {
        idx += 1;
    }

    let head = &data[idx..];
    head.starts_with(b"<!DOCTYPE")
        || head.starts_with(b"<html")
        || head.starts_with(b"<HTML")
        || head.starts_with(b"<")
}

pub fn is_wasm_magic(data: &[u8]) -> bool {
    data.len() >= 4 && &data[0..4] == b"\x00asm"
}

pub fn gzip_compress_max(data: &[u8]) -> anyhow::Result<Vec<u8>> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
    encoder.write_all(data)?;
    Ok(encoder.finish()?)
}

pub fn gzip_decompress_limited(data: &[u8], max_size: usize) -> anyhow::Result<Vec<u8>> {
    let mut decoder = GzDecoder::new(data);
    let mut out = Vec::new();
    let mut buf = [0u8; 8192];

    loop {
        let n = decoder.read(&mut buf)?;
        if n == 0 {
            break;
        }
        if out.len() + n > max_size {
            return Err(anyhow!("Decompressed bundle exceeds {max_size} bytes"));
        }
        out.extend_from_slice(&buf[..n]);
    }

    Ok(out)
}

pub fn normalize_bundle_bytes(
    data: &[u8],
    is_gzipped: bool,
    is_html: bool,
    max_decompressed_size: usize,
) -> anyhow::Result<NormalizedBundle> {
    let raw_bytes = if is_gzipped {
        gzip_decompress_limited(data, max_decompressed_size)?
    } else {
        if data.len() > max_decompressed_size {
            return Err(anyhow!("Bundle exceeds {max_decompressed_size} bytes"));
        }
        data.to_vec()
    };

    if is_html {
        if !looks_like_html(&raw_bytes) {
            return Err(anyhow!(
                "Bundle marked as HTML but contents do not look like HTML"
            ));
        }
    } else if !is_wasm_magic(&raw_bytes) {
        return Err(anyhow!("Invalid WASM file (missing magic number)"));
    }

    let gz_bytes = gzip_compress_max(&raw_bytes)?;
    let content_type = if is_html {
        HTML_CONTENT_TYPE
    } else {
        WASM_CONTENT_TYPE
    };

    Ok(NormalizedBundle {
        gz_bytes,
        content_type,
    })
}

pub fn sniff_content_type_from_gzip_bytes(data: &[u8]) -> anyhow::Result<&'static str> {
    let mut decoder = GzDecoder::new(data);
    let mut buf = [0u8; 512];
    let n = decoder.read(&mut buf)?;
    let head = &buf[..n];

    if is_wasm_magic(head) {
        Ok(WASM_CONTENT_TYPE)
    } else if looks_like_html(head) {
        Ok(HTML_CONTENT_TYPE)
    } else {
        Err(anyhow!("Unable to detect bundle content type"))
    }
}
