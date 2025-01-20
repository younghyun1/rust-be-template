use std::sync::Arc;

use axum::{extract::State, response::IntoResponse};
use chrono::{DateTime, Utc};
use diesel::{prelude::QueryableByName, sql_query};
use diesel_async::RunQueryDsl;
use serde_derive::Serialize;

use crate::{
    dto::responses::response_data::http_resp,
    errors::code_error::{code_err, CodeError, HandlerResult},
    init::state::ServerState,
    util::{duration_formatter::format_duration, now::t_now},
};

#[derive(Serialize)]
pub struct RootHandlerResponse {
    timestamp: DateTime<Utc>,
    server_uptime: String, // TODO: ISO-compliance
    responses_handled: u64,
    db_version: String,
    db_latency: String,
}

#[derive(QueryableByName)]
struct Version {
    #[sql_type = "diesel::sql_types::Text"]
    version: String,
}

pub async fn root_handler(
    State(state): State<Arc<ServerState>>,
) -> HandlerResult<impl IntoResponse> {
    let start = t_now();

    let db_start = t_now();

    let mut conn = state
        .get_conn()
        .await
        .map_err(|e| code_err(CodeError::DB_CONNECTION_ERROR, e))?;

    let version: Version = sql_query("SELECT version()")
        .get_result(&mut conn)
        .await
        .map_err(|e| code_err(CodeError::DB_QUERY_ERROR, e.into()))?;

    drop(conn);

    let db_elapsed = db_start.elapsed();

    Ok(http_resp::<RootHandlerResponse, ()>(
        RootHandlerResponse {
            timestamp: Utc::now(),
            server_uptime: format_duration(state.get_uptime()),
            responses_handled: state.get_responses_handled(),
            db_version: version.version,
            db_latency: format!("{:?}", db_elapsed),
        },
        (),
        start,
    ))
}
