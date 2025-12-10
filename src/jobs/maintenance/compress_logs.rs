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

const EXCLUDED_EXTENSIONS: [&'static str; 2] = ["gz", "zst"];

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
                error!("Error walking logs directory {}: {}", LOGS_DIR, err);
                continue;
            }
        };

        let path = entry.path();
        if path.is_file() {
            let extension = path.extension().and_then(|ext| ext.to_str());
            if let Some(ext) = extension {
                if EXCLUDED_EXTENSIONS.contains(&ext) || path.ends_with(&now_yyyy_mm_dd) {
                    continue;
                }
            }

            let compressed_path = match path.with_extension(
                path.extension()
                    .and_then(|e| e.to_str())
                    .map(|e| format!("{e}.zst"))
                    .unwrap_or_else(|| "zst".to_string()),
            ) {
                p => p,
            };

            if let Err(err) = (|| -> Result<(), Box<dyn std::error::Error>> {
                let input = File::open(path)?;
                let output = File::create(&compressed_path)?;

                let mut reader = BufReader::new(input);
                let mut writer = BufWriter::new(output);

                copy_encode(&mut reader, &mut writer, 11)?;

                std::fs::remove_file(path)?;

                Ok(())
            })() {
                error!("Failed to compress log file {:?}: {}", path, err);
            }
        }

        let duration = entry_start.elapsed();
        info!(
            log_file_path = %entry.path().display(),
            duration = ?duration,
            "Log file compressed"
        );
    }
}
