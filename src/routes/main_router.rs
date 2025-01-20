use std::sync::Arc;

use axum::routing::get;
use tower_http::compression::CompressionLayer;

use crate::{handlers::root::root_handler, init::state::ServerState};

pub fn build_router(state: Arc<ServerState>) -> axum::Router {
    let app = axum::Router::new()
        .route("/", get(root_handler))
        // .fallback(get(fallback_handler))
        .layer(CompressionLayer::new())
        .with_state(state);

    app
}
