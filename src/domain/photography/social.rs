//! Domain models for photograph social features (votes + threaded comments).
//!
//! Separate tables from the blog system but the same policy and shapes. The
//! `VoteState` enum and `UserBadgeInfo` are reused from the blog domain to keep
//! the public vote/badge representation identical across the site.

use chrono::{DateTime, Utc};
use diesel::{Insertable, Queryable, QueryableByName, Selectable};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::blog::blog::{UserBadgeInfo, VoteState};
use crate::schema::photograph_comments;

/// A photograph comment row as stored in the database.
#[derive(Clone, serde_derive::Serialize, QueryableByName, Queryable, Selectable, ToSchema)]
#[diesel(table_name = photograph_comments)]
pub struct PhotographComment {
    pub photograph_comment_id: Uuid,
    pub photograph_id: Uuid,
    pub user_id: Uuid,
    pub photograph_comment_content: String,
    pub photograph_comment_created_at: DateTime<Utc>,
    pub photograph_comment_updated_at: Option<DateTime<Utc>>,
    pub parent_photograph_comment_id: Option<Uuid>,
    pub photograph_comment_total_upvotes: i64,
    pub photograph_comment_total_downvotes: i64,
}

/// Insertable for a new photograph comment (supports threading via parent id).
#[derive(Insertable)]
#[diesel(table_name = photograph_comments)]
pub struct NewPhotographComment<'a> {
    pub photograph_id: &'a Uuid,
    pub user_id: &'a Uuid,
    pub photograph_comment_content: &'a str,
    pub parent_photograph_comment_id: Option<&'a Uuid>,
}

/// Enriched comment for API responses: adds the caller's vote state + the
/// author's badge (name, profile picture, country flag).
#[derive(Clone, serde_derive::Serialize, ToSchema)]
pub struct PhotographCommentResponse {
    pub photograph_comment_id: Uuid,
    pub photograph_id: Uuid,
    pub user_id: Uuid,
    pub photograph_comment_content: String,
    pub photograph_comment_created_at: DateTime<Utc>,
    pub photograph_comment_updated_at: Option<DateTime<Utc>>,
    pub parent_photograph_comment_id: Option<Uuid>,
    pub photograph_comment_total_upvotes: i64,
    pub photograph_comment_total_downvotes: i64,
    pub vote_state: VoteState,
    pub user_name: String,
    pub user_profile_picture_url: String,
    pub user_country_flag: Option<String>,
}

impl PhotographCommentResponse {
    pub fn from_comment_votestate_and_badge_info(
        comment: PhotographComment,
        vote_state: VoteState,
        user_badge_info: UserBadgeInfo,
    ) -> Self {
        Self {
            photograph_comment_id: comment.photograph_comment_id,
            photograph_id: comment.photograph_id,
            user_id: comment.user_id,
            photograph_comment_content: comment.photograph_comment_content,
            photograph_comment_created_at: comment.photograph_comment_created_at,
            photograph_comment_updated_at: comment.photograph_comment_updated_at,
            parent_photograph_comment_id: comment.parent_photograph_comment_id,
            photograph_comment_total_upvotes: comment.photograph_comment_total_upvotes,
            photograph_comment_total_downvotes: comment.photograph_comment_total_downvotes,
            vote_state,
            user_name: user_badge_info.user_name,
            user_profile_picture_url: user_badge_info.user_profile_picture_url,
            user_country_flag: user_badge_info.user_country_flag,
        }
    }
}

/// Result row for a FILTER-based upvote/downvote recount.
#[derive(QueryableByName)]
pub struct VoteCounts {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    pub upvote_count: i64,
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    pub downvote_count: i64,
}
