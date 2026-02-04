use crate::errors::code_error::CodeErrorResp;
use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
};

use axum::{
    extract::{ConnectInfo, State},
    http::HeaderMap,
    response::IntoResponse,
};

use crate::{
    dto::responses::response_data::http_resp,
    errors::code_error::HandlerResponse,
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
pub async fn lookup_my_ip_info(
    State(state): State<Arc<ServerState>>,
    ConnectInfo(info): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> HandlerResponse<impl IntoResponse> {
    let start = tokio_now();

    let client_ip = extract_client_ip(&headers, info.ip());

    let ip_info: IpInfo = match state.lookup_ip_location(client_ip) {
        Some(info) => info,
        None => IpInfo {
            ip: client_ip.to_string(),
            country_code: "XX".to_string(),
            country_name: "Unknown".to_string(),
            state: String::new(),
            city: String::new(),
            postal: String::new(),
            latitude: 0.0,
            longitude: 0.0,
        },
    };

    Ok(http_resp(ip_info, (), start))
}

fn extract_client_ip(headers: &HeaderMap, fallback: IpAddr) -> IpAddr {
    let forwarded = headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(parse_ip_header);

    let real = headers
        .get("x-real-ip")
        .and_then(|value| value.to_str().ok())
        .and_then(parse_ip_header);

    forwarded.or(real).unwrap_or(fallback)
}

fn parse_ip_header(value: &str) -> Option<IpAddr> {
    let first = value.split(',').next()?.trim();

    if let Ok(ip) = first.parse::<IpAddr>() {
        return Some(ip);
    }

    if let Ok(socket) = first.parse::<SocketAddr>() {
        return Some(socket.ip());
    }

    let trimmed = first.trim_matches(&['[', ']'][..]);
    trimmed.parse::<IpAddr>().ok()
}
