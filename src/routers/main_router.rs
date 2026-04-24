use std::sync::Arc;

use axum::{
    Router,
    extract::DefaultBodyLimit,
    http::{HeaderMap, StatusCode, Uri, header},
    middleware::{from_fn, from_fn_with_state},
    response::{IntoResponse, Response},
    routing::{delete, get, patch, post},
};
use mime_guess::from_path;
use rust_embed::Embed;
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};
use tower_http::{compression::CompressionLayer, cors::CorsLayer};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::{
    docs::ApiDoc,
    handlers::{
        admin::{get_host_stats::ws_host_stats_handler, sync_i18n_cache::sync_i18n_cache},
        auth::{
            check_if_user_exists::check_if_user_exists_handler, is_superuser::is_superuser_handler,
            login::login, logout::logout, me::me_handler, reset_password::reset_password,
            reset_password_request::reset_password_request_process, signup::signup_handler,
            verify_user_email::verify_user_email,
        },
        blog::{
            delete_comment::delete_comment, delete_post::delete_post, get_posts::get_posts,
            read_post::read_post, rescind_comment_vote::rescind_comment_vote,
            rescind_post_vote::rescind_post_vote, search_posts::search_posts,
            submit_comment::submit_comment, submit_post::submit_post,
            update_comment::update_comment, update_post::update_post, vote_comment::vote_comment,
            vote_post::vote_post,
        },
        countries::{
            get_countries::get_countries, get_country::get_country, get_language::get_language,
            get_languages::get_languages,
            get_subdivisions_for_country::get_subdivisions_for_country,
        },
        geo_ip::{lookup_ip::lookup_ip_info, lookup_my_ip::lookup_my_ip_info},
        i18n::get_ui_text_bundle::get_ui_text_bundle,
        live_chat::{get_live_chat_cache_stats, get_live_chat_messages, live_chat_ws_handler},
        photography::{
            delete_photographs::delete_photographs, get_photographs::get_photographs,
            upload_photograph::upload_photograph,
        },
        server::{
            get_host_fastfetch::get_host_fastfetch, healthcheck::healthcheck,
            lookup_ip_loc::lookup_ip_location, root::root_handler,
            visitor_board::get_visitor_board_entries,
        },
        user::{get_user_info::get_user_info, upload_profile_picture::upload_profile_picture},
        wasm_module::{
            delete_wasm_module, get_wasm_modules, serve_wasm, update_wasm_module,
            update_wasm_module_assets, upload_wasm_module,
        },
    },
    init::state::{DeploymentEnvironment, ServerState},
};

use super::middleware::{
    auth::auth_middleware, is_logged_in::is_logged_in_middleware, logging::log_middleware,
    role::require_superuser_middleware,
};

const MAX_REQUEST_SIZE: usize = 1024 * 1024 * 150; // 150MB

#[derive(Embed)]
#[folder = "fe/"]
struct EmbeddedAssets;

/// Serves static files embedded in the binary, prioritizing pre-compressed .zst files.
#[derive(Clone, Copy)]
enum ContentCodingPreference {
    Zstd,
    Gzip,
    Identity,
}

fn parse_quality(raw: &str) -> f32 {
    match raw.trim().parse::<f32>() {
        Ok(value) if (0.0..=1.0).contains(&value) => value,
        Ok(_) => 0.0,
        Err(_) => 0.0,
    }
}

fn set_max_quality(slot: &mut Option<f32>, quality: f32) {
    match *slot {
        Some(current) if current >= quality => {}
        _ => *slot = Some(quality),
    }
}

#[allow(
    clippy::manual_unwrap_or_default,
    clippy::manual_unwrap_or,
    clippy::needless_late_init
)]
fn select_static_encoding(headers: &HeaderMap) -> ContentCodingPreference {
    let accept_encoding = match headers.get(header::ACCEPT_ENCODING) {
        Some(value) => match value.to_str() {
            Ok(parsed) => parsed,
            Err(_) => return ContentCodingPreference::Identity,
        },
        None => return ContentCodingPreference::Identity,
    };

    let mut zstd_q: Option<f32> = None;
    let mut gzip_q: Option<f32> = None;
    let mut identity_q: Option<f32> = None;
    let mut wildcard_q: Option<f32> = None;

    for encoding_entry in accept_encoding.split(',') {
        let mut parts = encoding_entry.trim().split(';');
        let encoding_name = match parts.next() {
            Some(value) => value.trim().to_ascii_lowercase(),
            None => continue,
        };
        if encoding_name.is_empty() {
            continue;
        }

        let mut quality = 1.0_f32;
        for parameter in parts {
            let mut key_value = parameter.trim().splitn(2, '=');
            let key = match key_value.next() {
                Some(value) => value.trim(),
                None => "",
            };
            if key.eq_ignore_ascii_case("q") {
                let raw_quality: &str;
                match key_value.next() {
                    Some(value) => raw_quality = value,
                    None => raw_quality = "",
                }
                quality = parse_quality(raw_quality);
            }
        }

        match encoding_name.as_str() {
            "zstd" => set_max_quality(&mut zstd_q, quality),
            "gzip" | "x-gzip" => set_max_quality(&mut gzip_q, quality),
            "identity" => set_max_quality(&mut identity_q, quality),
            "*" => set_max_quality(&mut wildcard_q, quality),
            _ => {}
        }
    }

    let wildcard_default: f32;
    match wildcard_q {
        Some(value) => wildcard_default = value,
        None => wildcard_default = 0.0,
    }
    let zstd_effective = match zstd_q {
        Some(value) => value,
        None => wildcard_default,
    };
    let gzip_effective = match gzip_q {
        Some(value) => value,
        None => wildcard_default,
    };
    let identity_effective = match identity_q {
        Some(value) => value,
        None => match wildcard_q {
            Some(0.0) => 0.0,
            _ => 1.0,
        },
    };

    if zstd_effective > 0.0
        && zstd_effective >= gzip_effective
        && zstd_effective >= identity_effective
    {
        return ContentCodingPreference::Zstd;
    }

    if gzip_effective > 0.0 && gzip_effective >= identity_effective {
        return ContentCodingPreference::Gzip;
    }

    ContentCodingPreference::Identity
}

fn serve_compressed_asset(path: &str, coding: ContentCodingPreference) -> Option<Response> {
    let (extension, encoding_name) = match coding {
        ContentCodingPreference::Zstd => (".zst", "zstd"),
        ContentCodingPreference::Gzip => (".gz", "gzip"),
        ContentCodingPreference::Identity => return None,
    };

    let compressed_path = format!("{path}{extension}");
    match EmbeddedAssets::get(&compressed_path) {
        Some(content) => {
            let mime = from_path(path).first_or_octet_stream();
            Some(
                (
                    StatusCode::OK,
                    [
                        (header::CONTENT_TYPE, mime.as_ref()),
                        (header::CONTENT_ENCODING, encoding_name),
                        (header::VARY, "Accept-Encoding"),
                    ],
                    content.data,
                )
                    .into_response(),
            )
        }
        None => None,
    }
}

fn serve_uncompressed_asset(path: &str) -> Option<Response> {
    match EmbeddedAssets::get(path) {
        Some(content) => {
            let mime = from_path(path).first_or_octet_stream();
            Some(
                (
                    StatusCode::OK,
                    [
                        (header::CONTENT_TYPE, mime.as_ref()),
                        (header::VARY, "Accept-Encoding"),
                    ],
                    content.data,
                )
                    .into_response(),
            )
        }
        None => None,
    }
}

/// Serves static files embedded in the binary and negotiates zstd/gzip via Accept-Encoding.
async fn static_asset_handler(uri: Uri, headers: HeaderMap) -> impl IntoResponse {
    let mut path = uri.path().trim_start_matches('/').to_string();
    if path.is_empty() {
        path = "index.html".to_string();
    }

    let selected_encoding = select_static_encoding(&headers);

    // 1. Try an encoded version matching client support.
    if let Some(response) = serve_compressed_asset(&path, selected_encoding) {
        return response;
    }

    // 2. Fallback to the uncompressed direct path.
    if let Some(response) = serve_uncompressed_asset(&path) {
        return response;
    }

    // 3. SPA fallback: serve encoded index.html first, then plain index.html.
    if let Some(response) = serve_compressed_asset("index.html", selected_encoding) {
        return response;
    }

    if let Some(response) = serve_uncompressed_asset("index.html") {
        return response;
    }

    // 4. If nothing is found, return an error.
    (
        StatusCode::NOT_FOUND,
        [(header::VARY, "Accept-Encoding")],
        "Not Found",
    )
        .into_response()
}

const REPLENISHED_EVERY_MILLISECONDS: u64 = 63;
const RATE_LIMIT_BURST_SIZE: u32 = 1024;

pub fn build_router(state: Arc<ServerState>) -> axum::Router {
    let auth_middleware = from_fn_with_state(state.clone(), auth_middleware);
    let require_superuser_middleware = from_fn(require_superuser_middleware);
    // let api_key_check_middleware = from_fn_with_state(state.clone(), api_key_check_middleware);
    let log_middleware = from_fn_with_state(state.clone(), log_middleware);
    let is_logged_in_middleware = from_fn_with_state(state.clone(), is_logged_in_middleware);
    let compression_middleware = CompressionLayer::new().zstd(true).gzip(true);
    let cors_layer = CorsLayer::very_permissive();

    let governor_conf = match GovernorConfigBuilder::default()
        .per_millisecond(REPLENISHED_EVERY_MILLISECONDS)
        .burst_size(RATE_LIMIT_BURST_SIZE)
        .finish()
    {
        Some(conf) => Some(Arc::new(conf)),
        None => {
            tracing::error!(
                replenished_every_milliseconds = REPLENISHED_EVERY_MILLISECONDS,
                burst_size = RATE_LIMIT_BURST_SIZE,
                "Failed to build governor config; rate limiting disabled"
            );
            None
        }
    };

    // Publicly accessible API routes
    let public_router = Router::new()
        .route("/api/healthcheck/server", get(healthcheck))
        .route("/api/healthcheck/state", get(root_handler))
        .route("/api/healthcheck/fastfetch", get(get_host_fastfetch))
        .route("/ws/host-stats", get(ws_host_stats_handler))
        .route("/ws/live-chat", get(live_chat_ws_handler))
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
        .route("/api/geo-ip-info/me", get(lookup_my_ip_info))
        .route("/api/geo-ip-info/{ip_address}", get(lookup_ip_info))
        .route("/api/auth/signup", post(signup_handler))
        .route("/api/auth/me", get(me_handler))
        .route("/api/auth/is-superuser", get(is_superuser_handler))
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
        .route("/api/users/{user_name}", get(get_user_info))
        .route("/api/blog/posts", get(get_posts))
        .route("/api/blog/posts/{post_id}", get(read_post))
        .route("/api/blog/search", get(search_posts))
        .route("/api/live-chat/messages", get(get_live_chat_messages))
        .route("/api/live-chat/cache-stats", get(get_live_chat_cache_stats))
        .route("/api/i18n/ui-text", get(get_ui_text_bundle))
        .route("/api/photographs/get", get(get_photographs))
        // WASM modules - public read endpoints
        .route("/api/wasm-modules", get(get_wasm_modules))
        .route("/api/wasm-modules/{wasm_module_id}/wasm", get(serve_wasm));

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
        .route("/api/blog/{post_id}/{comment_id}", delete(delete_comment))
        .route("/api/blog/{post_id}/{comment_id}", patch(update_comment))
        .route("/api/blog/{post_id}", delete(delete_post))
        .route("/api/blog/{post_id}/comment", post(submit_comment))
        .route(
            "/api/blog/{post_id}/{comment_id}/vote",
            delete(rescind_comment_vote),
        )
        .layer(auth_middleware.clone());

    let superuser_router = Router::new()
        .route("/api/admin/sync-i18n-cache", get(sync_i18n_cache))
        .route("/api/blog/posts", post(submit_post))
        .route("/api/blog/{post_id}", patch(update_post))
        .route("/api/photographs/upload", post(upload_photograph))
        .route("/api/photographs/delete", delete(delete_photographs))
        // WASM modules - protected CUD endpoints
        .route("/api/wasm-modules", post(upload_wasm_module))
        .route(
            "/api/wasm-modules/{wasm_module_id}",
            patch(update_wasm_module),
        )
        .route(
            "/api/wasm-modules/{wasm_module_id}/assets",
            post(update_wasm_module_assets),
        )
        .route(
            "/api/wasm-modules/{wasm_module_id}",
            delete(delete_wasm_module),
        )
        .layer(require_superuser_middleware.clone())
        .layer(auth_middleware.clone());

    // Combine all API routes and apply shared middleware
    let mut api_router = public_router
        .merge(protected_router)
        .merge(superuser_router)
        .layer(is_logged_in_middleware)
        // .layer(api_key_check_middleware)
        .layer(log_middleware)
        .layer(DefaultBodyLimit::max(MAX_REQUEST_SIZE));

    if let Some(governor_conf) = governor_conf {
        api_router = api_router.layer(GovernorLayer::new(governor_conf));
    }

    let api_router = api_router.layer(cors_layer).with_state(state.clone());

    // Final router: merge API routes and set the static asset handler as the fallback
    let mut router = Router::new().merge(api_router);

    // Swagger UI is always available, but in prod it is gated behind auth + superuser.
    //
    // NOTE: `SwaggerUi` itself doesn't expose `.layer(...)`, so we nest it under a router where we
    // can apply middleware layers.
    let swagger_ui = SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi());

    let mut swagger_router = Router::new().merge(swagger_ui);

    if matches!(
        state.get_deployment_environment(),
        DeploymentEnvironment::Prod
    ) {
        swagger_router = swagger_router
            .layer(require_superuser_middleware)
            .layer(auth_middleware.clone());
    }

    router = router.merge(swagger_router).layer(compression_middleware);

    router.fallback_service(get(static_asset_handler))
}
