use init::server_init::server_init_proc;
use tracing::{info, level_filters};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::Layer;

// modules tree
pub mod domain {
    pub mod user;
}
pub mod dto {
    pub mod common {}
    pub mod requests {
        pub mod user {
            pub mod check_if_user_exists_request;
            pub mod signup_request;
        }
    }
    pub mod responses {
        pub mod user {
            pub mod signup_response;
        }
        pub mod response_data;
        pub mod response_meta;
    }
}
pub mod errors {

    pub mod code_error;
}
pub mod handlers {
    pub mod user {
        pub mod check_if_user_exists;
        pub mod signup;
    }
    pub mod fallback;
    pub mod root;
}
pub mod routers {
    pub mod middleware {
        pub mod logging;
    }
    pub mod main_router;
}
pub mod init {
    pub mod compile_regex;
    pub mod config;
    pub mod server_init;
    pub mod state;
}
pub mod jobs {}
pub mod util {
    pub mod crypto {

        pub mod hash_pw;
    }
    pub mod duration_formatter;
    pub mod now;
}

// main function
#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    let start = tokio::time::Instant::now();

    if std::env::var("IS_AWS").is_err() {
        println!("env. variable IS_AWS missing, assuming .env exists...");
        dotenvy::dotenv().map_err(|e| anyhow::anyhow!("Failed to load .env: {}", e))?;
    }

    let app_name_version = std::env::var("APP_NAME_VERSION")
        .map_err(|e| anyhow::anyhow!("Failed to get APP_NAME_VERSION: {}", e))?;

    let filename = format!("{}", app_name_version);

    let file_appender =
        tracing_appender::rolling::daily(format!("./log/{}", app_name_version), filename);
    let (non_blocking_file, _guard) = tracing_appender::non_blocking(file_appender);

    // Create a console layer
    let console_layer = tracing_subscriber::fmt::layer()
        // .json()
        .with_ansi(true)
        .with_target(true)
        .with_filter(level_filters::LevelFilter::INFO);

    // Create a file layer
    let file_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .json()
        // .with_filter(level_filters::LevelFilter::INFO)
        .with_writer(non_blocking_file);

    // Build a subscriber that combines both layers
    tracing_subscriber::registry()
        .with(console_layer)
        .with(file_layer)
        .init();

    info!("Initializing server...");
    server_init_proc(start).await?;

    Ok(())
}
