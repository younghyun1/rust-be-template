use ansi_to_html::convert;
use chrono::{DateTime, Utc};
use tokio::{process::Command, sync::RwLock};
use tracing::error;

use crate::errors::code_error::CodeError;

pub struct FastFetchCache {
    fastfetch_string: RwLock<String>,
    last_fetched: RwLock<DateTime<Utc>>,
}

impl FastFetchCache {
    pub async fn init() -> Self {
        let cache = FastFetchCache {
            fastfetch_string: RwLock::new(String::new()),
            last_fetched: RwLock::new(Utc::now()),
        };

        if let Err(_e) = cache.update_fastfetch_string().await {}

        cache
    }

    pub async fn get_last_fetched_time(&self) -> DateTime<Utc> {
        let guard = self.last_fetched.read().await;
        *guard
    }

    pub async fn get_fastfetch_string(&self) -> String {
        let guard = self.fastfetch_string.read().await;
        guard.clone()
    }

    pub async fn update_fastfetch_string(&self) -> Result<(), CodeError> {
        // Run the 'fastfetch' command asynchronously
        let output = match Command::new("fastfetch")
            .env("TERM", "xterm-256color")
            .output()
            .await
        {
            Ok(output) => output,
            Err(e) => {
                error!("Failed to run fastfetch: {}", e);
                return Err(CodeError::COULD_NOT_RUN_FASTFETCH);
            }
        };

        // Convert stdout to a String, assuming UTF-8/ANSI output
        let ansi_output = String::from_utf8_lossy(&output.stdout).to_string();
        let html_output = convert(&ansi_output).map_err(|e| {
            error!("Failed to convert fastfetch output to HTML: {}", e);
            CodeError::COULD_NOT_RUN_FASTFETCH
        })?;

        {
            let mut fastfetch_guard = self.fastfetch_string.write().await;
            *fastfetch_guard = html_output;
        }

        {
            let mut last_fetched_guard = self.last_fetched.write().await;
            *last_fetched_guard = Utc::now();
        }

        Ok(())
    }
}
