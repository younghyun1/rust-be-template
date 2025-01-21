use std::{net::SocketAddr, sync::Arc};

use axum::{
    body::Body,
    extract::{ConnectInfo, State},
    http::{HeaderValue, Request, Response, StatusCode},
    middleware::Next,
};
use tokio::time::Instant;
use tracing::Level;

use crate::init::state::ServerState;

// by default, debug and below not logged at all; hence why
macro_rules! log_codeerror {
    ($level:expr, $($arg:tt)*) => {
        match $level {
            Level::ERROR => tracing::error!($($arg)*),
            Level::WARN => tracing::warn!($($arg)*),
            Level::INFO => tracing::info!($($arg)*),
            Level::DEBUG => tracing::info!($($arg)*),
            Level::TRACE => tracing::info!($($arg)*),
        }
    };
}

pub async fn log_middleware(
    State(state): State<Arc<ServerState>>,
    ConnectInfo(info): ConnectInfo<SocketAddr>,
    request: Request<Body>,
    next: Next,
) -> Response<Body> {
    let start = Instant::now();
    state.add_responses_handled();

    let method = request.method().clone();
    let path = request.uri().path().to_owned();

    let client_ip: String = match request
        .headers()
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
    {
        Some(val) => val.to_owned(),
        None => info.to_string(),
    };

    tracing::info!("RECV: {} @ {} from {}", method, path, client_ip);

    let mut response = next.run(request).await;

    if response.status() == StatusCode::OK {
        let duration = start.elapsed();

        tracing::info!(
            "RESP: {} @ {} FROM {} took {:?}",
            method,
            path,
            client_ip,
            duration
        );
    } else {
        // Use lowercase header keys for consistency and use empty strings if headers are not present
        let headers = response.headers_mut();

        let log_level = header_value_to_str(headers.get("x-error-log-level")).unwrap_or("INFO");
        let status_code = header_value_to_str(headers.get("x-error-status-code")).unwrap_or("");
        let error_code = header_value_to_str(headers.get("x-error-code")).unwrap_or("");
        let message = header_value_to_str(headers.get("x-error-message")).unwrap_or("");
        let detail = header_value_to_str(headers.get("x-error-detail")).unwrap_or("");

        let duration = start.elapsed();

        log_codeerror!(
            log_level.parse::<Level>().unwrap_or(Level::ERROR),
            "RESP: {} @ {} FROM {} {} ({}) took {:?} - Error Code: {}, Message: \"{}\", Detail: {}",
            method,
            path,
            client_ip,
            "ERROR",
            status_code,
            duration,
            error_code,
            message,
            detail
        );

        headers.remove("x-error-log-level");
        headers.remove("x-error-status-code");
        headers.remove("x-error-code");
        headers.remove("x-error-message");
        headers.remove("x-error-detail");
    }

    response
}

fn header_value_to_str(value: Option<&HeaderValue>) -> Option<&str> {
    value.and_then(|v| v.to_str().ok())
}
