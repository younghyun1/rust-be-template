use axum::response::IntoResponse;
use serde_derive::Serialize;

use super::response_meta::ResponseMeta;

#[derive(Serialize)]
pub struct Response<D: serde::Serialize, M: serde::Serialize> {
    success: bool,
    data: D,
    meta: ResponseMeta<M>,
}

impl<D: serde::Serialize, M: serde::Serialize> IntoResponse for Response<D, M> {
    fn into_response(self) -> axum::response::Response {
        axum::response::Json(self).into_response()
    }
}

pub fn http_resp<D: serde::Serialize, M: serde::Serialize>(
    data: D,
    meta: M,
    start: tokio::time::Instant,
) -> Response<D, M> {
    Response {
        success: true,
        data,
        meta: ResponseMeta::from(start, meta),
    }
}
