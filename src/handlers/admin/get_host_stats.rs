use axum::{
    body::Bytes,
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::Response,
};
use sysinfo::System;
use tokio::time::{self, Duration};
use tracing::{error, info};

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

// run on spawn_blocking; heavily blocking function fsr
pub fn get_host_stats() -> HostStats {
    let mut system = System::new();
    system.refresh_cpu_usage();
    system.refresh_memory();

    let cpu_list = system.cpus();
    let cpu_total: f32 = cpu_list.iter().map(|cpu| cpu.cpu_usage()).sum();
    let cpu_usage: f32 = if !cpu_list.is_empty() {
        cpu_total / cpu_list.len() as f32
    } else {
        0.0
    };
    let mem_total = system.total_memory(); // KiB
    let mem_free = system.free_memory(); // KiB

    HostStats {
        cpu_usage,
        mem_total,
        mem_free,
    }
}

pub async fn ws_host_stats_handler(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(handle_host_stats_socket)
}

async fn handle_host_stats_socket(mut socket: WebSocket) {
    let mut interval = time::interval(Duration::from_secs(1));
    loop {
        interval.tick().await;

        let host_stats_result = tokio::task::spawn_blocking(|| get_host_stats()).await;
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
