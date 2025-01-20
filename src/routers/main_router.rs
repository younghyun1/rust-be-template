use std::sync::Arc;

use axum::{middleware::from_fn_with_state, routing::get};
use tower_http::compression::CompressionLayer;

use crate::{handlers::root::root_handler, init::state::ServerState};

use super::middleware::logging::log_middleware;

pub fn build_router(state: Arc<ServerState>) -> axum::Router {
    let app = axum::Router::new()
        .route("/", get(root_handler))
        // .fallback(get(fallback_handler))
        .layer(from_fn_with_state(state.clone(), log_middleware))
        .layer(CompressionLayer::new())
        .with_state(state);

    app
}
