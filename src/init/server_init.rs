use std::{
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use axum::{
    handler::HandlerWithoutStateExt,
    http::{StatusCode, Uri, uri::Authority},
    response::Redirect,
};
use axum_extra::extract::Host;
use axum_server::tls_rustls::RustlsConfig;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::pooled_connection::bb8::Pool;
use lettre::{AsyncSmtpTransport, Tokio1Executor, transport::smtp::authentication::Credentials};
use tracing::info;

use crate::{
    init::config::EmailConfig, jobs::job_funcs::init_scheduler::task_init,
    routers::main_router::build_router,
};

use super::{config::DbConfig, state::ServerState};

pub async fn server_init_proc(start: tokio::time::Instant) -> anyhow::Result<()> {
    let num_cores: u32 = num_cpus::get_physical() as u32;

    let host_ip: IpAddr = std::env::var("HOST_IP")
        .map_err(|e| anyhow::anyhow!("Failed to load HOST_IP from .env: {}", e))?
        .parse::<std::net::IpAddr>()
        .map_err(|e| anyhow::anyhow!("Failed to parse HOST_IP as IP address: {}", e))?;

    let host_port: u16 = std::env::var("HOST_PORT")
        .map_err(|e| anyhow::anyhow!("Failed to load HOST_PORT from .env: {}", e))?
        .parse()
        .map_err(|e| anyhow::anyhow!("Failed to parse HOST_PORT as u16: {}", e))?;

    let host_socket_addr: SocketAddr = SocketAddr::new(host_ip, host_port);

    info!(host_socket_addr = %host_socket_addr, "Loaded host configuration.");

    let cert_chain_path: PathBuf = std::env::var("CERT_CHAIN_DIR")
        .map_err(|_| anyhow::anyhow!("CERT_CHAIN_DIR environment variable is not set"))
        .map(PathBuf::from)?;

    let priv_key_path: PathBuf = std::env::var("PRIV_KEY_DIR")
        .map_err(|_| anyhow::anyhow!("PRIV_KEY_DIR environment variable is not set"))
        .map(PathBuf::from)?;

    // configure certificate and private key used by https
    let config = RustlsConfig::from_pem_file(cert_chain_path, priv_key_path)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to load TLS config: {}", e))?;

    info!("Loaded keys.");

    let db_config = DbConfig::from_env()
        .map_err(|e| anyhow::anyhow!("Failed to get DB config from environment: {}", e))?
        .to_url()
        .map_err(|e| anyhow::anyhow!("Failed to convert DB config to URL: {}", e))?;
    info!("Loaded DB configuration.");

    let pool_config =
        AsyncDieselConnectionManager::<diesel_async::AsyncPgConnection>::new(db_config);

    let pool = Pool::builder()
        .min_idle(Some(num_cores))
        .max_size(num_cores * 10u32)
        .connection_timeout(Duration::from_secs(2))
        .build(pool_config)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to build connection pool: {}", e))?;

    info!(
        "Connection pool built with {} connections. Will scale to {} connections.",
        num_cores,
        num_cores * 10u32
    );

    let app_name_version: String = std::env::var("APP_NAME_VERSION")
        .map_err(|e| anyhow::anyhow!("Failed to load APP_NAME_VAR from .env: {}", e))?;

    let email_config = EmailConfig::from_env()
        .map_err(|e| anyhow::anyhow!("Failed to load email configs from .env: {}", e))?;
    let email_creds: Credentials = email_config.to_creds();
    let email_client: AsyncSmtpTransport<Tokio1Executor> =
        AsyncSmtpTransport::<Tokio1Executor>::relay(&email_config.get_url())?
            .credentials(email_creds)
            .build();

    info!(
        "Email client configured; relay at {}",
        email_config.get_url()
    );

    let state = Arc::new(
        ServerState::builder()
            .app_name_version(app_name_version)
            .pool(pool)
            .server_start_time(start)
            .email_client(email_client)
            .build()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to build ServerState: {}", e))?,
    );

    // Failures on these should be fatal.
    state.synchronize_post_info_cache().await;
    state.sync_country_data().await?;
    state.sync_i18n_data().await?;
    state.sync_visitor_board_data().await?;

    let api_key = std::env::var("X_API_KEY")
        .map_err(|e| anyhow::anyhow!("Failed to load X_API_KEY from .env: {}", e))?;

    let api_key_uuid = uuid::Uuid::parse_str(&api_key)
        .map_err(|e| anyhow::anyhow!("Failed to parse X_API_KEY as UUID: {}", e))?;

    drop(api_key);

    state.insert_api_key(api_key_uuid).await?;

    info!("ServerState initialized.");

    // initialize scheduled jobs manager
    task_init(state.clone()).await?;

    tokio::spawn(redirect_http_to_https(Ports {
        http: 80,
        https: host_port,
    }));

    info!("Listening to Port {}...", host_port);

    info!(
        "Initialization complete in {:?}. Starting server now...",
        start.elapsed()
    );

    axum_server::bind_rustls(host_socket_addr, config)
        .serve(build_router(state).into_make_service_with_connect_info::<SocketAddr>())
        .await
        .map_err(|e| anyhow::anyhow!("Server error: {}", e))?;

    Ok(())
}

#[derive(Clone, Copy)]
struct Ports {
    http: u16,
    https: u16,
}

async fn redirect_http_to_https(ports: Ports) -> anyhow::Result<()> {
    fn make_https(host: &str, uri: Uri, https_port: u16) -> anyhow::Result<Uri> {
        let mut parts = uri.into_parts();

        parts.scheme = Some(axum::http::uri::Scheme::HTTPS);

        if parts.path_and_query.is_none() {
            parts.path_and_query = Some(
                "/".parse()
                    .map_err(|e| anyhow::anyhow!("Failed to parse '/' as path: {}", e))?,
            );
        }

        let authority: Authority = host
            .parse()
            .map_err(|e| anyhow::anyhow!("Failed to parse host into Authority: {}", e))?;
        let bare_host = match authority.port() {
            Some(port_struct) => authority
                .as_str()
                .strip_suffix(port_struct.as_str())
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "Failed to remove port ({}) from authority string",
                        port_struct
                    )
                })?
                .strip_suffix(':')
                .ok_or_else(|| anyhow::anyhow!("Failed to remove colon from authority string"))?,
            None => authority.as_str(),
        };

        parts.authority = Some(
            format!("{bare_host}:{https_port}")
                .parse()
                .map_err(|e| anyhow::anyhow!("Failed to parse new authority: {}", e))?,
        );

        Uri::from_parts(parts).map_err(|e| anyhow::anyhow!("Failed to construct HTTPS URI: {}", e))
    }

    let redirect = move |Host(host): Host, uri: Uri| async move {
        match make_https(&host, uri, ports.https) {
            Ok(uri) => Ok(Redirect::permanent(&uri.to_string())),
            Err(error) => {
                tracing::warn!(%error, "failed to convert URI to HTTPS");
                Err(StatusCode::BAD_REQUEST)
            }
        }
    };

    let addr = SocketAddr::from(([127, 0, 0, 1], ports.http));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to bind TCP listener: {}", e))?;
    tracing::debug!(
        "listening on {}",
        listener
            .local_addr()
            .map_err(|e| anyhow::anyhow!("Failed to get local address: {}", e))?
    );
    axum::serve(listener, redirect.into_make_service())
        .await
        .map_err(|e| anyhow::anyhow!("Failed to serve redirection: {}", e))
}
