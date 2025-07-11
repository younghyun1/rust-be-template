use std::sync::Arc;

use axum::{
    body::Bytes,
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::Response,
};
use sysinfo::System;
use tokio::time::{self, Duration};
use tracing::{error, info};

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
pub fn get_host_stats() -> HostStats {
    let mut system = System::new_all();
    std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
    system.refresh_cpu_usage();
    system.refresh_memory();

    let cpu_list = system.cpus();
    let cpu_total: f32 = cpu_list.iter().map(|cpu| cpu.cpu_usage()).sum();
    let cpu_usage: f32 = if !cpu_list.is_empty() {
        cpu_total / cpu_list.len() as f32
    } else {
        0.0
    };
    let mem_total = system.total_memory();
    let mem_free = system.free_memory();

    HostStats {
        cpu_usage,
        mem_total,
        mem_free,
    }
}

pub async fn ws_host_stats_handler(
    State(state): State<Arc<ServerState>>,
    ws: WebSocketUpgrade,
) -> Response {
    ws.on_upgrade(move |socket| handle_host_stats_socket(socket, state.clone()))
}

async fn handle_host_stats_socket(mut socket: WebSocket, _state: Arc<ServerState>) {
    let mut interval = time::interval(Duration::from_millis(1000));
    loop {
        interval.tick().await;

        let host_stats_result = tokio::task::spawn_blocking(get_host_stats).await;
        let host_stats = match host_stats_result {
            Ok(stats) => stats,
            Err(e) => {
                error!(error = ?e, "Failed to spawn_blocking get_host_stats");
                return;
            }
        };

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
