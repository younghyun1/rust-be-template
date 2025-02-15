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
        user::{
            check_if_user_exists::check_if_user_exists_handler, login::login, logout::logout,
            signup::signup_handler, verify_user_email::verify_user_email,
        },
    },
    init::state::ServerState,
};

use super::middleware::logging::log_middleware;

pub fn build_router(state: Arc<ServerState>) -> axum::Router {
    axum::Router::new()
        .route("/", get(root_handler))
        .route("/auth/signup", post(signup_handler))
        .route(
            "/auth/check-if-user-exists",
            post(check_if_user_exists_handler),
        )
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .route("/auth/verify-user-email", post(verify_user_email))
        .fallback(get(fallback_handler))
        .layer(from_fn_with_state(state.clone(), log_middleware))
        .layer(CompressionLayer::new())
        .with_state(state)
}
