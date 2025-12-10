use std::collections::VecDeque;

use tokio::sync::RwLock;

use crate::util::system::get_cpu_usage::get_cpu_usage;
use crate::util::system::get_memory_size::get_memory_size;
use crate::util::system::get_memory_usage::get_memory_usage;

pub struct SystemInfoState {
    pub history: RwLock<VecDeque<SystemInfo>>,
    pub max_len: usize,
    pub ram_total_size: u64,
}

impl Default for SystemInfoState {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemInfoState {
    pub fn new() -> Self {
        SystemInfoState {
            history: RwLock::new(VecDeque::with_capacity(3600)),
            max_len: 3600,
            ram_total_size: get_memory_size(),
        }
    }

    pub async fn push(&self, info: SystemInfo) {
        let mut history = self.history.write().await;
        if history.len() == self.max_len {
            history.pop_front();
        }
        history.push_back(info);
    }

    pub async fn len(&self) -> usize {
        let history = self.history.read().await;
        history.len()
    }

    pub async fn is_empty(&self) -> bool {
        let history = self.history.read().await;
        history.is_empty()
    }

    pub fn get_total_memory(&self) -> u64 {
        self.ram_total_size
    }

    pub async fn get_cpu_usage(&self) -> f64 {
        let history = self.history.read().await;
        history.back().map(|info| info.cpu_usage).unwrap_or(0.0)
    }

    pub async fn update(&self) {
        let info = SystemInfo {
            cpu_usage: get_cpu_usage().await,
            memory_usage: get_memory_usage(),
        };
        self.push(info).await;
    }

    pub async fn get_memory_usage(&self) -> u64 {
        let history = self.history.read().await;
        history.back().map(|info| info.memory_usage).unwrap_or(0)
    }
}

pub struct SystemInfo {
    pub cpu_usage: f64,
    pub memory_usage: u64, // bytes
}
