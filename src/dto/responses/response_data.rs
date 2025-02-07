use axum::http::header::SET_COOKIE;
use axum::http::HeaderValue;
use axum::response::IntoResponse;
use serde_derive::Serialize;
use tracing::error;

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

pub struct ResponseWithCookies<'a, D: serde::Serialize, M: serde::Serialize> {
    response: Response<D, M>,
    cookies_to_set: Option<Vec<axum_extra::extract::cookie::Cookie<'a>>>,
    cookies_to_unset: Option<Vec<axum_extra::extract::cookie::Cookie<'a>>>,
}

impl<D: serde::Serialize, M: serde::Serialize> IntoResponse for ResponseWithCookies<'_, D, M> {
    fn into_response(self) -> axum::response::Response {
        let mut response = self.response.into_response();
        let headers = response.headers_mut();

        if let Some(cookies) = self.cookies_to_set {
            for cookie in cookies {
                match HeaderValue::from_str(&cookie.to_string()) {
                    Ok(header_value) => {
                        headers.append(SET_COOKIE, header_value);
                    }
                    Err(e) => {
                        error!(cookie_val = %cookie.to_string(), error = %e, "Failed to set cookie");
                    }
                }
            }
        }

        if let Some(cookies) = self.cookies_to_unset {
            for mut cookie in cookies {
                cookie.make_removal();
                match HeaderValue::from_str(&cookie.to_string()) {
                    Ok(header_value) => {
                        headers.append(SET_COOKIE, header_value);
                    }
                    Err(e) => {
                        error!(cookie_val = %cookie.to_string(), error = %e, "Failed to unset cookie");
                    }
                }
            }
        }

        response
    }
}

pub fn http_resp_with_cookies<'a, D: serde::Serialize, M: serde::Serialize>(
    data: D,
    meta: M,
    start: tokio::time::Instant,
    cookies_to_set: Option<Vec<axum_extra::extract::cookie::Cookie<'a>>>,
    cookies_to_unset: Option<Vec<axum_extra::extract::cookie::Cookie<'a>>>,
) -> ResponseWithCookies<'a, D, M> {
    ResponseWithCookies {
        response: http_resp(data, meta, start),
        cookies_to_set,
        cookies_to_unset,
    }
}
