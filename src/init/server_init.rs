use std::{net::SocketAddr, sync::Arc, time::Duration};

use diesel_async::pooled_connection::bb8::Pool;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use tracing::info;

use crate::routers::main_router::build_router;

use super::{config::DbConfig, state::ServerState};

pub async fn server_init_proc(start: tokio::time::Instant) -> anyhow::Result<()> {
    let num_cores: u32 = num_cpus::get_physical() as u32;

    if std::env::var("IS_AWS").is_err() {
        dotenvy::dotenv().map_err(|e| anyhow::anyhow!("Failed to load .env: {}", e))?;
    }

    let db_config = DbConfig::from_env()
        .map_err(|e| anyhow::anyhow!("Failed to get DB config from environment: {}", e))?
        .to_url()
        .map_err(|e| anyhow::anyhow!("Failed to convert DB config to URL: {}", e))?;

    let pool_config =
        AsyncDieselConnectionManager::<diesel_async::AsyncPgConnection>::new(db_config);

    let pool = Pool::builder()
        .min_idle(Some(num_cores))
        .max_size(num_cores * 10u32)
        .connection_timeout(Duration::from_secs(2))
        .build(pool_config)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to build connection pool: {}", e))?;

    let app_name_version: String = std::env::var("APP_NAME_VERSION")
        .map_err(|e| anyhow::anyhow!("Failed to load APP_NAME_VAR from .env: {}", e))?;

    let state = Arc::new(
        ServerState::builder()
            .app_name_version(app_name_version)
            .pool(pool)
            .server_start_time(start)
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build ServerState: {}", e))?,
    );

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .map_err(|e| anyhow::anyhow!("Failed to bind TCP listener: {}", e))?;

    info!("Backend server starting...");

    axum::serve(
        listener,
        build_router(state).into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .map_err(|e| anyhow::anyhow!("Failed to serve application: {}", e))?;

    Ok(())
}
