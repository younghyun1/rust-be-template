use std::collections::HashMap as StdHashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;

use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::bb8::Pool;
use lettre::{AsyncSmtpTransport, Tokio1Executor};
use scc::HashSet;
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

use crate::domain::blog::blog::CachedPostInfo;
use crate::domain::country::{CountryAndSubdivisionsTable, IsoCurrencyTable, IsoLanguageTable};
use crate::domain::i18n::i18n_cache::I18nCache;
use crate::domain::live_chat::cache::LiveChatCache;
use crate::init::load_cache::fastfetch_cache::FastFetchCache;
use crate::init::load_cache::system_info::SystemInfoState;
use crate::init::search::PostSearchIndex;
use crate::util::geographic::ip_info_lookup::GeoIpDatabases;

use super::deployment_environment::DeploymentEnvironment;
use super::session::Session;

mod core;
mod geo;
mod i18n;
mod live_chat;
mod posts;
mod sessions;
mod visitors;
mod wasm;

pub struct ServerState {
    pub(crate) app_name_version: String,
    pub(crate) server_start_time: tokio::time::Instant,
    pub(crate) pool: Pool<AsyncPgConnection>,
    pub(crate) responses_handled: AtomicU64,
    pub(crate) email_client: AsyncSmtpTransport<Tokio1Executor>,
    pub(crate) session_map: scc::HashMap<uuid::Uuid, Session>,
    pub(crate) blog_posts_cache: scc::HashMap<uuid::Uuid, CachedPostInfo>,
    pub(crate) blog_post_slug_cache: scc::HashMap<String, uuid::Uuid>,
    pub(crate) blog_post_order_cache: RwLock<Vec<uuid::Uuid>>,
    pub(crate) search_index: PostSearchIndex,
    pub(crate) geo_ip_db: GeoIpDatabases,
    pub visitor_board_map: scc::HashMap<([u8; 8], [u8; 8]), u64>,
    pub(crate) visitor_log_buffer: Mutex<StdHashMap<VisitorLogKey, VisitorLogBatch>>,
    pub(crate) api_keys_set: HashSet<Uuid>,
    pub country_map: RwLock<CountryAndSubdivisionsTable>,
    pub languages_map: RwLock<IsoLanguageTable>,
    pub currency_map: RwLock<IsoCurrencyTable>,
    pub i18n_cache: RwLock<I18nCache>,
    pub(crate) deployment_environment: DeploymentEnvironment,
    pub(crate) request_client: reqwest::Client,
    pub system_info_state: SystemInfoState,
    pub aws_profile_picture_config: aws_config::SdkConfig,
    pub fastfetch: FastFetchCache,
    pub wasm_module_cache: scc::HashMap<Uuid, (Arc<[u8]>, bool, &'static str)>,
    pub live_chat_cache: LiveChatCache,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct VisitorLogKey {
    pub(crate) latitude_bytes: [u8; 8],
    pub(crate) longitude_bytes: [u8; 8],
    pub(crate) ip_address: IpAddr,
    pub(crate) city: String,
    pub(crate) country: String,
}

#[derive(Debug, Clone)]
pub(crate) struct VisitorLogBatch {
    pub(crate) count: u64,
    pub(crate) visited_at: chrono::DateTime<chrono::Utc>,
}
