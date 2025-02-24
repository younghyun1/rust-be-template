use chrono::{DateTime, Utc};
use diesel::{
    Insertable, Selectable,
    prelude::{Queryable, QueryableByName},
};

use crate::schema::posts;

#[derive(serde_derive::Serialize, QueryableByName, Queryable, Selectable)]
pub struct Post {
    pub post_id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub post_title: String,
    pub post_slug: String,
    pub post_content: String,
    pub post_summary: Option<String>,
    pub post_created_at: DateTime<Utc>,
    pub post_updated_at: DateTime<Utc>,
    pub post_published_at: Option<DateTime<Utc>>,
    pub post_is_published: bool,
}

#[derive(Insertable)]
#[diesel(table_name = posts)]
pub struct NewPost<'a> {
    pub user_id: &'a uuid::Uuid,
    pub post_title: &'a str,
    pub post_slug: &'a str,
    pub post_content: &'a str,
    pub post_is_published: bool,
}

impl<'a> NewPost<'a> {
    pub fn new(
        user_id: &'a uuid::Uuid,
        post_title: &'a str,
        post_slug: &'a str,
        post_content: &'a str,
        post_is_published: bool,
    ) -> Self {
        Self {
            user_id,
            post_title,
            post_slug,
            post_content,
            post_is_published,
        }
    }
}
