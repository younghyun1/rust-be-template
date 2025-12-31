use std::{net::IpAddr, sync::Arc};

use axum::{
    extract::{Path, State},
    response::IntoResponse,
};
use tracing::error;

use crate::{
    dto::responses::response_data::http_resp,
    errors::code_error::{CodeError, CodeErrorResp, HandlerResponse, code_err},
    init::state::ServerState,
    util::{geographic::ip_info_lookup::IpInfo, time::now::tokio_now},
};

#[utoipa::path(
    get,
    path = "/api/geo-ip-info/{ip_address}",
    tag = "geo",
    params(
        ("ip_address" = String, Path, description = "IP address to lookup")
    ),
    responses(
        (status = 200, description = "IP geo-information", body = IpInfo),
        (status = 400, description = "Invalid IP address", body = CodeErrorResp)
    )
)]
pub async fn lookup_ip_info(
    State(state): State<Arc<ServerState>>,
    Path(ip_addr_str): Path<String>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    let inp_ip: Option<IpAddr> = match ip_addr_str.parse() {
        Ok(ip) => Some(ip),
        Err(e) => {
            error!(
                error = ?e,
                client_ip = %ip_addr_str,
                "Could not parse IP address into IpAddr"
            );
            None
        }
    };

    let ip: IpAddr;
    match inp_ip {
        Some(inp_ip) => {
            ip = inp_ip;
        }
        None => {
            return Err(code_err(
                CodeError::INVALID_IP_ADDRESS,
                "Invalid IP address string! Input IPv4 or IPv6 string, please.",
            ));
        }
    }

    let ip_info: IpInfo = match state.lookup_ip_location(ip) {
        Some(info) => info,
        None => {
            tracing::error!(kind = "ip_lookup_fail", ip = %ip, "Failed to look up IP location");
            return Err(code_err(
                CodeError::INVALID_IP_ADDRESS,
                "Invalid IP address string! Input IPv4 or IPv6 string, please.",
            ));
        }
    };

    Ok(http_resp(ip_info, (), start))
}
