use diesel_async::pooled_connection::bb8::{Pool, PooledConnection};
use diesel_async::AsyncPgConnection;

pub struct ServerState {
    server_start_time: tokio::time::Instant,
    pool: Pool<AsyncPgConnection>,
}

impl ServerState {
    pub fn builder() -> ServerStateBuilder {
        ServerStateBuilder::default()
    }

    pub async fn get_conn(&self) -> anyhow::Result<PooledConnection<AsyncPgConnection>> {
        Ok(self.pool.get().await?)
    }

    pub fn get_uptime(&self) -> tokio::time::Duration {
        self.server_start_time.elapsed()
    }
}

#[derive(Default)]
pub struct ServerStateBuilder {
    server_start_time: Option<tokio::time::Instant>,
    pool: Option<Pool<AsyncPgConnection>>,
}

impl ServerStateBuilder {
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
            server_start_time: self
                .server_start_time
                .ok_or_else(|| anyhow::anyhow!("server_start_time is required"))?,
            pool: self
                .pool
                .ok_or_else(|| anyhow::anyhow!("pool is required"))?,
        })
    }
}
