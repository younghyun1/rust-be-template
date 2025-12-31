use chrono::{DateTime, Utc};
use utoipa::ToSchema;

use diesel::{
    Insertable, Selectable,
    prelude::{Queryable, QueryableByName},
};

use crate::schema::{comment_votes, post_tags, post_votes, posts, tags};

#[derive(Clone, serde_derive::Serialize, QueryableByName, Queryable, Selectable, ToSchema)]
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
    serde_derive::Serialize,
    serde_derive::Deserialize,
    Queryable,
    QueryableByName,
    Selectable,
    ToSchema,
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

#[derive(serde_derive::Serialize, ToSchema)]
pub struct UserBadgeInfo {
    pub user_name: String,
    pub user_profile_picture_url: String,
}

#[derive(serde_derive::Serialize, ToSchema)]
pub struct PostInfoWithVote {
    pub post_id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub user_name: String,
    pub user_profile_picture_url: String,
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
    pub vote_state: VoteState,
}

impl PostInfoWithVote {
    pub fn from_info_with_vote(
        post_info: PostInfo,
        vote_state: VoteState,
        user_badge_info: UserBadgeInfo,
    ) -> Self {
        Self {
            post_id: post_info.post_id,
            user_id: post_info.user_id,
            user_name: user_badge_info.user_name,
            user_profile_picture_url: user_badge_info.user_profile_picture_url,
            post_title: post_info.post_title,
            post_slug: post_info.post_slug,
            post_summary: post_info.post_summary,
            post_created_at: post_info.post_created_at,
            post_updated_at: post_info.post_updated_at,
            post_published_at: post_info.post_published_at,
            post_view_count: post_info.post_view_count,
            post_share_count: post_info.post_share_count,
            total_upvotes: post_info.total_upvotes,
            total_downvotes: post_info.total_downvotes,
            vote_state,
        }
    }
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

#[derive(Clone, serde_derive::Serialize, QueryableByName, Queryable, Selectable, ToSchema)]
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
#[derive(Clone, serde_derive::Serialize, ToSchema)]
pub struct CommentResponse {
    pub comment_id: uuid::Uuid,
    pub post_id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub comment_content: String,
    pub comment_created_at: DateTime<Utc>,
    pub comment_updated_at: Option<DateTime<Utc>>,
    pub parent_comment_id: Option<uuid::Uuid>,
    pub total_upvotes: i64,
    pub total_downvotes: i64,
    pub vote_state: VoteState,
    pub user_name: String,
    pub user_profile_picture_url: String,
}
impl CommentResponse {
    pub fn from_comment_votestate_and_badge_info(
        comment: Comment,
        vote_state: VoteState,
        user_badge_info: UserBadgeInfo,
    ) -> Self {
        Self {
            comment_id: comment.comment_id,
            post_id: comment.post_id,
            user_id: comment.user_id,
            comment_content: comment.comment_content,
            comment_created_at: comment.comment_created_at,
            comment_updated_at: comment.comment_updated_at,
            parent_comment_id: comment.parent_comment_id,
            total_upvotes: comment.total_upvotes,
            total_downvotes: comment.total_downvotes,
            vote_state,
            user_name: user_badge_info.user_name,
            user_profile_picture_url: user_badge_info.user_profile_picture_url,
        }
    }
}

#[derive(Clone, serde_derive::Serialize, QueryableByName, Queryable, Selectable, ToSchema)]
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

#[derive(Clone, serde_derive::Serialize, QueryableByName, Queryable, Selectable, ToSchema)]
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

#[derive(serde_derive::Deserialize, serde_derive::Serialize, ToSchema)]
pub struct PostMetadata {}

#[derive(Clone, serde_derive::Serialize, QueryableByName, Queryable, Selectable, ToSchema)]
#[diesel(table_name = tags)]
pub struct Tag {
    pub tag_id: i16,
    pub tag_name: String,
}

#[derive(Insertable)]
#[diesel(table_name = tags)]
pub struct NewTag<'a> {
    pub tag_name: &'a str,
}

impl<'a> NewTag<'a> {
    pub fn new(tag_name: &'a str) -> Self {
        Self { tag_name }
    }
}

#[derive(Clone, serde_derive::Serialize, QueryableByName, Queryable, Selectable, ToSchema)]
#[diesel(table_name = post_tags)]
pub struct PostTag {
    pub post_id: uuid::Uuid,
    pub tag_id: i16,
}

#[derive(Insertable)]
#[diesel(table_name = post_tags)]
pub struct NewPostTag<'a> {
    pub post_id: &'a uuid::Uuid,
    pub tag_id: &'a i16,
}

impl<'a> NewPostTag<'a> {
    pub fn new(post_id: &'a uuid::Uuid, tag_id: &'a i16) -> Self {
        Self { post_id, tag_id }
    }
}

#[repr(u8)]
#[derive(Clone, ToSchema)]
pub enum VoteState {
    Upvoted,
    Downvoted,
    DidNotVote,
}

impl serde::Serialize for VoteState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let value = match self {
            VoteState::Upvoted => 0u8,
            VoteState::Downvoted => 1u8,
            VoteState::DidNotVote => 2u8,
        };
        serializer.serialize_u8(value)
    }
}
