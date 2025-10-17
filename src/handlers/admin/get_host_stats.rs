use std::sync::Arc;

use axum::{
    body::Bytes,
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::Response,
};
use tokio::time::{self, Duration};
use tracing::info;

use crate::init::state::ServerState;

pub struct HostStats {
    pub cpu_usage: f32,
    pub mem_total: u64,
    pub mem_free: u64,
}

impl HostStats {
    fn into_bits(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(24);
        buf.extend_from_slice(&self.cpu_usage.to_be_bytes());
        buf.extend_from_slice(&self.mem_total.to_be_bytes());
        buf.extend_from_slice(&self.mem_free.to_be_bytes());
        buf
    }
}

// TODO: system information querying is unusually heavyweight for some reason. switch to cat proc or something lol
// run on spawn_blocking; heavily blocking function fsr
pub async fn get_host_stats(state: Arc<ServerState>) -> HostStats {
    let mem_total = state.system_info_state.get_total_memory();
    let mem_usage = state.system_info_state.get_memory_usage().await;
    let cpu_usage = state.system_info_state.get_cpu_usage().await;
    HostStats {
        cpu_usage: cpu_usage as f32,
        mem_total,
        mem_free: mem_total - mem_usage,
    }
}

pub async fn ws_host_stats_handler(
    State(state): State<Arc<ServerState>>,
    ws: WebSocketUpgrade,
) -> Response {
    ws.on_upgrade(move |socket| handle_host_stats_socket(socket, state.clone()))
}

async fn handle_host_stats_socket(mut socket: WebSocket, state: Arc<ServerState>) {
    let mut interval = time::interval(Duration::from_millis(1000));
    loop {
        interval.tick().await;

        let host_stats = get_host_stats(state.clone()).await;

        // Fix: Bytes::from for Message::Binary
        if let Err(e) = socket
            .send(Message::Binary(Bytes::from(host_stats.into_bits())))
            .await
        {
            info!(error = ?e, "WebSocket disconnected");
            return;
        }
    }
}
