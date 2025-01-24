use std::{net::SocketAddr, sync::Arc};

use axum::{
    body::Body,
    extract::{ConnectInfo, State},
    http::{HeaderValue, Request, Response, StatusCode},
    middleware::Next,
};
use chrono::Utc;
use tokio::time::Instant;
use tracing::Level;

use crate::init::state::ServerState;

// by default, debug and below not logged at all; hence why
macro_rules! log_codeerror {
    ($level:expr, $kind:expr, response.method = $method:expr, response.path = $path:expr, response.client_ip = $client_ip:expr, response.status = $status:expr, response.status_code = $status_code:expr, response.duration = $duration:expr, response.error_code = $error_code:expr, response.message = $message:expr, response.detail = $detail:expr) => {
        match $level {
            Level::ERROR => tracing::error!(kind = $kind, method = %$method, path = %$path, client_ip = %$client_ip, status = %$status, status_code = %$status_code, duration = %$duration, error_code = %$error_code, message = %$message, detail = %$detail),
            Level::WARN => tracing::warn!(kind = $kind, method = %$method, path = %$path, client_ip = %$client_ip, status = %$status, status_code = %$status_code, duration = %$duration, error_code = %$error_code, message = %$message, detail = %$detail),
            Level::INFO => tracing::info!(kind = $kind, method = %$method, path = %$path, client_ip = %$client_ip, status = %$status, status_code = %$status_code, duration = %$duration, error_code = %$error_code, message = %$message, detail = %$detail),
            Level::DEBUG => tracing::debug!(kind = $kind, method = %$method, path = %$path, client_ip = %$client_ip, status = %$status, status_code = %$status_code, duration = %$duration, error_code = %$error_code, message = %$message, detail = %$detail),
            Level::TRACE => tracing::trace!(kind = $kind, method = %$method, path = %$path, client_ip = %$client_ip, status = %$status, status_code = %$status_code, duration = %$duration, error_code = %$error_code, message = %$message, detail = %$detail),
        }
    };
}

pub async fn log_middleware(
    State(state): State<Arc<ServerState>>,
    ConnectInfo(info): ConnectInfo<SocketAddr>,
    mut request: Request<Body>,
    next: Next,
) -> Response<Body> {
    let start = Instant::now();
    let now = Utc::now(); // earliest possible timestamp of server-received request

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

    tracing::info!(kind = "RECV", method = %method, path = %path, client_ip = %client_ip);
    request.extensions_mut().insert(now);

    let mut response = next.run(request).await;

    if response.status() == StatusCode::OK {
        let duration = start.elapsed();

        tracing::info!(kind = "RESP", method = %method, path = %path, client_ip = %client_ip, duration = ?duration);
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
            "ERSP",
            response.method = method,
            response.path = path,
            response.client_ip = client_ip,
            response.status = "ERROR",
            response.status_code = status_code,
            response.duration = format!("{:?}", duration),
            response.error_code = error_code,
            response.message = message,
            response.detail = detail
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
