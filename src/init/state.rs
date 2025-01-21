use std::sync::atomic::AtomicU64;

use diesel_async::pooled_connection::bb8::{Pool, PooledConnection};
use diesel_async::AsyncPgConnection;

// use super::compile_regex::get_email_regex;

pub struct ServerState {
    app_name_version: String,
    server_start_time: tokio::time::Instant,
    pool: Pool<AsyncPgConnection>,
    responses_handled: AtomicU64,
    // regexes: [regex::Regex; 1],
}

impl ServerState {
    pub fn builder() -> ServerStateBuilder {
        ServerStateBuilder::default()
    }

    pub fn get_app_name_version(&self) -> String {
        self.app_name_version.clone()
    }

    pub fn get_uptime(&self) -> tokio::time::Duration {
        self.server_start_time.elapsed()
    }

    pub async fn get_conn(&self) -> anyhow::Result<PooledConnection<AsyncPgConnection>> {
        Ok(self.pool.get().await?)
    }

    pub fn get_responses_handled(&self) -> u64 {
        self.responses_handled
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn add_responses_handled(&self) {
        self.responses_handled
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
}

#[derive(Default)]
pub struct ServerStateBuilder {
    app_name_version: Option<String>,
    server_start_time: Option<tokio::time::Instant>,
    pool: Option<Pool<AsyncPgConnection>>,
}

impl ServerStateBuilder {
    pub fn app_name_version(mut self, app_name_version: String) -> Self {
        self.app_name_version = Some(app_name_version);
        self
    }

    pub fn server_start_time(mut self, server_start_time: tokio::time::Instant) -> Self {
        self.server_start_time = Some(server_start_time);
        self
    }

    pub fn pool(mut self, pool: Pool<AsyncPgConnection>) -> Self {
        self.pool = Some(pool);
        self
    }

    pub fn build(self) -> anyhow::Result<ServerState> {
        Ok(ServerState {
            app_name_version: self
                .app_name_version
                .ok_or_else(|| anyhow::anyhow!("app_name_version is required"))?,
            server_start_time: self
                .server_start_time
                .ok_or_else(|| anyhow::anyhow!("server_start_time is required"))?,
            pool: self
                .pool
                .ok_or_else(|| anyhow::anyhow!("pool is required"))?,
            responses_handled: AtomicU64::new(0u64),
            // regexes: [get_email_regex()],
        })
    }
}
