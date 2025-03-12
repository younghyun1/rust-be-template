use std::sync::Arc;

use axum::{
    middleware::from_fn_with_state,
    routing::{get, post},
};
use tower_http::{compression::CompressionLayer, cors::CorsLayer, services::ServeDir};

use crate::{
    handlers::{
        auth::{
            check_if_user_exists::check_if_user_exists_handler, login::login, logout::logout,
            reset_password::reset_password, reset_password_request::reset_password_request_process,
            signup::signup_handler, verify_user_email::verify_user_email,
        },
        blog::{get_posts::get_posts, read_post::read_post, submit_post::submit_post},
        countries::{
            get_countries::get_countries, get_country::get_country, get_language::get_language,
            get_languages::get_languages,
            get_subdivisions_for_country::get_subdivisions_for_country,
        },
        server::{
            fallback::fallback_handler, healthcheck::healthcheck,
            lookup_ip_loc::lookup_ip_location, root::root_handler,
        },
    },
    init::state::ServerState,
};

use super::middleware::{
    api_key::api_key_check_middleware, auth::auth_middleware, logging::log_middleware,
};

pub fn build_router(state: Arc<ServerState>) -> axum::Router {
    let auth_middleware = from_fn_with_state(state.clone(), auth_middleware);
    let api_key_check_middleware = from_fn_with_state(state.clone(), api_key_check_middleware);
    let log_middleware = from_fn_with_state(state.clone(), log_middleware);
    let compression_middleware = CompressionLayer::new().gzip(true);
    let cors_layer = CorsLayer::very_permissive();

    // API router with API-specific middleware (api_key_check and cors)
    let api_router = axum::Router::new()
        .route("/api/healthcheck/server", get(healthcheck))
        .route("/api/healthcheck/state", get(root_handler))
        .route("/api/dropdown/language", get(get_languages))
        .route("/api/dropdown/language/{language_id}", get(get_language))
        .route("/api/dropdown/country", get(get_countries))
        .route("/api/dropdown/country/{country_id}", get(get_country))
        .route(
            "/api/dropdown/country/{country_id}/subdivision",
            get(get_subdivisions_for_country),
        )
        .route("/api/geolocate/{ip_address}", get(lookup_ip_location))
        .route("/api/auth/signup", post(signup_handler))
        .route(
            "/api/auth/check-if-user-exists",
            post(check_if_user_exists_handler),
        )
        .route("/api/auth/login", post(login))
        .route("/api/auth/logout", post(logout).layer(auth_middleware))
        .route(
            "/api/auth/reset-password-request",
            post(reset_password_request_process),
        )
        .route("/api/auth/reset-password", post(reset_password))
        .route("/api/auth/verify-user-email", post(verify_user_email))
        .route("/api/blog/posts", get(get_posts))
        .route("/api/blog/posts/{post_id}", get(read_post))
        .route("/api/blog/posts", post(submit_post))
        .fallback(get(fallback_handler))
        .layer(api_key_check_middleware)
        .layer(cors_layer)
        .with_state(state.clone());

    // Frontend router to serve static files
    let fe_router = axum::Router::new().nest_service("/", ServeDir::new("fe"));

    // Merged router with common middleware (compression and logging)
    let merged_router = axum::Router::new()
        .merge(api_router)
        .merge(fe_router)
        .layer(compression_middleware)
        .layer(log_middleware);

    merged_router
}
