use std::collections::HashMap;

use diesel::{ExpressionMethods, QueryDsl, SelectableHelper};
use diesel_async::RunQueryDsl;
use tracing::error;
use uuid::Uuid;

use crate::{
    domain::blog::blog::{CachedPostInfo, PostInfo},
    init::state::ServerState,
    schema::{post_tags, posts, tags},
};

pub async fn load_post_info(state: &ServerState) -> anyhow::Result<Vec<CachedPostInfo>> {
    let mut conn = match state.get_conn().await {
        Ok(conn) => conn,
        Err(e) => {
            error!(error = %e, "Could not get conn out of pool while synchronizing state.");
            return Err(e);
        }
    };

    // Load all posts
    let post_infos: Vec<PostInfo> = match posts::table
        .select(PostInfo::as_select())
        .order(posts::post_created_at.desc())
        .load::<PostInfo>(&mut conn)
        .await
    {
        Ok(post_infos) => post_infos,
        Err(e) => return Err(e.into()),
    };

    // Load all post_tags with tag names in one query
    let tag_data: Vec<(Uuid, String)> = match post_tags::table
        .inner_join(tags::table)
        .select((post_tags::post_id, tags::tag_name))
        .load::<(Uuid, String)>(&mut conn)
        .await
    {
        Ok(data) => data,
        Err(e) => return Err(e.into()),
    };

    drop(conn);

    // Build a map of post_id -> Vec<tag_name>
    let mut tags_by_post: HashMap<Uuid, Vec<String>> = HashMap::new();
    for (post_id, tag_name) in tag_data {
        tags_by_post.entry(post_id).or_default().push(tag_name);
    }

    // Combine posts with their tags
    let cached_posts: Vec<CachedPostInfo> = post_infos
        .into_iter()
        .map(|post| {
            let tags = tags_by_post.remove(&post.post_id).unwrap_or_default();
            CachedPostInfo::from_post_info_with_tags(post, tags)
        })
        .collect();

    Ok(cached_posts)
}
