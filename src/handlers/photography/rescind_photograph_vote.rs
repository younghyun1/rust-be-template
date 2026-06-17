//! `DELETE /api/photographs/{photograph_id}/vote` — remove the caller's vote.

use std::sync::Arc;

use axum::{
    Extension,
    extract::{Path, State},
    response::IntoResponse,
};
use diesel::{BoolExpressionMethods, ExpressionMethods, QueryDsl};
use diesel_async::{AsyncConnection, RunQueryDsl};
use uuid::Uuid;

use crate::{
    domain::photography::social::VoteCounts,
    dto::responses::response_data::http_resp,
    errors::code_error::{CodeError, CodeErrorResp, HandlerResponse, code_err},
    init::state::ServerState,
    schema::{photograph_votes, photographs},
    util::time::now::tokio_now,
};

#[utoipa::path(
    delete,
    path = "/api/photographs/{photograph_id}/vote",
    tag = "photography",
    params(("photograph_id" = Uuid, Path, description = "Photograph to rescind a vote on")),
    responses(
        (status = 200, description = "Vote rescinded", body = CodeErrorResp),
        (status = 401, description = "Unauthorized", body = CodeErrorResp),
        (status = 404, description = "Vote does not exist", body = CodeErrorResp)
    )
)]
pub async fn rescind_photograph_vote(
    Extension(user_id): Extension<Uuid>,
    State(state): State<Arc<ServerState>>,
    Path(photograph_id): Path<Uuid>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    let mut conn = state
        .get_conn()
        .await
        .map_err(|e| code_err(CodeError::POOL_ERROR, e))?;

    match conn
        .transaction::<_, diesel::result::Error, _>(async |conn| {
            let affected_rows = diesel::delete(
                photograph_votes::table.filter(
                    photograph_votes::photograph_id
                        .eq(photograph_id)
                        .and(photograph_votes::user_id.eq(user_id)),
                ),
            )
            .execute(&mut *conn)
            .await?;

            if affected_rows == 0 {
                return Err(diesel::result::Error::NotFound);
            }

            let counts: VoteCounts = diesel::sql_query(
                "SELECT \
                        COUNT(*) FILTER (WHERE is_upvote = true) AS upvote_count, \
                        COUNT(*) FILTER (WHERE is_upvote = false) AS downvote_count \
                     FROM photograph_votes \
                     WHERE photograph_id = $1",
            )
            .bind::<diesel::sql_types::Uuid, Uuid>(photograph_id)
            .get_result(&mut *conn)
            .await?;

            diesel::update(photographs::table.filter(photographs::photograph_id.eq(photograph_id)))
                .set((
                    photographs::photograph_total_upvotes.eq(counts.upvote_count),
                    photographs::photograph_total_downvotes.eq(counts.downvote_count),
                ))
                .execute(&mut *conn)
                .await?;

            Ok(())
        })
        .await
    {
        Ok(()) => {}
        Err(diesel::result::Error::NotFound) => {
            return Err(CodeError::UPVOTE_DOES_NOT_EXIST.into());
        }
        Err(e) => return Err(code_err(CodeError::DB_DELETION_ERROR, e)),
    }

    Ok(http_resp((), (), start))
}
