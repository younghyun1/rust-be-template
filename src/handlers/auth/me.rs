// An endpoint to get the user data if logged in.

use std::sync::Arc;

use axum::{Extension, extract::State, response::IntoResponse};
use diesel::{ExpressionMethods, QueryDsl, SelectableHelper};
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::{
    domain::user::{UserInfo, UserProfilePicture},
    dto::responses::{auth::me_response::MeResponse, response_data::http_resp},
    errors::code_error::{CodeError, HandlerResponse, code_err},
    init::state::ServerState,
    schema::{user_profile_pictures, users},
    util::time::now::tokio_now,
};

pub async fn me_handler(
    Extension(user_id): Extension<Uuid>,
    State(state): State<Arc<ServerState>>,
) -> HandlerResponse<impl IntoResponse> {
    // Already UNAUTHORIZED response returned if not logged in. Just handle returning the user data.
    let start = tokio_now();

    let mut conn = state
        .get_conn()
        .await
        .map_err(|e| code_err(CodeError::POOL_ERROR, e))?;

    let user_info: UserInfo = users::table
        .filter(users::user_id.eq(user_id))
        .select(UserInfo::as_select())
        .first(&mut conn)
        .await
        .map_err(|e| code_err(CodeError::USER_NOT_FOUND, e))?;

    let user_profile_picture: Option<UserProfilePicture> = match user_profile_pictures::table
        .filter(user_profile_pictures::user_id.eq(user_id))
        .first::<UserProfilePicture>(&mut conn)
        .await
        .map_err(|e| code_err(CodeError::USER_NOT_FOUND, e))
    {
        Ok(upp) => Some(upp),
        Err(_) => None,
    };

    drop(conn);

    Ok(http_resp(
        MeResponse {
            user_info,
            user_profile_picture,
        },
        (),
        start,
    ))
}
