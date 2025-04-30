use std::sync::Arc;

use axum::{
    extract::DefaultBodyLimit,
    http::StatusCode,
    middleware::from_fn_with_state,
    response::IntoResponse,
    routing::{get, get_service, post},
};
use tower_http::{compression::CompressionLayer, cors::CorsLayer, services::ServeDir};

use crate::{
    handlers::{
        admin::sync_i18n_cache::sync_i18n_cache,
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
        i18n::get_country_language_bundle::get_country_language_bundle,
        server::{healthcheck::healthcheck, lookup_ip_loc::lookup_ip_location, root::root_handler},
        user::upload_profile_picture::upload_profile_picture,
    },
    init::state::ServerState,
};

use super::middleware::{
    api_key::api_key_check_middleware, auth::auth_middleware, logging::log_middleware,
};

const MAX_REQUEST_SIZE: usize = 1024 * 1024 * 50; // 50MB

async fn spa_fallback() -> impl axum::response::IntoResponse {
    match tokio::fs::read_to_string("fe/index.html").await {
        Ok(html) => axum::response::Html(html).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Internal Server Error".to_string(),
        )
            .into_response(),
    }
}

pub fn build_router(state: Arc<ServerState>) -> axum::Router {
    let auth_middleware = from_fn_with_state(state.clone(), auth_middleware);
    let api_key_check_middleware = from_fn_with_state(state.clone(), api_key_check_middleware);
    let log_middleware = from_fn_with_state(state.clone(), log_middleware);
    let compression_middleware = CompressionLayer::new().gzip(true);
    let cors_layer = CorsLayer::very_permissive();

    // API router with API-specific middleware
    let public_router = axum::Router::new()
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
        .route(
            "/api/auth/reset-password-request",
            post(reset_password_request_process),
        )
        .route("/api/auth/reset-password", post(reset_password))
        .route("/api/auth/verify-user-email", post(verify_user_email))
        .route("/api/blog/posts", get(get_posts))
        .route("/api/blog/posts/{post_id}", get(read_post))
        .route("/api/blog/posts", post(submit_post))
        .route(
            "/api/i18n/country-language-bundle",
            get(get_country_language_bundle),
        )
        .route(
            "/api/admin/sync-country-language-bundle",
            get(sync_i18n_cache),
        ); //TODO: Get this cordoned off to some admin router requiring special elevated privileges

    let protected_router = axum::Router::new()
        .route("/api/auth/logout", post(logout))
        .route(
            "/api/user/upload-profile-picture",
            post(upload_profile_picture),
        )
        .layer(auth_middleware.clone());

    let api_router = public_router
        .merge(protected_router)
        .layer(api_key_check_middleware)
        .layer(cors_layer)
        .with_state(state.clone());

    // Configure ServeDir to serve static files and fall back to index.html
    let spa_fallback_service = get(spa_fallback);

    let serve_dir = ServeDir::new("fe")
        .append_index_html_on_directories(true)
        .not_found_service(spa_fallback_service);

    let static_files = get_service(serve_dir).handle_error(|error| async move {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Static file error: {}", error),
        )
    });

    // Merge API router and set static_files as the fallback
    let app = axum::Router::new()
        .merge(api_router)
        .fallback_service(static_files)
        .layer(compression_middleware)
        .layer(log_middleware)
        .layer(DefaultBodyLimit::max(MAX_REQUEST_SIZE));

    app
}
