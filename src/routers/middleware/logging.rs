use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
};

use axum::{
    body::Body,
    extract::{ConnectInfo, State},
    http::{HeaderMap, HeaderValue, Request, Response, StatusCode},
    middleware::Next,
};
use chrono::Utc;
use tokio::time::Instant;
use tracing::Level;
use uuid::Uuid;

use crate::{
    build_info::{BUILD_TIME_UTC, LIB_VERSION_MAP, RUSTC_VERSION},
    errors::code_error::CodeErrorLogContext,
    init::state::{DeploymentEnvironment, ServerState},
    util::extract::client_ip::extract_client_ip,
};

#[derive(Debug, Clone)]
pub struct RequestLogContext {
    pub request_id: String,
    pub received_at: chrono::DateTime<chrono::Utc>,
    pub client_ip: Option<IpAddr>,
}

macro_rules! log_request_completion {
    ($level:expr_2021, request_id = $request_id:expr_2021, method = $method:expr_2021, path = $path:expr_2021, client_ip = $client_ip:expr_2021, status_code = $status_code:expr_2021, duration = $duration:expr_2021, error_code = $error_code:expr_2021, message = $message:expr_2021, detail = $detail:expr_2021) => {
        match $level {
            Level::ERROR => tracing::error!(event = "request_completed", request_id = %$request_id, method = %$method, path = %$path, client_ip = ?$client_ip, status_code = %$status_code, duration = %$duration, error_code = ?$error_code, message = ?$message, detail = ?$detail),
            Level::WARN => tracing::warn!(event = "request_completed", request_id = %$request_id, method = %$method, path = %$path, client_ip = ?$client_ip, status_code = %$status_code, duration = %$duration, error_code = ?$error_code, message = ?$message, detail = ?$detail),
            Level::INFO => tracing::info!(event = "request_completed", request_id = %$request_id, method = %$method, path = %$path, client_ip = ?$client_ip, status_code = %$status_code, duration = %$duration, error_code = ?$error_code, message = ?$message, detail = ?$detail),
            Level::DEBUG => tracing::debug!(event = "request_completed", request_id = %$request_id, method = %$method, path = %$path, client_ip = ?$client_ip, status_code = %$status_code, duration = %$duration, error_code = ?$error_code, message = ?$message, detail = ?$detail),
            Level::TRACE => tracing::trace!(event = "request_completed", request_id = %$request_id, method = %$method, path = %$path, client_ip = ?$client_ip, status_code = %$status_code, duration = %$duration, error_code = ?$error_code, message = ?$message, detail = ?$detail),
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

    let client_ip = extract_client_ip(request.headers(), info);
    let request_id = request_id_from_headers(request.headers());

    match state.get_deployment_environment() {
        DeploymentEnvironment::Local
        | DeploymentEnvironment::Staging
        | DeploymentEnvironment::Dev => (),
        DeploymentEnvironment::Prod => {
            state.enqueue_visitor_log(client_ip).await;
        }
    }

    request.extensions_mut().insert(now);
    request.extensions_mut().insert(RequestLogContext {
        request_id: request_id.clone(),
        received_at: now,
        client_ip,
    });

    let mut response = next.run(request).await;
    add_server_headers(&mut response, &request_id);

    let duration = start.elapsed();
    let status = response.status();
    let error_context = response.extensions().get::<CodeErrorLogContext>().cloned();
    log_completed_request(
        &request_id,
        &method,
        &path,
        client_ip,
        status,
        duration,
        error_context,
    );

    response
}

fn request_id_from_headers(headers: &HeaderMap) -> String {
    match headers.get("x-request-id") {
        Some(value) => match value.to_str() {
            Ok(parsed) => {
                let trimmed = parsed.trim();
                if trimmed.is_empty() {
                    Uuid::new_v4().to_string()
                } else {
                    trimmed.to_owned()
                }
            }
            Err(_) => Uuid::new_v4().to_string(),
        },
        None => Uuid::new_v4().to_string(),
    }
}

fn add_server_headers(response: &mut Response<Body>, request_id: &str) {
    let headers = response.headers_mut();

    headers.insert(
        "x-server-built-time",
        HeaderValue::from_static(BUILD_TIME_UTC),
    );
    headers.insert(
        "x-server-name",
        match LIB_VERSION_MAP.get("axum") {
            Some(libversion) => {
                let value = format!("{} {}", libversion.get_name(), libversion.get_version());
                match HeaderValue::from_str(&value) {
                    Ok(header_value) => header_value,
                    Err(e) => {
                        tracing::warn!(error = %e, value = %value, "Failed to build x-server-name header");
                        HeaderValue::from_static("unknown")
                    }
                }
            }
            None => HeaderValue::from_static("unknown"),
        },
    );
    headers.insert(
        "x-server-rust-version",
        HeaderValue::from_static(RUSTC_VERSION),
    );
    match HeaderValue::from_str(request_id) {
        Ok(header_value) => {
            headers.insert("x-request-id", header_value);
        }
        Err(e) => {
            tracing::warn!(error = %e, request_id = %request_id, "Failed to build x-request-id header");
        }
    }
}

fn log_completed_request(
    request_id: &str,
    method: &axum::http::Method,
    path: &str,
    client_ip: Option<IpAddr>,
    status: StatusCode,
    duration: std::time::Duration,
    error_context: Option<CodeErrorLogContext>,
) {
    let duration = format!("{duration:?}");
    match error_context {
        Some(context) => {
            log_request_completion!(
                context.log_level,
                request_id = request_id,
                method = method,
                path = path,
                client_ip = client_ip,
                status_code = context.status_code.as_u16(),
                duration = duration,
                error_code = Some(context.error_code),
                message = Some(context.message.as_str()),
                detail = Some(context.detail.as_str())
            );
        }
        None => {
            let level = if status.is_server_error() {
                Level::ERROR
            } else if status.is_client_error() {
                Level::WARN
            } else {
                Level::INFO
            };
            log_request_completion!(
                level,
                request_id = request_id,
                method = method,
                path = path,
                client_ip = client_ip,
                status_code = status.as_u16(),
                duration = duration,
                error_code = Option::<u8>::None,
                message = Option::<&str>::None,
                detail = Option::<&str>::None
            );
        }
    }
}
