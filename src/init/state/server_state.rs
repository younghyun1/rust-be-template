use std::net::IpAddr;
use std::sync::atomic::AtomicU64;

use diesel::QueryDsl;
use diesel_async::pooled_connection::bb8::{Pool, PooledConnection};
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use lettre::{AsyncSmtpTransport, Tokio1Executor};
use scc::HashSet;
use tokio::sync::RwLock;
use tracing::{error, info};
use uuid::Uuid;

use crate::domain::auth::user::User;
use crate::domain::blog::blog::CachedPostInfo;
use crate::domain::country::{
    CountryAndSubdivisionsTable, IsoCountry, IsoCountrySubdivision, IsoCurrency, IsoCurrencyTable,
    IsoLanguage, IsoLanguageTable,
};
use crate::domain::i18n::i18n::InternationalizationString;
use crate::domain::i18n::i18n_cache::I18nCache;
use crate::init::load_cache::fastfetch_cache::FastFetchCache;
use crate::init::load_cache::post_info::load_post_info;
use crate::init::load_cache::system_info::SystemInfoState;
use crate::init::search::PostSearchIndex;
use crate::schema::{iso_country, iso_country_subdivision, iso_currency, iso_language};
use crate::util::geographic::ip_info_lookup::{
    GeoIpDatabases, IpInfo, lookup_ip_location_from_map,
};
use crate::util::time::now::tokio_now;

use super::builder::ServerStateBuilder;
use super::deployment_environment::DeploymentEnvironment;
use super::session::{DEFAULT_SESSION_DURATION, Session};

pub struct ServerState {
    pub(crate) app_name_version: String,
    pub(crate) server_start_time: tokio::time::Instant,
    pub(crate) pool: Pool<AsyncPgConnection>,
    pub(crate) responses_handled: AtomicU64,
    pub(crate) email_client: lettre::AsyncSmtpTransport<Tokio1Executor>,
    // regexes: [regex::Regex; 1],
    pub(crate) session_map: scc::HashMap<uuid::Uuid, Session>, // read/write
    pub(crate) blog_posts_cache: scc::HashMap<uuid::Uuid, CachedPostInfo>, // read/write
    pub(crate) search_index: PostSearchIndex,                  // in-memory full-text search
    pub(crate) geo_ip_db: GeoIpDatabases,                      // read-only, full v4+v6
    pub visitor_board_map: scc::HashMap<([u8; 8], [u8; 8]), u64>, // read/write
    pub(crate) api_keys_set: HashSet<Uuid>,                    // read-only
    pub country_map: RwLock<CountryAndSubdivisionsTable>,
    pub languages_map: RwLock<IsoLanguageTable>,
    pub currency_map: RwLock<IsoCurrencyTable>,
    pub i18n_cache: RwLock<I18nCache>,
    pub(crate) deployment_environment: DeploymentEnvironment,
    pub(crate) request_client: reqwest::Client,
    pub system_info_state: SystemInfoState,
    pub aws_profile_picture_config: aws_config::SdkConfig,
    pub fastfetch: FastFetchCache,
}

impl ServerState {
    pub async fn new_session(
        &self,
        user: &User,
        is_email_verified: bool,
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
                    is_email_verified,
                    created_at: now,
                    expires_at,
                    user_id: user.user_id,
                    user_language: user.user_language,
                    user_name: user.user_name.clone(),
                    user_country: user.user_country,
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
        let (mut pruned, mut remaining): (usize, usize) = (0, 0);

        self.session_map
            .iter_mut_async(|entry| {
                if entry.1.expires_at < now {
                    pruned += 1;
                    let _ = entry.consume();
                    false // continue iterating with mutation
                } else {
                    remaining += 1;
                    true // keep going
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

    pub async fn get_conn(&self) -> anyhow::Result<PooledConnection<'_, AsyncPgConnection>> {
        Ok(self.pool.get().await?)
    }

    pub fn get_email_client(&self) -> &AsyncSmtpTransport<Tokio1Executor> {
        &self.email_client
    }

    pub fn get_responses_handled(&self) -> u64 {
        std::sync::atomic::AtomicU64::load(
            &self.responses_handled,
            std::sync::atomic::Ordering::SeqCst,
        )
    }

    pub async fn check_api_key(&self, key: &Uuid) -> bool {
        self.api_keys_set.contains_async(key).await
    }

    pub async fn insert_api_key(&self, key: Uuid) -> anyhow::Result<()> {
        match self.api_keys_set.insert_async(key).await {
            Ok(_) => Ok(()),
            Err(e) => Err(anyhow::anyhow!("Failed to insert API key: {:?}", e)),
        }
    }

    pub fn add_responses_handled(&self) {
        self.responses_handled
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    pub async fn synchronize_post_info_cache(&self) {
        let start = tokio_now();

        let post_info_vec = match load_post_info(self).await {
            Ok(post_info_vec) => post_info_vec,
            Err(e) => {
                error!("Could not synchronize post metadata cache: {:?}", e);
                return;
            }
        };

        for post_info in &post_info_vec {
            match self
                .blog_posts_cache
                .insert_async(post_info.post_id, post_info.clone())
                .await
            {
                Ok(_) => (),
                Err(_) => {
                    panic!("Could not insert posts into in-RAM cache");
                }
            };
        }

        // Sync search index with cached posts (only published)
        // This checks coherence and only adds/removes what's needed
        let posts_for_index = post_info_vec
            .iter()
            .filter(|p| p.post_is_published)
            .map(|p| (p.post_id, p.post_title.as_str(), p.post_tags.as_slice()));

        match self.search_index.sync_with_posts(posts_for_index) {
            Ok((added, removed)) => {
                if added > 0 || removed > 0 {
                    info!(
                        added = added,
                        removed = removed,
                        total_indexed = self.search_index.num_docs(),
                        "Search index synchronized with cache"
                    );
                } else {
                    info!(
                        total_indexed = self.search_index.num_docs(),
                        "Search index already coherent"
                    );
                }
            }
            Err(e) => {
                error!("Failed to sync search index: {:?}", e);
                // Fall back to full rebuild
                let posts_for_rebuild = post_info_vec
                    .iter()
                    .filter(|p| p.post_is_published)
                    .map(|p| (p.post_id, p.post_title.as_str(), p.post_tags.as_slice()));
                if let Err(e) = self.search_index.rebuild_index(posts_for_rebuild) {
                    error!("Failed to rebuild search index: {:?}", e);
                }
            }
        }

        let elapsed = start.elapsed();
        info!(rows_synchronized = %self.blog_posts_cache.len(), elapsed=%format!("{elapsed:?}"), "Post metadata cache synchronized.");
    }

    pub async fn get_posts_from_cache(
        &self,
        page: usize,
        page_size: usize,
        include_unpublished: bool,
    ) -> (Vec<CachedPostInfo>, usize) {
        let page_size = page_size.max(1);
        let start_index = (page.saturating_sub(1)) * page_size;
        let _end_index = start_index + page_size;

        let mut all_posts: Vec<CachedPostInfo> = Vec::with_capacity(self.blog_posts_cache.len());

        self.blog_posts_cache
            .iter_async(|_, post_info| {
                if include_unpublished || post_info.post_is_published {
                    all_posts.push(post_info.clone());
                }
                true
            })
            .await;

        let total_posts = all_posts.len();
        let total_pages = total_posts.div_ceil(page_size); // Calculate total number of pages

        if start_index >= total_posts {
            return (vec![], total_pages);
        }

        all_posts.sort_by_key(|p| p.post_created_at);
        all_posts.reverse();

        let posts: Vec<CachedPostInfo> = all_posts
            .into_iter()
            .skip(start_index)
            .take(page_size)
            .collect();

        (posts, total_pages)
    }

    pub async fn delete_post_from_cache(&self, post_id: Uuid) {
        let _ = self.blog_posts_cache.remove_async(&post_id).await;
        // Also remove from search index and persist to disk
        if let Err(e) = self.search_index.remove_post_and_commit(post_id) {
            error!("Failed to remove post from search index: {:?}", e);
        }
    }

    pub async fn insert_post_to_cache(&self, post: &CachedPostInfo) {
        let _ = self
            .blog_posts_cache
            .insert_async(post.post_id, post.to_owned())
            .await;
        // Also update search index if post is published and persist to disk
        if post.post_is_published {
            if let Err(e) =
                self.search_index
                    .update_post(post.post_id, &post.post_title, &post.post_tags)
            {
                error!("Failed to update search index: {:?}", e);
            }
        } else {
            // If unpublished, remove from search index (shouldn't be searchable)
            if let Err(e) = self.search_index.remove_post_and_commit(post.post_id) {
                // Ignore "not found" errors - post may not have been indexed
                error!(
                    "Failed to remove unpublished post from search index: {:?}",
                    e
                );
            }
        }
    }

    /// Get a single post from cache by ID.
    pub async fn get_post_from_cache(&self, post_id: &Uuid) -> Option<CachedPostInfo> {
        self.blog_posts_cache
            .read_async(post_id, |_, v| v.clone())
            .await
    }

    /// Search posts by title. Returns matching posts from cache.
    pub async fn search_posts_by_title(&self, query: &str, limit: usize) -> Vec<CachedPostInfo> {
        let post_ids = match self.search_index.search_by_title(query, limit) {
            Ok(ids) => ids,
            Err(e) => {
                error!("Search by title failed: {:?}", e);
                return vec![];
            }
        };

        let mut results = Vec::with_capacity(post_ids.len());
        for post_id in post_ids {
            if let Some(post) = self.get_post_from_cache(&post_id).await {
                results.push(post);
            }
        }
        results
    }

    /// Search posts by tag. Returns matching posts from cache.
    pub async fn search_posts_by_tag(&self, tag: &str, limit: usize) -> Vec<CachedPostInfo> {
        let post_ids = match self.search_index.search_by_tag(tag, limit) {
            Ok(ids) => ids,
            Err(e) => {
                error!("Search by tag failed: {:?}", e);
                return vec![];
            }
        };

        let mut results = Vec::with_capacity(post_ids.len());
        for post_id in post_ids {
            if let Some(post) = self.get_post_from_cache(&post_id).await {
                results.push(post);
            }
        }
        results
    }

    /// full v4+v6 support
    pub fn lookup_ip_location(&self, ip: IpAddr) -> Option<IpInfo> {
        lookup_ip_location_from_map(&self.geo_ip_db, ip)
    }

    pub async fn sync_country_data(&self) -> anyhow::Result<()> {
        let start = tokio::time::Instant::now();

        let country_fut = async {
            let mut conn = self.get_conn().await?;
            let countries: Vec<IsoCountry> = iso_country::table.load(&mut conn).await?;
            let subdivisions: Vec<IsoCountrySubdivision> =
                iso_country_subdivision::table.load(&mut conn).await?;
            let total_rows = countries.len() + subdivisions.len();
            Ok::<(CountryAndSubdivisionsTable, usize), anyhow::Error>((
                CountryAndSubdivisionsTable::new(countries, subdivisions),
                total_rows,
            ))
        };

        let language_fut = async {
            let mut conn = self.get_conn().await?;
            let languages: Vec<IsoLanguage> = iso_language::table.load(&mut conn).await?;
            Ok::<(IsoLanguageTable, usize), anyhow::Error>((
                IsoLanguageTable::from(languages.clone()),
                languages.len(),
            ))
        };

        let currency_fut = async {
            let mut conn = self.get_conn().await?;
            let currencies: Vec<IsoCurrency> = iso_currency::table.load(&mut conn).await?;
            Ok::<(IsoCurrencyTable, usize), anyhow::Error>((
                IsoCurrencyTable::from(currencies.clone()),
                currencies.len(),
            ))
        };

        let (country_res, lang_res, curr_res) =
            tokio::join!(country_fut, language_fut, currency_fut);

        if let Ok((new_country_map, country_rows)) = country_res {
            let mut lock = self.country_map.write().await;
            *lock = new_country_map;
            info!(rows_synchronized = %country_rows, "Synchronized country data data.");
        } else if let Err(e) = country_res {
            tracing::error!("Error synchronizing country data: {:?}", e);
        }

        if let Ok((new_langs_map, lang_rows)) = lang_res {
            let mut lock = self.languages_map.write().await;
            *lock = new_langs_map;
            info!(rows_synchronized = %lang_rows, "Synchronized language data.");
        } else if let Err(e) = lang_res {
            tracing::error!("Error synchronizing languages data: {:?}", e);
        }

        if let Ok((new_currency_map, curr_rows)) = curr_res {
            let mut lock = self.currency_map.write().await;
            *lock = new_currency_map;
            info!(rows_synchronized = %curr_rows, "Synchronized currency data.");
        } else if let Err(e) = curr_res {
            tracing::error!("Error synchronizing currency data: {:?}", e);
        }

        let elapsed = start.elapsed();
        info!(elapsed = %format!("{:?}", elapsed), "Country/language/currency data cache synchronized.");

        Ok(())
    }

    pub async fn sync_i18n_data(&self) -> anyhow::Result<usize> {
        let start = tokio_now();

        let rows = InternationalizationString::get_all(self.get_conn().await?).await?;
        let num_rows = rows.len();
        let mut lock = self.i18n_cache.write().await;
        *lock = I18nCache::from_rows(rows);

        info!(elapsed = ?start.elapsed(), rows_synchronized = %num_rows, "Synchronized i18n data.");
        Ok(num_rows)
    }

    pub async fn sync_visitor_board_data(&self) -> anyhow::Result<usize> {
        use crate::schema::visitation_data::dsl as vdsl;

        let start = tokio_now();
        let mut conn = self.get_conn().await?;

        let visits: Vec<(f64, f64)> = vdsl::visitation_data
            .select((vdsl::latitude, vdsl::longitude))
            .load::<(f64, f64)>(&mut conn)
            .await?;

        let mut visit_counts = std::collections::HashMap::<([u8; 8], [u8; 8]), u64>::new();

        for (latitude, longitude) in visits.iter().copied() {
            let lat_bytes = latitude.to_be_bytes();
            let long_bytes = longitude.to_be_bytes();
            let key = (lat_bytes, long_bytes);
            *visit_counts.entry(key).or_insert(0) += 1;
        }

        for (key, count) in visit_counts {
            let _ = self.visitor_board_map.insert_async(key, count).await;
        }

        let num_rows = visits.len();

        info!(elapsed = ?start.elapsed(), rows_synchronized = %num_rows, "Synchronized visitor board data.");
        Ok(num_rows)
    }

    pub async fn get_visitor_board_entries(&self) -> Vec<((f64, f64), u64)> {
        let mut result = Vec::new();
        self.visitor_board_map
            .iter_async(|&(lat_bytes, long_bytes), &count| {
                let lat = f64::from_be_bytes(lat_bytes);
                let long = f64::from_be_bytes(long_bytes);
                if !lat.is_nan() && !long.is_nan() {
                    result.push(((lat, long), count));
                }
                true // continue iteration
            })
            .await;
        result
    }

    pub fn get_deployment_environment(&self) -> DeploymentEnvironment {
        self.deployment_environment
    }

    pub fn get_request_client(&self) -> &reqwest::Client {
        &self.request_client
    }
}
