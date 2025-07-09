use std::{net::Ipv4Addr, sync::Arc};

use axum::{
    extract::{Path, State},
    response::IntoResponse,
};

use crate::{
    dto::responses::response_data::http_resp,
    errors::code_error::{CodeError, HandlerResponse, code_err},
    init::state::ServerState,
    util::{geographic::ip_info_lookup::IpInfo, time::now::tokio_now},
};

pub async fn lookup_ip_location(
    State(state): State<Arc<ServerState>>,
    Path(ip_address): Path<String>,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    let ip_address: Ipv4Addr = ip_address
        .parse()
        .map_err(|e| code_err(CodeError::INVALID_IP_ADDRESS, e))?;

    let info: IpInfo = state
        .lookup_ip_location(ip_address)
        .ok_or_else(|| code_err(CodeError::INVALID_IP_ADDRESS, "IP geo info not in DB!"))?;

    Ok(http_resp(info, (), start))
}
