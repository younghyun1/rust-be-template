use std::env;
use std::path::PathBuf;
use std::sync::Arc;

use axum::{
    extract::DefaultBodyLimit,
    http::{
        StatusCode,
        header::{CONTENT_ENCODING, CONTENT_TYPE},
    },
    middleware::from_fn_with_state,
    response::{Html, IntoResponse},
    routing::{delete, get, get_service, post},
};
use tower_http::{compression::CompressionLayer, cors::CorsLayer, services::ServeDir};

use crate::{
    handlers::{
        admin::sync_i18n_cache::sync_i18n_cache,
        auth::{
            check_if_user_exists::check_if_user_exists_handler, login::login, logout::logout,
            me::me_handler, reset_password::reset_password,
            reset_password_request::reset_password_request_process, signup::signup_handler,
            verify_user_email::verify_user_email,
        },
        blog::{
            get_posts::get_posts, read_post::read_post, rescind_comment_vote::rescind_comment_vote,
            rescind_post_vote::rescind_post_vote, submit_comment::submit_comment,
            submit_post::submit_post, vote_comment::vote_comment, vote_post::vote_post,
        },
        countries::{
            get_countries::get_countries, get_country::get_country, get_language::get_language,
            get_languages::get_languages,
            get_subdivisions_for_country::get_subdivisions_for_country,
        },
        i18n::get_country_language_bundle::get_country_language_bundle,
        server::{
            healthcheck::healthcheck, lookup_ip_loc::lookup_ip_location, root::root_handler,
            visitor_board::get_visitor_board_entries,
        },
        user::upload_profile_picture::upload_profile_picture,
    },
    init::state::ServerState,
};

use super::middleware::{
    api_key::api_key_check_middleware, auth::auth_middleware,
    is_logged_in::is_logged_in_middleware, logging::log_middleware,
};

const MAX_REQUEST_SIZE: usize = 1024 * 1024 * 50; // 50MB

async fn spa_fallback() -> impl axum::response::IntoResponse {
    // Get FE_ASSETS_DIR from environment variable, fallback to "fe"
    let fe_dir = env::var("FE_ASSETS_DIR").unwrap_or_else(|_| "fe".to_string());

    // Try serving zstd-compressed first, then fallback to plain index.html
    let zstd_path = PathBuf::from(&fe_dir).join("index.html.zst");
    let html_path = PathBuf::from(&fe_dir).join("index.html");

    match tokio::fs::read(&zstd_path).await {
        Ok(zstd_bytes) => (
            StatusCode::OK,
            [
                (CONTENT_TYPE.as_str(), "text/html; charset=utf-8"),
                (CONTENT_ENCODING.as_str(), "zstd"),
            ],
            zstd_bytes,
        )
            .into_response(),
        Err(zstd_err) => {
            println!(
                "spa_fallback: Failed to read zstd-compressed index.html at {zstd_path:?}: {zstd_err}"
            );

            // Fallback to plain index.html
            match tokio::fs::read_to_string(&html_path).await {
                Ok(html) => Html(html).into_response(),
                Err(e) => {
                    println!("spa_fallback: Failed to read plain index.html at {html_path:?}: {e}");
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Internal Server Error".to_string(),
                    )
                        .into_response()
                }
            }
        }
    }
}

pub fn build_router(state: Arc<ServerState>) -> axum::Router {
    let auth_middleware = from_fn_with_state(state.clone(), auth_middleware);
    let api_key_check_middleware = from_fn_with_state(state.clone(), api_key_check_middleware);
    let log_middleware = from_fn_with_state(state.clone(), log_middleware);
    let is_logged_in_middleware = from_fn_with_state(state.clone(), is_logged_in_middleware);
    let compression_middleware = CompressionLayer::new().zstd(true);
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
        .route("/api/visitor-board", get(get_visitor_board_entries))
        .route("/api/auth/signup", post(signup_handler))
        .route("/api/auth/me", get(me_handler))
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
            get(get_country_language_bundle), // TODO: Restify
        )
        .route(
            "/api/admin/sync-country-language-bundle",
            get(sync_i18n_cache), // TODO: Restify
        ); //TODO: Get this cordoned off to some admin router requiring special elevated privileges

    let protected_router = axum::Router::new()
        .route("/api/auth/logout", post(logout))
        .route(
            "/api/user/upload-profile-picture",
            post(upload_profile_picture),
        )
        .route("/api/blog/{post_id}/vote", post(vote_post))
        .route("/api/blog/{post_id}/{comment_id}/vote", post(vote_comment))
        .route("/api/blog/{post_id}/vote", delete(rescind_post_vote))
        .route("/api/blog/{post_id}/comment", post(submit_comment))
        .route(
            "/api/blog/{post_id}/{comment_id}/vote",
            delete(rescind_comment_vote),
        )
        .layer(auth_middleware.clone());

    let api_router = public_router
        .merge(protected_router)
        .layer(is_logged_in_middleware)
        .layer(api_key_check_middleware)
        .layer(compression_middleware)
        .layer(log_middleware)
        .layer(DefaultBodyLimit::max(MAX_REQUEST_SIZE))
        .layer(cors_layer)
        .with_state(state.clone());

    // Configure ServeDir to serve static files and fall back to index.html
    let spa_fallback_service = get(spa_fallback);

    let serve_dir = ServeDir::new("fe")
        .precompressed_zstd()
        .append_index_html_on_directories(true)
        .not_found_service(spa_fallback_service);

    let static_files = get_service(serve_dir).handle_error(|error| async move {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Static file error: {error}"),
        )
    });

    // Merge API router and set static_files as the fallback

    axum::Router::new()
        .merge(api_router)
        .fallback_service(static_files)
}
