use std::sync::Arc;

use diesel_async::{pooled_connection::AsyncDieselConnectionManager, AsyncPgConnection};
use tracing::info;

use crate::routes::main_router::build_router;

use super::{config::DbConfig, state::ServerState};

type Pool = bb8::Pool<AsyncDieselConnectionManager<AsyncPgConnection>>;

pub async fn server_init_proc(start: tokio::time::Instant) -> anyhow::Result<()> {
    let num_cores: u32 = num_cpus::get_physical() as u32;

    tracing_subscriber::fmt().init();
    dotenvy::dotenv()?;

    let db_config = DbConfig::from_env()?.to_url()?;
    let pool_config =
        AsyncDieselConnectionManager::<diesel_async::AsyncPgConnection>::new(db_config);

    let pool: Pool = bb8::Pool::builder()
        .min_idle(Some(num_cores))
        .max_size(num_cores * 10u32)
        .queue_strategy(bb8::QueueStrategy::Fifo)
        .build(pool_config)
        .await?;

    let state = Arc::new(
        ServerState::builder()
            .pool(pool)
            .server_start_time(start)
            .build()?,
    );

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;

    info!("Backend server starting...");
    axum::serve(listener, build_router(state)).await?;
    Ok(())
}
