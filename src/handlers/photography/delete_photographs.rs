use std::{str::FromStr, sync::Arc};

use axum::{Extension, Json, extract::State, response::IntoResponse};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::{
    dto::{
        requests::photography::delete_photographs_request::DeletePhotographsRequest,
        responses::response_data::http_resp,
    },
    errors::code_error::{CodeError, CodeErrorResp, HandlerResponse, code_err},
    init::state::ServerState,
    routers::middleware::is_logged_in::AuthStatus,
    schema::photographs::dsl::*,
    util::{auth::is_superuser::is_superuser, time::now::tokio_now},
};

#[utoipa::path(
    delete,
    path = "/api/photographs/delete",
    tag = "photography",
    request_body = DeletePhotographsRequest,
    responses(
        (status = 200, description = "Photographs deleted successfully"),
        (status = 401, description = "Unauthorized", body = CodeErrorResp),
        (status = 403, description = "Forbidden (not superuser)", body = CodeErrorResp),
        (status = 500, description = "Internal server error", body = CodeErrorResp)
    )
)]
pub async fn delete_photographs(
    Extension(is_logged_in): Extension<AuthStatus>,
    State(state): State<Arc<ServerState>>,
    Json(body): Json<DeletePhotographsRequest>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    // Only superusers can delete photographs
    let requester_id: Uuid = match is_logged_in {
        AuthStatus::LoggedIn(id) => id,
        AuthStatus::LoggedOut => {
            return Err(code_err(
                CodeError::UNAUTHORIZED_ACCESS,
                "Unauthorized deletion request!",
            ));
        }
    };

    let is_superuser = match is_superuser(state.clone(), requester_id).await {
        Ok(is_superuser) => is_superuser,
        Err(e) => {
            return Err(code_err(
                CodeError::DB_QUERY_ERROR,
                format!("Failed to check superuser status: {e}"),
            ));
        }
    };

    if !is_superuser {
        return Err(code_err(
            CodeError::UNAUTHORIZED_ACCESS,
            "Only superusers may delete photographs",
        ));
    }

    let mut conn = state
        .get_conn()
        .await
        .map_err(|e| code_err(CodeError::POOL_ERROR, e))?;

    if body.photograph_ids.is_empty() {
        return Ok(http_resp(
            serde_json::json!({
                "deleted_count": 0usize,
                "s3_deleted_count": 0usize
            }),
            (),
            start,
        ));
    }

    // Load links for all requested photographs
    let target_photographs: Vec<(String, String)> = photographs
        .filter(photograph_id.eq_any(&body.photograph_ids))
        .select((photograph_link, photograph_thumbnail_link))
        .load::<(String, String)>(&mut conn)
        .await
        .map_err(|e| code_err(CodeError::DB_QUERY_ERROR, e))?;

    // 2. Perform a single batch delete in the DB
    let deleted_rows =
        diesel::delete(photographs.filter(photograph_id.eq_any(&body.photograph_ids)))
            .execute(&mut conn)
            .await
            .map_err(|e| code_err(CodeError::DB_DELETION_ERROR, e))?;

    drop(conn);

    // 3. After DB deletion succeeds, delete objects from S3.
    //    We treat S3 deletion as a best-effort side effect. Failures are logged
    //    but do not roll back the DB (which already reflects the authoritative state).
    use aws_sdk_s3::{Client, types::ObjectIdentifier};

    let aws_config = state.aws_profile_picture_config.clone();
    let s3_client = Client::new(&aws_config);

    // Decide bucket from environment; fall back to a reasonable default.
    let bucket = std::env::var("AWS_PHOTOGRAPHS_BUCKET")
        .or_else(|_| std::env::var("AWS_IMAGE_UPLOAD_BUCKET"))
        .unwrap_or_else(|_| "cyhdev-photographs".to_string());

    // Helper: convert full URL to bucket-relative key (strip leading '/')
    fn url_to_key(url_str: &str) -> Option<String> {
        if url_str.trim().is_empty() {
            return None;
        }

        match reqwest::Url::from_str(url_str) {
            Ok(u) => {
                let path = u.path().trim_start_matches('/');
                if path.is_empty() {
                    None
                } else {
                    Some(path.to_string())
                }
            }
            Err(e) => {
                tracing::warn!(
                    url = url_str,
                    error = %e,
                    "Failed to parse photograph S3 URL; skipping key"
                );
                None
            }
        }
    }

    let mut object_keys: Vec<String> = Vec::new();
    for (link, thumb) in target_photographs {
        if let Some(k) = url_to_key(&link) {
            object_keys.push(k);
        }
        if let Some(k) = url_to_key(&thumb) {
            object_keys.push(k);
        }
    }

    let s3_deleted_count: usize;

    if object_keys.is_empty() {
        s3_deleted_count = 0;
    } else {
        let mut total_deleted = 0usize;

        for chunk in object_keys.chunks(1000) {
            let mut identifiers: Vec<ObjectIdentifier> = Vec::with_capacity(chunk.len());
            for k in chunk {
                match ObjectIdentifier::builder().key(k).build() {
                    Ok(obj_id) => identifiers.push(obj_id),
                    Err(e) => {
                        tracing::error!(
                            key = %k,
                            error = %e,
                            "Failed to build S3 ObjectIdentifier; skipping key"
                        );
                    }
                }
            }

            let delete = match aws_sdk_s3::types::Delete::builder()
                .set_objects(Some(identifiers))
                .build()
            {
                Ok(d) => d,
                Err(e) => {
                    tracing::error!(
                        error = %e,
                        "Failed to build S3 Delete request; skipping batch"
                    );
                    continue;
                }
            };

            let resp = s3_client
                .delete_objects()
                .bucket(&bucket)
                .set_delete(Some(delete))
                .send()
                .await;

            match resp {
                Ok(output) => {
                    total_deleted += output.deleted().len();
                    for err in output.errors() {
                        tracing::error!(
                            key = ?err.key(),
                            code = ?err.code(),
                            message = ?err.message(),
                            "Failed to delete S3 object for photograph"
                        );
                    }
                }
                Err(e) => {
                    tracing::error!(
                        error = %e,
                        "S3 batch deletion for photographs failed"
                    );
                }
            }
        }

        s3_deleted_count = total_deleted;
    }

    tracing::info!(
        deleted_db_rows = deleted_rows,
        s3_deleted_objects = s3_deleted_count,
        "Completed batch deletion of photographs from DB and S3"
    );

    Ok(http_resp(
        serde_json::json!({
            "deleted_count": deleted_rows,
            "s3_deleted_count": s3_deleted_count
        }),
        (),
        start,
    ))
}
