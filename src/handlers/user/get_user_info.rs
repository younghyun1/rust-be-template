use std::sync::Arc;

use axum::{
    extract::{Path, State},
    response::IntoResponse,
};
use diesel::{ExpressionMethods, OptionalExtension, QueryDsl};
use diesel_async::RunQueryDsl;

use crate::{
    dto::responses::{
        response_data::http_resp, user::public_user_info_response::PublicUserInfoResponse,
    },
    errors::code_error::{CodeError, CodeErrorResp, HandlerResponse, code_err},
    init::state::ServerState,
    schema::{user_profile_pictures, users},
    util::time::now::tokio_now,
};

#[utoipa::path(
    get,
    path = "/api/users/{user_name}",
    tag = "user",
    params(("user_name" = String, Path, description = "Public username")),
    responses(
        (status = 200, description = "Public user information", body = PublicUserInfoResponse),
        (status = 404, description = "User not found", body = CodeErrorResp),
        (status = 500, description = "Internal server error", body = CodeErrorResp)
    )
)]
pub async fn get_user_info(
    State(state): State<Arc<ServerState>>,
    Path(user_name): Path<String>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();
    let user_name = user_name.trim().to_string();
    if user_name.is_empty() {
        return Err(CodeError::USER_NAME_INVALID.into());
    }

    let mut conn = state
        .get_conn()
        .await
        .map_err(|e| code_err(CodeError::POOL_ERROR, e))?;

    let user_row: Option<(uuid::Uuid, String, chrono::DateTime<chrono::Utc>, i32)> = users::table
        .filter(users::user_name.eq(&user_name))
        .select((
            users::user_id,
            users::user_name,
            users::user_created_at,
            users::user_country,
        ))
        .order(users::user_created_at.asc())
        .first(&mut conn)
        .await
        .optional()
        .map_err(|e| code_err(CodeError::DB_QUERY_ERROR, e))?;

    let (user_id, user_name, user_created_at, user_country) = match user_row {
        Some(user_row) => user_row,
        None => return Err(CodeError::USER_NOT_FOUND.into()),
    };

    let user_profile_picture_url: Option<String> = user_profile_pictures::table
        .filter(user_profile_pictures::user_id.eq(user_id))
        .order(user_profile_pictures::user_profile_picture_updated_at.desc())
        .select(user_profile_pictures::user_profile_picture_link)
        .first::<Option<String>>(&mut conn)
        .await
        .optional()
        .map_err(|e| code_err(CodeError::DB_QUERY_ERROR, e))?
        .flatten();

    drop(conn);

    let user_country_flag = state.country_flag_for_country_code(user_country).await;

    Ok(http_resp(
        PublicUserInfoResponse {
            user_id,
            user_name,
            user_created_at,
            user_country_flag,
            user_profile_picture_url,
        },
        (),
        start,
    ))
}
