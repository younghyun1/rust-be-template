use init::server_init::server_init_proc;
use mimalloc::MiMalloc;
use tracing::{info, level_filters};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

pub mod build_info;
pub mod domain;
pub mod dto;
pub mod errors;
pub mod handlers;
pub mod init;
pub mod jobs;
pub mod routers;
pub mod schema;
pub mod util;

pub const DOMAIN_NAME: &str = "cyhdev.com";
pub const LOGS_DIR: &'static str = "./logs/";

// main function
#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    let start = tokio::time::Instant::now();
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    if std::env::var("IS_AWS_ECS").is_err() {
        dotenvy::dotenv().map_err(|e| anyhow::anyhow!("Failed to load .env: {}", e))?;
    }

    // TODO: replace with a more refined build.rs constant
    let app_name_version = std::env::var("APP_NAME_VERSION")
        .map_err(|e| anyhow::anyhow!("Failed to get APP_NAME_VERSION: {}", e))?;

    let filename = app_name_version.to_string();

    let file_appender =
        tracing_appender::rolling::daily(format!("{LOGS_DIR}{app_name_version}"), filename);
    let (_non_blocking_file, _guard) = tracing_appender::non_blocking(file_appender);

    let console_layer = tracing_subscriber::fmt::layer()
        // .json()
        .with_ansi(true)
        .with_target(true)
        .pretty()
        .with_filter(level_filters::LevelFilter::INFO);

    let file_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .json()
        .with_writer(_non_blocking_file)
        .with_filter(level_filters::LevelFilter::DEBUG);

    // Build a subscriber that combines both layers
    tracing_subscriber::registry()
        .with(console_layer)
        .with(file_layer)
        .init();

    info!("Initializing server...");

    // Apparently, when you listen in from Tokio's main thread, that slows down performance due to delegation overhead as the main thread is reserved...
    let server_handle = tokio::spawn(async move { server_init_proc(start).await });

    match server_handle.await {
        Ok(Ok(_)) => {}
        Ok(Err(e)) => {
            tracing::error!(error = ?e, "Server initialization failed (inner error)");
            return Err(e);
        }
        Err(e) => {
            tracing::error!(error = ?e, "Server task join failed");
            return Err(anyhow::anyhow!("Server task join error: {e}"));
        }
    }

    Ok(())
}
