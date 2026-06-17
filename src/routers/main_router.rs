use std::sync::Arc;

use axum::{
    Router,
    extract::DefaultBodyLimit,
    http::{HeaderValue, Method, header},
    middleware::{from_fn, from_fn_with_state},
    routing::{delete, get, patch, post},
};
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};
use tower_http::{
    compression::CompressionLayer,
    cors::{AllowOrigin, CorsLayer},
};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::{
    DOMAIN_NAME,
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
            batch_list::batch_list, batch_status::batch_status, batch_upload::batch_upload,
            delete_photograph_comment::delete_photograph_comment,
            delete_photographs::delete_photographs, get_photographs::get_photographs,
            read_photograph::read_photograph,
            rescind_photograph_comment_vote::rescind_photograph_comment_vote,
            rescind_photograph_vote::rescind_photograph_vote,
            submit_photograph_comment::submit_photograph_comment,
            update_photograph_comment::update_photograph_comment,
            upload_photograph::upload_photograph, vote_photograph::vote_photograph,
            vote_photograph_comment::vote_photograph_comment,
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

mod static_assets;

use static_assets::static_asset_handler;

const MAX_REQUEST_SIZE: usize = 1024 * 1024 * 150; // 150MB
const BATCH_REQUEST_SIZE: usize = 1024 * 1024 * 1024; // 1GB (route-scoped to batch upload)

const REPLENISHED_EVERY_MILLISECONDS: u64 = 63;
const RATE_LIMIT_BURST_SIZE: u32 = 1024;

pub fn build_router(state: Arc<ServerState>) -> axum::Router {
    let auth_middleware = from_fn_with_state(state.clone(), auth_middleware);
    let require_superuser_middleware = from_fn(require_superuser_middleware);
    // let api_key_check_middleware = from_fn_with_state(state.clone(), api_key_check_middleware);
    let log_middleware = from_fn_with_state(state.clone(), log_middleware);
    let is_logged_in_middleware = from_fn_with_state(state.clone(), is_logged_in_middleware);
    let compression_middleware = CompressionLayer::new().zstd(true).gzip(true);

    // Auth is cookie-based (session_id cookie with credentials), so CORS must NOT reflect an
    // arbitrary Origin while allowing credentials. We build an explicit allow-list of trusted
    // frontend origins. Never combine `AllowOrigin::any()` with `allow_credentials(true)`:
    // tower-http panics on that combination at runtime.
    let cors_layer = CorsLayer::new()
        .allow_origin(AllowOrigin::list(build_trusted_origins(
            state.get_deployment_environment(),
        )))
        .allow_credentials(true)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION]);

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
        .route("/api/photographs/{photograph_id}", get(read_photograph))
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
        // Photograph social (votes + comments), mirroring the blog tier.
        .route(
            "/api/photographs/{photograph_id}/vote",
            post(vote_photograph),
        )
        .route(
            "/api/photographs/{photograph_id}/vote",
            delete(rescind_photograph_vote),
        )
        .route(
            "/api/photographs/{photograph_id}/comment",
            post(submit_photograph_comment),
        )
        .route(
            "/api/photographs/{photograph_id}/{comment_id}/vote",
            post(vote_photograph_comment),
        )
        .route(
            "/api/photographs/{photograph_id}/{comment_id}/vote",
            delete(rescind_photograph_comment_vote),
        )
        .route(
            "/api/photographs/{photograph_id}/{comment_id}",
            patch(update_photograph_comment),
        )
        .route(
            "/api/photographs/{photograph_id}/{comment_id}",
            delete(delete_photograph_comment),
        )
        .layer(auth_middleware.clone());

    // Batch upload accepts large multi-file bodies. The route-scoped
    // DefaultBodyLimit here is closest to the handler, so it overrides the global
    // 150MB limit added later on api_router, without widening it for other routes.
    let batch_upload_router = Router::new()
        .route("/api/photographs/batch-upload", post(batch_upload))
        .layer(DefaultBodyLimit::max(BATCH_REQUEST_SIZE));

    let superuser_router = Router::new()
        .route("/api/admin/sync-i18n-cache", get(sync_i18n_cache))
        .route("/api/blog/posts", post(submit_post))
        .route("/api/blog/{post_id}", patch(update_post))
        .route("/api/photographs/upload", post(upload_photograph))
        .route("/api/photographs/delete", delete(delete_photographs))
        .route("/api/photographs/batch/{batch_id}", get(batch_status))
        .route("/api/photographs/batches", get(batch_list))
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
        .merge(batch_upload_router)
        .layer(require_superuser_middleware.clone())
        .layer(auth_middleware.clone());

    // Combine all API routes and apply shared middleware. Rate limiting is intentionally NOT
    // applied here; it is applied to the outer router below so that the static fallback and
    // Swagger UI assets are throttled too (otherwise those surfaces are unbounded). CORS stays
    // scoped to the API router only.
    let api_router = public_router
        .merge(protected_router)
        .merge(superuser_router)
        .layer(is_logged_in_middleware)
        // .layer(api_key_check_middleware)
        .layer(log_middleware)
        .layer(DefaultBodyLimit::max(MAX_REQUEST_SIZE))
        .layer(cors_layer)
        .with_state(state.clone());

    // Final router: merge API routes and set the static asset handler as the fallback
    let router = Router::new().merge(api_router);

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

    // Set the static asset fallback first, then wrap the entire router (API + swagger + static
    // fallback) in the rate limiter so every request surface is throttled, then apply compression
    // as the outermost layer. `governor_conf` is consumed exactly once here.
    let mut router = router
        .merge(swagger_router)
        .fallback_service(get(static_asset_handler));

    if let Some(governor_conf) = governor_conf {
        router = router.layer(GovernorLayer::new(governor_conf));
    }

    router.layer(compression_middleware)
}

/// Builds the explicit list of trusted CORS origins for credentialed requests.
///
/// The session cookie is scoped to [`DOMAIN_NAME`], so the production frontend origins are
/// `https://{DOMAIN_NAME}` and `https://www.{DOMAIN_NAME}`. Non-production environments also
/// accept the usual local dev origins so the frontend dev server can talk to the API.
///
/// Additional origins may be supplied via the comma-separated `CORS_ALLOWED_ORIGINS` env var
/// (e.g. a separately-hosted frontend) without recompiling.
///
/// Origins that fail to parse into a [`HeaderValue`] are logged and skipped rather than panicking,
/// keeping startup infallible.
fn build_trusted_origins(env: DeploymentEnvironment) -> Vec<HeaderValue> {
    let mut origins: Vec<String> = vec![
        format!("https://{DOMAIN_NAME}"),
        format!("https://www.{DOMAIN_NAME}"),
    ];

    // In non-prod environments, allow the local frontend dev server origins.
    if !matches!(env, DeploymentEnvironment::Prod) {
        origins.extend(
            [
                "http://localhost:3000",
                "http://127.0.0.1:3000",
                "http://localhost:5173",
                "http://127.0.0.1:5173",
            ]
            .into_iter()
            .map(str::to_owned),
        );
    }

    // Operator-supplied extra origins (comma-separated), for split-origin deployments.
    if let Ok(extra) = std::env::var("CORS_ALLOWED_ORIGINS") {
        origins.extend(
            extra
                .split(',')
                .map(str::trim)
                .filter(|origin| !origin.is_empty())
                .map(str::to_owned),
        );
    }

    origins
        .into_iter()
        .filter_map(|origin| match HeaderValue::from_str(&origin) {
            Ok(value) => Some(value),
            Err(e) => {
                tracing::error!(origin = %origin, error = %e, "Skipping invalid CORS origin");
                None
            }
        })
        .collect()
}
