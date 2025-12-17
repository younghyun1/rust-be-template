use std::sync::Arc;

use axum::{
    Router,
    extract::DefaultBodyLimit,
    http::{StatusCode, Uri, header},
    middleware::from_fn_with_state,
    response::IntoResponse,
    routing::{delete, get, post},
};
use mime_guess::from_path;
use rust_embed::Embed;
use tower_http::{compression::CompressionLayer, cors::CorsLayer};

use crate::{
    handlers::{
        admin::{get_host_stats::ws_host_stats_handler, sync_i18n_cache::sync_i18n_cache},
        auth::{
            check_if_user_exists::check_if_user_exists_handler, login::login, logout::logout,
            me::me_handler, reset_password::reset_password,
            reset_password_request::reset_password_request_process, signup::signup_handler,
            verify_user_email::verify_user_email,
        },
        blog::{
            delete_comment::delete_comment, delete_post::delete_post, get_posts::get_posts,
            read_post::read_post, rescind_comment_vote::rescind_comment_vote,
            rescind_post_vote::rescind_post_vote, submit_comment::submit_comment,
            submit_post::submit_post, update_comment::update_comment, update_post::update_post,
            vote_comment::vote_comment, vote_post::vote_post,
        },
        countries::{
            get_countries::get_countries, get_country::get_country, get_language::get_language,
            get_languages::get_languages,
            get_subdivisions_for_country::get_subdivisions_for_country,
        },
        geo_ip::lookup_ip::lookup_ip_info,
        i18n::get_country_language_bundle::get_country_language_bundle,
        photography::{
            delete_photographs::delete_photographs, get_photographs::get_photographs,
            upload_photograph::upload_photograph,
        },
        server::{
            get_host_fastfetch::get_host_fastfetch, healthcheck::healthcheck,
            lookup_ip_loc::lookup_ip_location, root::root_handler,
            visitor_board::get_visitor_board_entries,
        },
        user::upload_profile_picture::upload_profile_picture,
    },
    init::state::ServerState,
};

use super::middleware::{
    auth::auth_middleware, is_logged_in::is_logged_in_middleware, logging::log_middleware,
};

const MAX_REQUEST_SIZE: usize = 1024 * 1024 * 150; // 150MB

#[derive(Embed)]
#[folder = "fe/"]
struct EmbeddedAssets;

/// Serves static files embedded in the binary, prioritizing pre-compressed .gz files.
async fn static_asset_handler(uri: Uri) -> impl IntoResponse {
    let mut path = uri.path().trim_start_matches('/').to_string();
    if path.is_empty() {
        path = "index.html".to_string();
    }

    // 1. Check for a pre-compressed .gz file first
    let gzip_path = format!("{path}.gz");
    if let Some(content) = EmbeddedAssets::get(&gzip_path) {
        let mime = from_path(&path).first_or_octet_stream(); // Guess MIME from original path
        return (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, mime.as_ref()),
                (header::CONTENT_ENCODING, "gzip"),
            ],
            content.data,
        )
            .into_response();
    }

    // 2. Fallback to the uncompressed file (if it exists)
    if let Some(content) = EmbeddedAssets::get(&path) {
        let mime = from_path(&path).first_or_octet_stream();
        return (
            StatusCode::OK,
            [(header::CONTENT_TYPE, mime.as_ref())],
            content.data,
        )
            .into_response();
    }

    // 3. If no direct asset is found, handle SPA fallback to index.html
    // This handles client-side routes like `/login` or `/dashboard`
    if let Some(content) = EmbeddedAssets::get("index.html.gz") {
        return (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, "text/html"),
                (header::CONTENT_ENCODING, "gzip"),
            ],
            content.data,
        )
            .into_response();
    }

    if let Some(content) = EmbeddedAssets::get("index.html") {
        return (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "text/html")],
            content.data,
        )
            .into_response();
    }

    // 4. If nothing is found, return an error
    (StatusCode::NOT_FOUND, "Not Found").into_response()
}

pub fn build_router(state: Arc<ServerState>) -> axum::Router {
    let auth_middleware = from_fn_with_state(state.clone(), auth_middleware);
    // let api_key_check_middleware = from_fn_with_state(state.clone(), api_key_check_middleware);
    let log_middleware = from_fn_with_state(state.clone(), log_middleware);
    let is_logged_in_middleware = from_fn_with_state(state.clone(), is_logged_in_middleware);
    let compression_middleware = CompressionLayer::new().gzip(true);
    let cors_layer = CorsLayer::very_permissive();

    // Publicly accessible API routes
    let public_router = Router::new()
        .route("/api/healthcheck/server", get(healthcheck))
        .route("/api/healthcheck/state", get(root_handler))
        .route("/api/healthcheck/fastfetch", get(get_host_fastfetch))
        .route("/ws/host-stats", get(ws_host_stats_handler))
        .route("/api/dropdown/language", get(get_languages))
        .route("/api/dropdown/language/{language_id}", get(get_language))
        .route("/api/dropdown/country", get(get_countries))
        .route("/api/dropdown/country/{country_id}", get(get_country))
        .route(
            "/api/dropdown/country/{country_id}/subdivision",
            get(get_subdivisions_for_country),
        )
        .route("/api/visitor-board", get(get_visitor_board_entries))
        .route("/api/geolocate/{ip_address}", get(lookup_ip_location))
        .route("/api/geo-ip-info/{ip_address}", get(lookup_ip_info))
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
        .route("/api/auth/verify-user-email", get(verify_user_email))
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
        )
        .route("/api/photographs/get", get(get_photographs));

    // API routes requiring authentication
    let protected_router = Router::new()
        .route("/api/auth/logout", post(logout))
        .route(
            "/api/user/upload-profile-picture",
            post(upload_profile_picture),
        )
        .route("/api/blog/{post_id}/vote", post(vote_post))
        .route("/api/blog/{post_id}/{comment_id}/vote", post(vote_comment))
        .route("/api/blog/{post_id}/vote", delete(rescind_post_vote))
        .route(
            "/api/blog/{post_id}/{comment_id}",
            delete(delete_comment).patch(update_comment),
        )
        .route(
            "/api/blog/{post_id}",
            delete(delete_post).patch(update_post),
        )
        .route("/api/blog/{post_id}/comment", post(submit_comment))
        .route(
            "/api/blog/{post_id}/{comment_id}/vote",
            delete(rescind_comment_vote),
        )
        .route("/api/photographs/upload", post(upload_photograph))
        .route("/api/photographs/delete", delete(delete_photographs))
        .layer(auth_middleware.clone());

    // Combine all API routes and apply shared middleware
    let api_router = public_router
        .merge(protected_router)
        .layer(is_logged_in_middleware)
        // .layer(api_key_check_middleware)
        .layer(compression_middleware)
        .layer(log_middleware)
        .layer(DefaultBodyLimit::max(MAX_REQUEST_SIZE))
        .layer(cors_layer)
        .with_state(state.clone());

    // Final router: merge API routes and set the static asset handler as the fallback
    Router::new()
        .merge(api_router)
        .fallback_service(get(static_asset_handler))
}
