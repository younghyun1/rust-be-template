use crate::errors::code_error::CodeErrorResp;
use std::{net::IpAddr, sync::Arc};

use axum::{extract::State, http::Request, response::IntoResponse};

use crate::{
    dto::responses::response_data::http_resp,
    errors::code_error::{CodeError, HandlerResponse, code_err},
    init::state::ServerState,
    util::{geographic::ip_info_lookup::IpInfo, time::now::tokio_now},
};

#[utoipa::path(
    get,
    path = "/api/geo-ip-info/me",
    tag = "geo",
    responses(
        (status = 200, description = "Client's IP geo-information", body = IpInfo),
        (status = 400, description = "Could not determine client IP", body = CodeErrorResp)
    )
)]
pub async fn lookup_my_ip_info<B>(
    State(state): State<Arc<ServerState>>,
    request: Request<B>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    let client_ip_str: String = match request
        .headers()
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
    {
        Some(val) => val.to_owned(),
        None => {
            return Err(code_err(
                CodeError::INVALID_IP_ADDRESS,
                "Could not determine client IP address.",
            ));
        }
    };

    let client_ip: IpAddr = match client_ip_str.parse() {
        Ok(ip) => ip,
        Err(_) => {
            return Err(code_err(
                CodeError::INVALID_IP_ADDRESS,
                "Could not parse client IP address.",
            ));
        }
    };

    let ip_info: IpInfo = match state.lookup_ip_location(client_ip) {
        Some(info) => info,
        None => {
            return Err(code_err(
                CodeError::INVALID_IP_ADDRESS,
                "Could not look up location for client IP address.",
            ));
        }
    };

    Ok(http_resp(ip_info, (), start))
}
