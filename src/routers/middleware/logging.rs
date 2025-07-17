use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
};

use axum::{
    body::Body,
    extract::{ConnectInfo, State},
    http::{HeaderValue, Request, Response, StatusCode},
    middleware::Next,
};
use chrono::Utc;
use scc::hash_map::Entry;
use tokio::time::Instant;
use tracing::{Level, error, info};

use crate::{
    build_info::{AXUM_VERSION, BUILD_TIME},
    init::state::ServerState,
    util::{geographic::ip_info_lookup::IpInfo, time::now::tokio_now},
};

// by default, debug and below not logged at all; hence why
macro_rules! log_codeerror {
    ($level:expr_2021, $kind:expr_2021, response.method = $method:expr_2021, response.path = $path:expr_2021, response.client_ip = $client_ip:expr_2021, response.status = $status:expr_2021, response.status_code = $status_code:expr_2021, response.duration = $duration:expr_2021, response.error_code = $error_code:expr_2021, response.message = $message:expr_2021, response.detail = $detail:expr_2021) => {
        match $level {
            Level::ERROR => tracing::error!(kind = %$kind, method = %$method, path = %$path, client_ip = ?$client_ip, status = %$status, status_code = %$status_code, duration = %$duration, error_code = %$error_code, message = %$message, detail = %$detail),
            Level::WARN => tracing::warn!(kind = %$kind, method = %$method, path = %$path, client_ip = ?$client_ip, status = %$status, status_code = %$status_code, duration = %$duration, error_code = %$error_code, message = %$message, detail = %$detail),
            Level::INFO => tracing::info!(kind = %$kind, method = %$method, path = %$path, client_ip = ?$client_ip, status = %$status, status_code = %$status_code, duration = %$duration, error_code = %$error_code, message = %$message, detail = %$detail),
            Level::DEBUG => tracing::debug!(kind = %$kind, method = %$method, path = %$path, client_ip = ?$client_ip, status = %$status, status_code = %$status_code, duration = %$duration, error_code = %$error_code, message = %$message, detail = %$detail),
            Level::TRACE => tracing::trace!(kind = %$kind, method = %$method, path = %$path, client_ip = ?$client_ip, status = %$status, status_code = %$status_code, duration = %$duration, error_code = %$error_code, message = %$message, detail = %$detail),
        }
    };
}

pub async fn log_middleware(
    State(state): State<Arc<ServerState>>,
    ConnectInfo(info): ConnectInfo<SocketAddr>,
    mut request: Request<Body>,
    next: Next,
) -> Response<Body> {
    let start = Instant::now();
    let now = Utc::now(); // earliest possible timestamp of server-received request

    state.add_responses_handled();

    let method = request.method().clone();
    let path = request.uri().path().to_owned();

    let client_ip: String = match request
        .headers()
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
    {
        Some(val) => val.to_owned(),
        None => info.to_string(),
    };

    let client_ip: Option<IpAddr> = match client_ip.parse() {
        Ok(ip) => Some(ip),
        Err(e) => {
            error!(error=?e, client_ip, "Could not parse IP address into IpAddr");
            None
        }
    };

    if std::env::var("CURR_ENV")
        .ok()
        .as_deref()
        .map(|v| v.trim().to_lowercase() == "prd")
        .unwrap_or(false)
    {
        tokio::spawn(log_visitors(state.clone(), client_ip.clone()));
    }

    tracing::info!(kind = %"RECV", method = %method, path = %path, client_ip = ?client_ip);
    request.extensions_mut().insert(now);

    let mut response = next.run(request).await;

    if response.status() == StatusCode::OK {
        let duration = start.elapsed();
        let headers = response.headers_mut();

        headers.insert("x-server-built-time", HeaderValue::from_static(BUILD_TIME));
        headers.insert("x-server-name", HeaderValue::from_static(AXUM_VERSION));
        headers.insert(
            "x-server-rust-version",
            HeaderValue::from_static(crate::build_info::RUST_VERSION),
        );

        tracing::info!(kind = %"RESP", method = %method, path = %path, client_ip = ?client_ip, duration = ?duration);
    } else {
        // Use lowercase header keys for consistency and use empty strings if headers are not present
        let headers = response.headers_mut();

        let log_level = header_value_to_str(headers.get("x-error-log-level")).unwrap_or("INFO");
        let status_code = header_value_to_str(headers.get("x-error-status-code")).unwrap_or("");
        let error_code = header_value_to_str(headers.get("x-error-code")).unwrap_or("");
        let message = header_value_to_str(headers.get("x-error-message")).unwrap_or("");
        let detail = header_value_to_str(headers.get("x-error-detail")).unwrap_or("");

        let duration = start.elapsed();

        log_codeerror!(
            log_level.parse::<Level>().unwrap_or(Level::ERROR),
            "ERSP",
            response.method = method,
            response.path = path,
            response.client_ip = client_ip,
            response.status = "ERROR",
            response.status_code = status_code,
            response.duration = format!("{:?}", duration),
            response.error_code = error_code,
            response.message = message,
            response.detail = detail
        );

        headers.remove("x-error-log-level");
        headers.remove("x-error-status-code");
        headers.remove("x-error-code");
        headers.remove("x-error-message");
        headers.remove("x-error-detail");

        headers.insert("x-server-built-time", HeaderValue::from_static(BUILD_TIME));
        headers.insert("x-server-name", HeaderValue::from_static(AXUM_VERSION));
        headers.insert(
            "x-server-rust-version",
            HeaderValue::from_static(crate::build_info::RUST_VERSION),
        );
    }

    response
}

fn header_value_to_str(value: Option<&HeaderValue>) -> Option<&str> {
    value.and_then(|v| v.to_str().ok())
}

async fn log_visitors(state: Arc<ServerState>, inp_ip: Option<IpAddr>) {
    let start = tokio_now();
    use diesel_async::RunQueryDsl;

    let ip: IpAddr;
    match inp_ip {
        Some(inp_ip) => {
            ip = inp_ip;
        }
        None => {
            return;
        }
    }
    let ip_info: IpInfo = match state.lookup_ip_location(ip) {
        Some(info) => info,
        None => {
            tracing::error!(kind = "ip_lookup_fail", ip = %ip, "Failed to look up IP location");
            return;
        }
    };

    // Use the already available data from IpInfo directly.
    let city = ip_info.city.clone();
    let country = ip_info.country_name.clone();
    let city_lat = ip_info.latitude;
    let city_lon = ip_info.longitude;

    // now toss that into the state scc cache
    // Use only the lower 8 bytes of the big-endian representation of each f64 as key
    let lat_bytes = city_lat.to_be_bytes();
    let lon_bytes = city_lon.to_be_bytes();

    let key = (lat_bytes, lon_bytes);

    match state.visitor_board_map.entry(key) {
        Entry::Occupied(mut occ) => {
            *occ.get_mut() += 1;
        }
        Entry::Vacant(vac) => {
            vac.insert_entry(1);
        }
    }

    // now persist to DB
    let mut conn = match state.get_conn().await {
        Ok(conn) => conn,
        Err(e) => {
            error!(kind = "db_conn_error", error = %e, "Failed to get database connection");
            return;
        }
    };

    use crate::schema::visitation_data;
    use chrono::Utc;
    use diesel::prelude::*;

    let visited_at = Utc::now();

    let new_row = (
        visitation_data::latitude.eq(city_lat),
        visitation_data::longitude.eq(city_lon),
        visitation_data::ip_address.eq(ipnet::IpNet::from(ip)),
        visitation_data::city.eq(city.clone()),
        visitation_data::country.eq(country.clone()),
        visitation_data::visited_at.eq(visited_at),
    );

    if let Err(e) = diesel::insert_into(visitation_data::table)
        .values(new_row)
        .execute(&mut conn)
        .await
    {
        error!(kind = "db_insert_error", error = %e, city = %city, country = %country, ip = %ip, "Failed to insert visitor row");
    }

    drop(conn);
    info!(duration = ?start.elapsed(), "Visitor logged successfully");
}
