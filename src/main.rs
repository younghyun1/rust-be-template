use init::server_init::server_init_proc;
use tracing::info;

// modules tree
pub mod domain {}
pub mod dto {
    pub mod common {}
    pub mod requests {}
    pub mod responses {
        pub mod response_meta;
    }
}
pub mod errors {

    pub mod code_error;
}
pub mod handlers {
    pub mod fallback;
    pub mod root;
}
pub mod routes {
    pub mod main_router;
}
pub mod init {
    pub mod config;
    pub mod server_init;
    pub mod state;
}
pub mod util {}

// main function
#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    let start = tokio::time::Instant::now();
    tracing_subscriber::fmt().init();

    info!("Initializing server...");
    server_init_proc(start).await?;

    Ok(())
}
