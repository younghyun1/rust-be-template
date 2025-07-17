// An endpoint to get the user data if logged in.

use std::sync::Arc;

use axum::{Extension, extract::State, response::IntoResponse};
use diesel::{ExpressionMethods, QueryDsl, SelectableHelper};
use diesel_async::RunQueryDsl;

use crate::{
    build_info::{BUILD_TIME_UTC, LIB_VERSION_MAP},
    domain::user::{UserInfo, UserProfilePicture},
    dto::responses::{auth::me_response::MeResponse, response_data::http_resp},
    errors::code_error::{CodeError, HandlerResponse, code_err},
    init::state::ServerState,
    routers::middleware::is_logged_in::AuthStatus,
    schema::{user_profile_pictures, users},
    util::time::now::tokio_now,
};

pub async fn me_handler(
    Extension(is_logged_in): Extension<AuthStatus>,
    State(state): State<Arc<ServerState>>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    // Determine the user_id based on AuthStatus
    let user_id = match is_logged_in {
        AuthStatus::LoggedIn(ref user_id) => Some(*user_id),
        AuthStatus::LoggedOut => None,
    };

    // If not logged in, return None for all user data fields
    if let Some(user_id) = user_id {
        let mut conn = state
            .get_conn()
            .await
            .map_err(|e| code_err(CodeError::POOL_ERROR, e))?;

        let user_info: Option<UserInfo> = users::table
            .filter(users::user_id.eq(user_id))
            .select(UserInfo::as_select())
            .first(&mut conn)
            .await
            .ok();

        let user_profile_picture: Option<UserProfilePicture> = user_profile_pictures::table
            .filter(user_profile_pictures::user_id.eq(user_id))
            .first::<UserProfilePicture>(&mut conn)
            .await
            .map_err(|e| code_err(CodeError::USER_NOT_FOUND, e))
            .ok();

        drop(conn);

        let axum_version: Option<&crate::build_info::LibVersion> = LIB_VERSION_MAP.get("axum");
        let axum_version = match axum_version {
            Some(lib) => [lib.get_name(), lib.get_version()].concat(),
            None => String::from("Unknown"),
        };

        Ok(http_resp(
            MeResponse {
                user_info,
                user_profile_picture,
                build_time: BUILD_TIME_UTC,
                axum_version: axum_version,
            },
            (),
            start,
        ))
    } else {
        let axum_version: Option<&crate::build_info::LibVersion> = LIB_VERSION_MAP.get("axum");
        let axum_version = match axum_version {
            Some(lib) => [lib.get_name(), lib.get_version()].concat(),
            None => String::from("Unknown"),
        };

        Ok(http_resp(
            MeResponse {
                user_info: None,
                user_profile_picture: None,
                build_time: BUILD_TIME_UTC,
                axum_version: axum_version,
            },
            (),
            start,
        ))
    }
}
