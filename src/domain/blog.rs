use chrono::{DateTime, Utc};
use diesel::{
    Insertable, Selectable,
    prelude::{Queryable, QueryableByName},
};

use crate::schema::{comment_votes, post_votes, posts};

#[derive(Clone, serde_derive::Serialize, QueryableByName, Queryable, Selectable)]
#[diesel(table_name = posts)]
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
    pub post_view_count: i64,
    pub post_share_count: i64,
    pub post_metadata: serde_json::Value,
    pub total_upvotes: i64,
    pub total_downvotes: i64,
}

// TODO: return user info w. profile picture link and stuff
#[derive(
    Clone,
    serde_derive:: Serialize,
    serde_derive::Deserialize,
    Queryable,
    QueryableByName,
    Selectable,
)]
#[diesel(table_name = posts)]
pub struct PostInfo {
    pub post_id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub post_title: String,
    pub post_slug: String,
    pub post_summary: Option<String>,
    pub post_created_at: DateTime<Utc>,
    pub post_updated_at: DateTime<Utc>,
    pub post_published_at: Option<DateTime<Utc>>,
    pub post_view_count: i64,
    pub post_share_count: i64,
    pub total_upvotes: i64,
    pub total_downvotes: i64,
}

impl From<Post> for PostInfo {
    fn from(post: Post) -> Self {
        Self {
            post_id: post.post_id,
            user_id: post.user_id,
            post_title: post.post_title,
            post_slug: post.post_slug,
            post_summary: post.post_summary,
            post_created_at: post.post_created_at,
            post_updated_at: post.post_updated_at,
            post_published_at: post.post_published_at,
            post_view_count: post.post_view_count,
            post_share_count: post.post_share_count,
            total_upvotes: post.total_upvotes,
            total_downvotes: post.total_downvotes,
        }
    }
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

use crate::schema::comments;

#[derive(Clone, serde_derive::Serialize, QueryableByName, Queryable, Selectable)]
#[diesel(table_name = comments)]
pub struct Comment {
    pub comment_id: uuid::Uuid,
    pub post_id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub comment_content: String,
    pub comment_created_at: DateTime<Utc>,
    pub comment_updated_at: Option<DateTime<Utc>>,
    pub parent_comment_id: Option<uuid::Uuid>,
    pub total_upvotes: i64,
    pub total_downvotes: i64,
}

#[derive(Clone, serde_derive::Serialize, QueryableByName, Queryable, Selectable)]
#[diesel(table_name = comment_votes)]
pub struct CommentVote {
    pub vote_id: uuid::Uuid,
    pub comment_id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub is_upvote: bool,
}

#[derive(Insertable)]
#[diesel(table_name = comment_votes)]
pub struct NewCommentVote<'a> {
    pub comment_id: &'a uuid::Uuid,
    pub user_id: &'a uuid::Uuid,
    pub is_upvote: bool,
}

impl<'a> NewCommentVote<'a> {
    pub fn new(comment_id: &'a uuid::Uuid, user_id: &'a uuid::Uuid, is_upvote: bool) -> Self {
        Self {
            comment_id,
            user_id,
            is_upvote,
        }
    }
}

#[derive(Clone, serde_derive::Serialize, QueryableByName, Queryable, Selectable)]
#[diesel(table_name = post_votes)]
pub struct PostVote {
    pub vote_id: uuid::Uuid,
    pub post_id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub is_upvote: bool,
}

#[derive(Insertable)]
#[diesel(table_name = post_votes)]
pub struct NewPostVote<'a> {
    pub post_id: &'a uuid::Uuid,
    pub user_id: &'a uuid::Uuid,
    pub is_upvote: bool,
}

impl<'a> NewPostVote<'a> {
    pub fn new(post_id: &'a uuid::Uuid, user_id: &'a uuid::Uuid, is_upvote: bool) -> Self {
        Self {
            post_id,
            user_id,
            is_upvote,
        }
    }
}

#[derive(serde_derive::Deserialize, serde_derive::Serialize)]
pub struct PostMetadata {
    pub tags: Vec<String>,
}
