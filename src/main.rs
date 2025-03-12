use init::server_init::server_init_proc;
use mimalloc::MiMalloc;
use tracing::{info, level_filters};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

// modules tree
pub mod schema;
pub mod domain {
    pub mod blog;
    pub mod country;
    pub mod domain_traits;
    pub mod user;
}
pub mod dto {
    pub mod requests {
        pub mod auth {
            pub mod check_if_user_exists_request;
            pub mod login_request;
            pub mod reset_password;
            pub mod reset_password_request;
            pub mod signup_request;
            pub mod verify_user_email_request;
        }
        pub mod blog {
            pub mod get_posts_request;
            pub mod read_post;
            pub mod submit_post_request;
            pub mod upvote_comment_request;
            pub mod upvote_post_request;
        }
    }
    pub mod responses {
        pub mod auth {
            pub mod email_validate_response;
            pub mod login_response;
            pub mod logout_response;
            pub mod reset_password_request_response;
            pub mod reset_password_response;
            pub mod signup_response;
        }
        pub mod blog {

            pub mod get_posts;
            pub mod read_post_response;
            pub mod submit_post_response;
        }
        pub mod response_data;
        pub mod response_meta;
    }
}
pub mod errors {
    pub mod code_error;
}
pub mod handlers {
    pub mod auth {
        pub mod check_if_user_exists;
        pub mod login;
        pub mod logout;
        pub mod reset_password;
        pub mod reset_password_request;
        pub mod signup;
        pub mod verify_user_email;
    }
    pub mod blog {
        pub mod get_posts;
        pub mod read_post;
        pub mod rescind_comment_upvote;
        pub mod rescind_post_upvote;
        pub mod submit_post;
        pub mod upvote_comment;
        pub mod upvote_post;
    }
    pub mod server {
        pub mod fallback;
        pub mod healthcheck;
        pub mod lookup_ip_loc;
        pub mod root;
    }
    pub mod countries {

        pub mod get_countries;
        pub mod get_country;
        pub mod get_language;
        pub mod get_languages;
        pub mod get_subdivisions_for_country;
    }
}
pub mod routers {
    pub mod middleware {
        pub mod api_key;
        pub mod auth;
        pub mod logging;
    }
    pub mod main_router;
}
pub mod init {
    pub mod load_cache {
        pub mod post_info;
    }
    pub mod compile_regex;
    pub mod config;
    pub mod server_init;
    pub mod state;
}
pub mod jobs {
    pub mod auth {
        pub mod invalidate_sessions;
        pub mod purge_nonverified_users;
    }
    pub mod job_funcs {
        pub mod every_hour;
        pub mod every_minute;
        pub mod every_second;
        pub mod init_scheduler;
    }
}
pub mod util {
    pub mod email {
        pub mod emails;
    }
    pub mod string {
        pub mod generate_slug;
        pub mod validations;
    }
    pub mod crypto {
        pub mod hash_pw;
        pub mod random_pw;
        pub mod verify_pw;
    }
    pub mod time {
        pub mod duration_formatter;
        pub mod now;
    }
    pub mod geographic {
        pub mod ip_info_lookup;
    }
    pub mod image {
        pub mod process_uploaded_images;
    }
}

// main function
#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    let start = tokio::time::Instant::now();

    if std::env::var("IS_AWS").is_err() {
        dotenvy::dotenv().map_err(|e| anyhow::anyhow!("Failed to load .env: {}", e))?;
    }

    let app_name_version = std::env::var("APP_NAME_VERSION")
        .map_err(|e| anyhow::anyhow!("Failed to get APP_NAME_VERSION: {}", e))?;

    let filename = app_name_version.to_string();

    let file_appender =
        tracing_appender::rolling::daily(format!("./logs/{}", app_name_version), filename);
    let (_non_blocking_file, _guard) = tracing_appender::non_blocking(file_appender);

    let console_layer = tracing_subscriber::fmt::layer()
        // .json()
        .with_ansi(true)
        .with_target(true)
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

    server_handle.await??;

    Ok(())
}
