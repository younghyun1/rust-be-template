use std::{net::IpAddr, sync::Arc};

use axum::{
    extract::{Path, State},
    response::IntoResponse,
};

use crate::{
    dto::responses::response_data::http_resp,
    errors::code_error::{CodeError, CodeErrorResp, HandlerResponse, code_err},
    init::state::ServerState,
    util::{geographic::ip_info_lookup::IpInfo, time::now::tokio_now},
};

#[utoipa::path(
    get,
    path = "/api/geolocate/{ip_address}",
    params(
        ("ip_address" = String, Path, description = "IP address to lookup")
    ),
    responses(
        (status = 200, description = "IP location information", body = IpInfo),
        (status = 400, description = "Invalid IP address", body = CodeErrorResp)
    )
)]
pub async fn lookup_ip_location(
    State(state): State<Arc<ServerState>>,
    Path(ip_address): Path<String>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    let ip_address: IpAddr = ip_address
        .parse()
        .map_err(|e| code_err(CodeError::INVALID_IP_ADDRESS, e))?;

    let info: IpInfo = state
        .lookup_ip_location(ip_address)
        .ok_or_else(|| code_err(CodeError::INVALID_IP_ADDRESS, "IP geo info not in DB!"))?;

    Ok(http_resp(info, (), start))
}
