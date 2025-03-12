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

use super::middleware::{api_key::api_key_check_middleware, logging::log_middleware};

pub fn build_router(state: Arc<ServerState>) -> axum::Router {
    axum::Router::new()
        .route("/", get(root_handler))
        .route("/dropdown/language/get-all", get(get_languages))
        .route("/dropdown/language", get(get_language))
        .route("/dropdown/country/get-all", get(get_countries))
        .route("/dropdown/country", get(get_country))
        .route(
            "/dropdown/country/subdivision",
            get(get_subdivisions_for_country),
        )
        .route("/healthcheck", get(healthcheck))
        .route("/geolocate", get(lookup_ip_location))
        .route("/auth/signup", post(signup_handler))
        .route(
            "/auth/check-if-user-exists",
            post(check_if_user_exists_handler),
        )
        .route("/auth/login", post(login))
        .route(
            "/auth/logout",
            post(logout), // .layer(from_fn_with_state(state.clone(), auth_middleware)),
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
        // TODO: Clean up layering, middleware, add vote related stuff, RESTify
        .fallback(get(fallback_handler))
        .layer(from_fn_with_state(state.clone(), api_key_check_middleware))
        .layer(from_fn_with_state(state.clone(), log_middleware))
        .layer(CorsLayer::very_permissive())
        .layer(CompressionLayer::new())
        .with_state(state)
}
