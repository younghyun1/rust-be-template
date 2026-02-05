use std::sync::atomic::AtomicU64;

use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::bb8::Pool;
use lettre::{AsyncSmtpTransport, Tokio1Executor};
use tokio::sync::RwLock;
use tracing::info;
use uuid::Uuid;

use crate::domain::country::{CountryAndSubdivisionsTable, IsoCurrencyTable, IsoLanguageTable};
use crate::domain::i18n::i18n_cache::I18nCache;
use crate::init::load_cache::fastfetch_cache::FastFetchCache;
use crate::init::load_cache::system_info::SystemInfoState;
use crate::init::search::PostSearchIndex;
use crate::util::geographic::ip_info_lookup::decompress_and_deserialize;

use super::deployment_environment::DeploymentEnvironment;
use super::server_state::ServerState;

#[derive(Default)]
pub struct ServerStateBuilder {
    app_name_version: Option<String>,
    server_start_time: Option<tokio::time::Instant>,
    pool: Option<Pool<AsyncPgConnection>>,
    email_client: Option<AsyncSmtpTransport<Tokio1Executor>>, // regexes: [regex::Regex; 1],
}

impl ServerStateBuilder {
    pub fn app_name_version(mut self, app_name_version: String) -> Self {
        self.app_name_version = Some(app_name_version);
        self
    }

    pub fn server_start_time(mut self, server_start_time: tokio::time::Instant) -> Self {
        self.server_start_time = Some(server_start_time);
        self
    }

    pub fn pool(mut self, pool: Pool<AsyncPgConnection>) -> Self {
        self.pool = Some(pool);
        self
    }

    pub fn email_client(mut self, email_client: AsyncSmtpTransport<Tokio1Executor>) -> Self {
        self.email_client = Some(email_client);
        self
    }

    pub async fn build(self) -> anyhow::Result<ServerState> {
        let aws_profile_picture_config = {
            use aws_config::BehaviorVersion;
            use aws_config::meta::region::RegionProviderChain;

            let aws_key = std::env::var("AWS_IMAGE_UPLOAD_KEY")
                .map_err(|_| anyhow::anyhow!("AWS_IMAGE_UPLOAD_KEY not set"))?;
            let aws_secret = std::env::var("AWS_IMAGE_UPLOAD_SECRET_KEY")
                .map_err(|_| anyhow::anyhow!("AWS_IMAGE_UPLOAD_SECRET_KEY not set"))?;
            let credentials = aws_sdk_s3::config::Credentials::new(
                aws_key,
                aws_secret,
                None,                     // token
                None,                     // expiration
                "cyhdev-profile-picture", // provider name
            );
            // Use default region chain or fallback if not set.
            let region_provider = RegionProviderChain::default_provider().or_else("us-west-1");
            aws_config::defaults(BehaviorVersion::latest())
                .region(region_provider)
                .credentials_provider(credentials)
                .load()
                .await
        };

        let fastfetch_cache = FastFetchCache::init().await;

        Ok(ServerState {
            app_name_version: self
                .app_name_version
                .ok_or_else(|| anyhow::anyhow!("app_name_version is required"))?,
            server_start_time: self
                .server_start_time
                .ok_or_else(|| anyhow::anyhow!("server_start_time is required"))?,
            pool: self
                .pool
                .ok_or_else(|| anyhow::anyhow!("pool is required"))?,
            responses_handled: AtomicU64::new(0u64),
            email_client: self
                .email_client
                .ok_or_else(|| anyhow::anyhow!("email_client is required"))?,
            // regexes: [get_email_regex()],
            session_map: scc::HashMap::new(),
            blog_posts_cache: scc::HashMap::new(),
            search_index: {
                // Use disk-persisted index, configurable via env var
                let index_path = std::env::var("SEARCH_INDEX_PATH")
                    .unwrap_or_else(|_| "./data/search_index".to_string());
                let index = PostSearchIndex::open_or_create(&index_path)?;
                info!(path = %index_path, "Search index initialized");
                index
            },
            geo_ip_db: {
                let (dbs, dur) = decompress_and_deserialize()?;
                info!(elapsed=%format!("{dur:?}"), "Geo-IP database loaded and interned.");
                dbs
            },
            api_keys_set: scc::HashSet::<Uuid>::new(),
            country_map: RwLock::new(CountryAndSubdivisionsTable::new_empty()),
            languages_map: RwLock::new(IsoLanguageTable::new_empty()),
            currency_map: RwLock::new(IsoCurrencyTable::new_empty()),
            i18n_cache: RwLock::new(I18nCache::new()),
            deployment_environment: match std::env::var("CURR_ENV").as_deref() {
                Ok(s) => match s.to_ascii_lowercase().as_str() {
                    // Local
                    "local" | "localhost" => DeploymentEnvironment::Local,
                    // Dev
                    "dev" | "develop" | "development" => DeploymentEnvironment::Dev,
                    // Staging
                    "staging" | "stage" | "stg" => DeploymentEnvironment::Staging,
                    // Prod
                    "prd" | "prod" | "production" => DeploymentEnvironment::Prod,
                    // Default fallback: push _ to Local
                    _ => DeploymentEnvironment::Local,
                },
                Err(_) => DeploymentEnvironment::Prod,
            },
            request_client: reqwest::Client::builder()
                .user_agent("cyhdev.com")
                .build()?,
            visitor_board_map: scc::HashMap::new(),
            system_info_state: SystemInfoState::new(),
            aws_profile_picture_config,
            fastfetch: fastfetch_cache,
            wasm_module_cache: scc::HashMap::new(),
        })
    }
}
