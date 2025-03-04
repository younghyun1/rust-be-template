use std::collections::BTreeMap;
use std::net::Ipv4Addr;
use std::sync::atomic::AtomicU64;

use chrono::Utc;
use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::bb8::{Pool, PooledConnection};
use lettre::{AsyncSmtpTransport, Tokio1Executor};
use tokio::sync::RwLock;
use tracing::{error, info};
use uuid::Uuid;

use crate::domain::blog::PostInfo;
use crate::util::geographic::ip_info_lookup::{
    IpEntry, IpInfo, decompress_and_deserialize, lookup_ip_location_from_map,
};
use crate::util::time::now::tokio_now;

use super::load_cache::post_info::load_post_info;

// use super::compile_regex::get_email_regex;

const DEFAULT_SESSION_DURATION: chrono::Duration = chrono::Duration::hours(1);

pub struct ServerState {
    app_name_version: String,
    server_start_time: tokio::time::Instant,
    pool: Pool<AsyncPgConnection>,
    responses_handled: AtomicU64,
    email_client: lettre::AsyncSmtpTransport<Tokio1Executor>,
    // regexes: [regex::Regex; 1],
    session_map: scc::HashMap<uuid::Uuid, Session>,
    blog_posts_cache: RwLock<Vec<PostInfo>>,
    geo_ip_db: BTreeMap<u32, IpEntry>,
}

impl ServerState {
    pub async fn new_session(
        &self,
        user_id: Uuid,
        valid_for: Option<chrono::Duration>,
    ) -> anyhow::Result<Uuid> {
        let session_id = Uuid::new_v4();
        let now = chrono::Utc::now();
        let expires_at = now + valid_for.unwrap_or(DEFAULT_SESSION_DURATION);
        match self
            .session_map
            .insert_async(
                session_id,
                Session {
                    session_id,
                    user_id,
                    created_at: now,
                    expires_at,
                },
            )
            .await
        {
            Ok(_) => (),
            Err(_) => {
                return Err(anyhow::anyhow!(
                    "Failed to insert session into scc::HashMap; key already exists!"
                ));
            }
        };

        Ok(session_id)
    }

    pub async fn get_session(&self, session_id: &Uuid) -> anyhow::Result<Session> {
        match self
            .session_map
            .read_async(session_id, |_, v| v.clone())
            .await
        {
            Some(session) => Ok(session),
            None => Err(anyhow::anyhow!("Session not found")),
        }
    }

    pub fn get_session_length(&self) -> usize {
        self.session_map.len()
    }

    pub async fn remove_session(&self, session_id: Uuid) -> anyhow::Result<(Uuid, usize)> {
        let cur_session_count = self.session_map.len();
        match self.session_map.remove_async(&session_id).await {
            Some((session_id, _)) => Ok((session_id, cur_session_count - 1)),
            None => Err(anyhow::anyhow!("Session map out of sync!")),
        }
    }

    pub async fn purge_expired_sessions(&self) -> (usize, usize) {
        let now = chrono::Utc::now();
        let (mut pruned, mut remaining): (usize, usize) = (0usize, 0usize);

        self.session_map
            .prune_async(|_, session| {
                if session.expires_at < now {
                    pruned += 1;
                    None
                } else {
                    remaining += 1;
                    Some(session)
                }
            })
            .await;

        (pruned, remaining)
    }

    pub fn builder() -> ServerStateBuilder {
        ServerStateBuilder::default()
    }

    pub fn get_app_name_version(&self) -> String {
        self.app_name_version.clone()
    }

    pub fn get_uptime(&self) -> tokio::time::Duration {
        self.server_start_time.elapsed()
    }

    pub async fn get_conn(&self) -> anyhow::Result<PooledConnection<AsyncPgConnection>> {
        Ok(self.pool.get().await?)
    }

    pub fn get_email_client(&self) -> &AsyncSmtpTransport<Tokio1Executor> {
        &self.email_client
    }

    pub fn get_responses_handled(&self) -> u64 {
        self.responses_handled
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn add_responses_handled(&self) {
        self.responses_handled
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    pub async fn synchronize_post_info_cache(&self) {
        let start = tokio_now();

        let post_info = match load_post_info(self).await {
            Ok(post_info) => post_info,
            Err(e) => {
                error!("Could not synchronize post metadata cache: {:?}", e);
                return;
            }
        };

        let mut cache_write_lock = self.blog_posts_cache.write().await;
        *cache_write_lock = post_info;

        let elapsed = start.elapsed();
        info!(number_of_rows = %cache_write_lock.len(), elapsed=%format!("{elapsed:?}"), "Post metadata cache synchronized.");
        drop(cache_write_lock);
    }

    pub async fn get_posts_from_cache(
        &self,
        page: usize,
        page_size: usize,
    ) -> (Vec<PostInfo>, usize) {
        let start_index = (page.saturating_sub(1)) * page_size;
        let end_index = start_index + page_size;

        let cache_read_lock = self.blog_posts_cache.read().await;
        let total_posts = cache_read_lock.len();
        let total_pages = total_posts.div_ceil(page_size); // Calculate total number of pages

        if start_index >= total_posts {
            return (vec![], total_pages);
        }

        let posts = cache_read_lock[start_index..end_index.min(total_posts)].to_vec();
        (posts, total_pages)
    }

    // TODO: Insert post (write-through cache)
    pub async fn insert_post_to_cache(&self, post: &PostInfo) {
        let mut cache_write_lock = self.blog_posts_cache.write().await;
        cache_write_lock.push(post.to_owned());
    }

    pub fn lookup_ip_location(&self, ip: Ipv4Addr) -> Option<IpInfo> {
        lookup_ip_location_from_map(&self.geo_ip_db, ip)
    }
}

#[derive(Default)]
pub struct ServerStateBuilder {
    app_name_version: Option<String>,
    server_start_time: Option<tokio::time::Instant>,
    pool: Option<Pool<AsyncPgConnection>>,
    email_client: Option<lettre::AsyncSmtpTransport<Tokio1Executor>>, // regexes: [regex::Regex; 1],
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

    pub fn email_client(
        mut self,
        email_client: lettre::AsyncSmtpTransport<Tokio1Executor>,
    ) -> Self {
        self.email_client = Some(email_client);
        self
    }

    pub fn build(self) -> anyhow::Result<ServerState> {
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
            blog_posts_cache: tokio::sync::RwLock::new(vec![]),
            geo_ip_db: {
                let (map, dur) = decompress_and_deserialize()?;
                info!(elapsed=%format!("{dur:?}"), "Geo-IP database mapped to BTreeMap.");
                map
            },
        })
    }
}

#[derive(Debug, Clone, serde_derive::Serialize, serde_derive::Deserialize)]
pub struct Session {
    session_id: uuid::Uuid,
    user_id: uuid::Uuid,
    created_at: chrono::DateTime<chrono::Utc>,
    expires_at: chrono::DateTime<chrono::Utc>,
}

impl Session {
    pub fn is_valid(&self) -> bool {
        let now = Utc::now();

        self.created_at < now && self.expires_at > now
    }

    pub fn get_user_id(&self) -> uuid::Uuid {
        self.user_id
    }
}
