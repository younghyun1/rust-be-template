use std::sync::Arc;

use axum::{
    middleware::from_fn_with_state,
    routing::{get, post},
};
use tower_http::compression::CompressionLayer;

use crate::{
    handlers::{
        fallback::fallback_handler,
        root::root_handler,
        user::{check_if_user_exists::check_if_user_exists_handler, signup::signup_handler},
    },
    init::state::ServerState,
};

use super::middleware::logging::log_middleware;

pub fn build_router(state: Arc<ServerState>) -> axum::Router {
    let app = axum::Router::new()
        .route("/", get(root_handler))
        .route("/auth/signup", post(signup_handler))
        .route(
            "/auth/check-if-user-exists",
            post(check_if_user_exists_handler),
        )
        .fallback(get(fallback_handler))
        .layer(from_fn_with_state(state.clone(), log_middleware))
        .layer(CompressionLayer::new())
        .with_state(state);

    app
}
