use chrono::{DateTime, Utc};
use diesel::Insertable;

use crate::schema::posts;

#[derive(serde_derive::Serialize)]
pub struct Post {
    post_id: uuid::Uuid,
    user_id: uuid::Uuid,
    post_title: String,
    post_slug: String,
    post_content: String,
    post_summary: Option<String>,
    post_created_at: DateTime<Utc>,
    post_updated_at: DateTime<Utc>,
    post_published_at: Option<DateTime<Utc>>,
    post_is_published: bool,
}

#[derive(Insertable)]
#[diesel(table_name = posts)]
pub struct NewPost<'a> {
    pub user_id: &'a uuid::Uuid,
    pub post_title: &'a str,
    pub post_slug: &'a str,
    pub post_content: &'a str,
}

impl<'a> NewPost<'a> {
    pub fn new(
        user_id: &'a uuid::Uuid,
        post_title: &'a str,
        post_slug: &'a str,
        post_content: &'a str,
    ) -> Self {
        Self {
            user_id,
            post_title,
            post_slug,
            post_content,
        }
    }
}
