use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;
use std::sync::Arc;

use chrono::Utc;
use tracing::{error, info};
use zstd::stream::copy_encode;

use crate::LOGS_DIR;
use crate::init::state::ServerState;
use crate::util::time::now::tokio_now;

const EXCLUDED_EXTENSIONS: [&str; 2] = ["gz", "zst"];

pub async fn compress_old_logs(_state: Arc<ServerState>) {
    let now = Utc::now();
    let now_yyyy_mm_dd = now.format("%Y-%m-%d").to_string();

    let logs_dir_path = Path::new(LOGS_DIR);

    let mut filewalker = walkdir::WalkDir::new(logs_dir_path);
    filewalker = filewalker.max_depth(4);

    for entry in filewalker {
        let entry_start = tokio_now();
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                error!(logs_dir = LOGS_DIR, error = %err, "Error walking logs directory");
                continue;
            }
        };

        let path = entry.path();
        if path.is_file() {
            let file_name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n,
                None => continue,
            };

            // Check if extension is excluded
            if let Some(ext) = path.extension().and_then(|e| e.to_str())
                && EXCLUDED_EXTENSIONS.contains(&ext)
            {
                continue;
            }

            // Exclude today's log file (by checking filename for today's date)
            if file_name.contains(&now_yyyy_mm_dd) {
                continue;
            }

            let compressed_path = path.with_extension(
                path.extension()
                    .and_then(|e| e.to_str())
                    .map(|e| format!("{e}.zst"))
                    .unwrap_or_else(|| "zst".to_string()),
            );

            // spawn_blocking needs owned paths (CLAUDE.md: blocking work off the runtime threads)
            let owned_path = path.to_path_buf();
            let owned_compressed_path = compressed_path.clone();

            let join_result = tokio::task::spawn_blocking(move || -> std::io::Result<()> {
                let input = File::open(&owned_path)?;
                let output = File::create(&owned_compressed_path)?;

                let mut reader = BufReader::new(input);
                let mut writer = BufWriter::new(output);

                copy_encode(&mut reader, &mut writer, 11)?;

                std::fs::remove_file(&owned_path)?;

                Ok(())
            })
            .await;

            match join_result {
                Ok(Ok(())) => {
                    let duration = entry_start.elapsed();
                    info!(
                        log_file_path = %entry.path().display(),
                        duration = ?duration,
                        "Log file compressed"
                    );
                }
                Ok(Err(err)) => {
                    error!(log_file_path = %path.display(), error = %err, "Failed to compress log file");
                }
                Err(join_err) => {
                    error!(log_file_path = %path.display(), error = %join_err, "Compression task panicked or was cancelled");
                }
            }
        }
    }
}
