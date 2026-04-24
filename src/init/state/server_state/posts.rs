use tracing::{error, info};
use uuid::Uuid;

use super::ServerState;
use crate::domain::blog::blog::CachedPostInfo;
use crate::init::load_cache::post_info::load_post_info;
use crate::util::time::now::tokio_now;

impl ServerState {
    fn normalize_post_slug(slug: &str) -> Option<String> {
        let normalized = slug.trim().to_lowercase();
        if normalized.is_empty() {
            None
        } else {
            Some(normalized)
        }
    }

    async fn upsert_post_cache_internal(&self, post: &CachedPostInfo, sync_search_index: bool) {
        let previous_slug = self
            .blog_posts_cache
            .read_async(&post.post_id, |_, cached| cached.post_slug.clone())
            .await;

        if self
            .blog_posts_cache
            .update_async(&post.post_id, |_, cached| {
                *cached = post.clone();
            })
            .await
            .is_none()
        {
            let _ = self
                .blog_posts_cache
                .insert_async(post.post_id, post.clone())
                .await;
        }

        let new_slug_normalized = Self::normalize_post_slug(&post.post_slug);
        if let Some(old_slug) = previous_slug
            && let Some(old_slug_normalized) = Self::normalize_post_slug(&old_slug)
            && let Some(mapped_post_id) = self
                .blog_post_slug_cache
                .read_async(&old_slug_normalized, |_, post_id| *post_id)
                .await
            && mapped_post_id == post.post_id
            && Some(old_slug_normalized.as_str()) != new_slug_normalized.as_deref()
        {
            let _ = self
                .blog_post_slug_cache
                .remove_async(&old_slug_normalized)
                .await;
        }

        if let Some(new_slug) = new_slug_normalized
            && self
                .blog_post_slug_cache
                .update_async(&new_slug, |_, mapped_post_id| {
                    *mapped_post_id = post.post_id;
                })
                .await
                .is_none()
        {
            let _ = self
                .blog_post_slug_cache
                .insert_async(new_slug, post.post_id)
                .await;
        }

        if !sync_search_index {
            return;
        }

        if post.post_is_published {
            if let Err(e) =
                self.search_index
                    .update_post(post.post_id, &post.post_title, &post.post_tags)
            {
                error!(error = ?e, post_id = %post.post_id, "Failed to update search index");
            }
        } else if let Err(e) = self.search_index.remove_post_and_commit(post.post_id) {
            error!(
                error = ?e,
                post_id = %post.post_id,
                "Failed to remove unpublished post from search index"
            );
        }
    }

    async fn rebuild_post_order_cache(&self) {
        let mut ordered_posts: Vec<(chrono::DateTime<chrono::Utc>, Uuid)> =
            Vec::with_capacity(self.blog_posts_cache.len());

        self.blog_posts_cache
            .iter_async(|post_id, post_info| {
                ordered_posts.push((post_info.post_created_at, *post_id));
                true
            })
            .await;

        ordered_posts.sort_by_key(|post| std::cmp::Reverse(post.0));
        let ordered_post_ids = ordered_posts
            .into_iter()
            .map(|(_, post_id)| post_id)
            .collect();
        let mut lock = self.blog_post_order_cache.write().await;
        *lock = ordered_post_ids;
    }

    pub async fn synchronize_post_info_cache(&self) {
        let start = tokio_now();

        let post_info_vec = match load_post_info(self).await {
            Ok(post_info_vec) => post_info_vec,
            Err(e) => {
                error!(error = ?e, "Could not synchronize post metadata cache");
                return;
            }
        };

        self.blog_posts_cache
            .iter_mut_async(|entry| {
                let _ = entry.consume();
                true
            })
            .await;
        self.blog_post_slug_cache
            .iter_mut_async(|entry| {
                let _ = entry.consume();
                true
            })
            .await;

        for post_info in &post_info_vec {
            self.upsert_post_cache_internal(post_info, false).await;
        }
        self.rebuild_post_order_cache().await;

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
                error!(error = ?e, "Failed to sync search index");
                let posts_for_rebuild = post_info_vec
                    .iter()
                    .filter(|p| p.post_is_published)
                    .map(|p| (p.post_id, p.post_title.as_str(), p.post_tags.as_slice()));
                if let Err(e) = self.search_index.rebuild_index(posts_for_rebuild) {
                    error!(error = ?e, "Failed to rebuild search index");
                }
            }
        }

        let elapsed = start.elapsed();
        info!(
            rows_synchronized = %self.blog_posts_cache.len(),
            slug_rows_synchronized = %self.blog_post_slug_cache.len(),
            elapsed=%format!("{elapsed:?}"),
            "Post metadata cache synchronized."
        );
    }

    pub async fn get_posts_from_cache(
        &self,
        page: usize,
        page_size: usize,
        include_unpublished: bool,
    ) -> (Vec<CachedPostInfo>, usize) {
        let page_size = page_size.max(1);
        let start_index = (page.saturating_sub(1)) * page_size;
        let ordered_post_ids = {
            let lock = self.blog_post_order_cache.read().await;
            lock.clone()
        };

        let mut posts: Vec<CachedPostInfo> = Vec::with_capacity(page_size);
        let mut visible_posts = 0usize;

        for post_id in ordered_post_ids {
            let post = match self.get_post_from_cache(&post_id).await {
                Some(post) => post,
                None => continue,
            };

            if !include_unpublished && !post.post_is_published {
                continue;
            }

            if visible_posts >= start_index && posts.len() < page_size {
                posts.push(post);
            }
            visible_posts += 1;
        }

        let total_pages = visible_posts.div_ceil(page_size);

        (posts, total_pages)
    }

    pub async fn delete_post_from_cache(&self, post_id: Uuid) {
        if let Some((_, removed_post)) = self.blog_posts_cache.remove_async(&post_id).await
            && let Some(removed_slug) = Self::normalize_post_slug(&removed_post.post_slug)
            && let Some(mapped_post_id) = self
                .blog_post_slug_cache
                .read_async(&removed_slug, |_, cached_post_id| *cached_post_id)
                .await
            && mapped_post_id == post_id
        {
            let _ = self.blog_post_slug_cache.remove_async(&removed_slug).await;
        }
        self.rebuild_post_order_cache().await;
        if let Err(e) = self.search_index.remove_post_and_commit(post_id) {
            error!(error = ?e, post_id = %post_id, "Failed to remove post from search index");
        }
    }

    pub async fn insert_post_to_cache(&self, post: &CachedPostInfo) {
        self.upsert_post_cache_internal(post, true).await;
        self.rebuild_post_order_cache().await;
    }

    pub async fn insert_post_to_cache_without_search_sync(&self, post: &CachedPostInfo) {
        self.upsert_post_cache_internal(post, false).await;
        self.rebuild_post_order_cache().await;
    }

    pub async fn get_post_from_cache(&self, post_id: &Uuid) -> Option<CachedPostInfo> {
        self.blog_posts_cache
            .read_async(post_id, |_, v| v.clone())
            .await
    }

    pub async fn get_post_id_by_slug_from_cache(&self, post_slug: &str) -> Option<Uuid> {
        let normalized_slug = Self::normalize_post_slug(post_slug)?;
        self.blog_post_slug_cache
            .read_async(&normalized_slug, |_, post_id| *post_id)
            .await
    }

    pub async fn cache_post_slug_mapping(&self, post_slug: &str, post_id: Uuid) {
        let Some(normalized_slug) = Self::normalize_post_slug(post_slug) else {
            return;
        };

        if self
            .blog_post_slug_cache
            .update_async(&normalized_slug, |_, cached_post_id| {
                *cached_post_id = post_id;
            })
            .await
            .is_none()
        {
            let _ = self
                .blog_post_slug_cache
                .insert_async(normalized_slug, post_id)
                .await;
        }
    }

    pub async fn get_post_from_cache_by_slug(&self, post_slug: &str) -> Option<CachedPostInfo> {
        let post_id = self.get_post_id_by_slug_from_cache(post_slug).await?;
        self.get_post_from_cache(&post_id).await
    }

    async fn posts_from_ids(&self, post_ids: Vec<Uuid>) -> Vec<CachedPostInfo> {
        let mut results = Vec::with_capacity(post_ids.len());
        for post_id in post_ids {
            if let Some(post) = self.get_post_from_cache(&post_id).await {
                results.push(post);
            }
        }
        results
    }

    pub async fn search_posts_by_title(
        &self,
        query: &str,
        offset: usize,
        limit: usize,
    ) -> (Vec<CachedPostInfo>, usize) {
        let (post_ids, total_matches) = match self
            .search_index
            .search_by_title_paged(query, offset, limit)
        {
            Ok(result) => result,
            Err(e) => {
                error!(error = ?e, "Search by title failed");
                return (vec![], 0);
            }
        };

        (self.posts_from_ids(post_ids).await, total_matches)
    }

    pub async fn search_posts_by_title_and_tags(
        &self,
        query: &str,
        tags: &[String],
        offset: usize,
        limit: usize,
    ) -> (Vec<CachedPostInfo>, usize) {
        let (post_ids, total_matches) = match self
            .search_index
            .search_by_title_and_tags_paged(query, tags, offset, limit)
        {
            Ok(result) => result,
            Err(e) => {
                error!(error = ?e, "Search by title and tags failed");
                return (vec![], 0);
            }
        };

        (self.posts_from_ids(post_ids).await, total_matches)
    }

    pub async fn search_posts_by_tags(
        &self,
        tags: &[String],
        offset: usize,
        limit: usize,
    ) -> (Vec<CachedPostInfo>, usize) {
        let (post_ids, total_matches) =
            match self.search_index.search_by_tags_paged(tags, offset, limit) {
                Ok(result) => result,
                Err(e) => {
                    error!(error = ?e, "Search by tags failed");
                    return (vec![], 0);
                }
            };

        (self.posts_from_ids(post_ids).await, total_matches)
    }

    pub async fn search_posts_by_tag(
        &self,
        tag: &str,
        offset: usize,
        limit: usize,
    ) -> (Vec<CachedPostInfo>, usize) {
        let (post_ids, total_matches) =
            match self.search_index.search_by_tag_paged(tag, offset, limit) {
                Ok(result) => result,
                Err(e) => {
                    error!(error = ?e, "Search by tag failed");
                    return (vec![], 0);
                }
            };

        (self.posts_from_ids(post_ids).await, total_matches)
    }
}
