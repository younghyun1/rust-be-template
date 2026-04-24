use std::{net::IpAddr, net::SocketAddr};

use axum::http::HeaderMap;
use tracing::error;

pub fn extract_client_ip(headers: &HeaderMap, fallback: SocketAddr) -> Option<IpAddr> {
    let raw_ip = headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(parse_ip_header)
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|value| value.to_str().ok())
                .and_then(parse_ip_header)
        });

    match raw_ip {
        Some(ip) => Some(ip),
        None => Some(fallback.ip()),
    }
}

fn parse_ip_header(value: &str) -> Option<IpAddr> {
    let first = match value.split(',').next() {
        Some(value) => value.trim(),
        None => return None,
    };
    if first.is_empty() {
        return None;
    }

    match first.parse::<IpAddr>() {
        Ok(ip) => Some(ip),
        Err(e) => {
            error!(
                error = ?e,
                client_ip = %first,
                "Could not parse forwarded IP address"
            );
            None
        }
    }
}
