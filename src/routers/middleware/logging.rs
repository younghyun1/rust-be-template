use std::sync::Arc;

use axum::{
    body::Body,
    debug_middleware,
    extract::State,
    http::{Request, Response},
    middleware::Next,
};
use tokio::time::Instant;

use crate::init::state::ServerState;

#[debug_middleware]
pub async fn log_middleware(
    State(state): State<Arc<ServerState>>,
    request: Request<Body>,
    next: Next,
) -> Response<Body> {
    let start = Instant::now();
    state.add_responses_handled();

    let method = request.method().clone();
    let path = request.uri().path().to_owned();
    let client_ip = request
        .headers()
        .get("X-Forwarded-For")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("unknown")
        .to_owned();

    tracing::info!("RECV: {} @ {} FROM {}", method, path, client_ip);

    let response = next.run(request).await;

    let duration = start.elapsed();

    tracing::info!(
        "RESP: {} @ {} FROM {} took {:?}",
        method,
        path,
        client_ip,
        duration
    );

    response
}
