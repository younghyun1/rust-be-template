use std::sync::Arc;

use axum::{
    middleware::from_fn_with_state,
    routing::{get, post},
};
use tower_http::{compression::CompressionLayer, cors::CorsLayer};

use crate::{
    handlers::{
        auth::{
            check_if_user_exists::check_if_user_exists_handler, login::login, logout::logout,
            reset_password::reset_password, reset_password_request::reset_password_request_process,
            signup::signup_handler, verify_user_email::verify_user_email,
        },
        blog::{get_posts::get_posts, read_post::read_post, submit_post::submit_post},
        server::{fallback::fallback_handler, healthcheck::healthcheck, root::root_handler},
    },
    init::state::ServerState,
};

use super::middleware::{auth::auth_middleware, logging::log_middleware};

pub fn build_router(state: Arc<ServerState>) -> axum::Router {
    axum::Router::new()
        .route("/", get(root_handler))
        .route("/healthcheck", get(healthcheck))
        .route("/auth/signup", post(signup_handler))
        .route(
            "/auth/check-if-user-exists",
            post(check_if_user_exists_handler),
        )
        .route("/auth/login", post(login))
        .route(
            "/auth/logout",
            post(logout).layer(from_fn_with_state(state.clone(), auth_middleware)),
        )
        .route(
            "/auth/reset-password-request",
            post(reset_password_request_process),
        )
        .route("/auth/reset-password", post(reset_password))
        .route("/auth/verify-user-email", post(verify_user_email))
        .route("/blog/submit-post", post(submit_post))
        .route("/blog/get-posts", get(get_posts))
        .route("/blog/read-post", get(read_post))
        .fallback(get(fallback_handler))
        .layer(from_fn_with_state(state.clone(), log_middleware))
        .layer(CorsLayer::very_permissive())
        .layer(CompressionLayer::new())
        .with_state(state)
}
