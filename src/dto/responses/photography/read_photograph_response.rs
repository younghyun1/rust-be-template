use utoipa::ToSchema;

use crate::domain::blog::blog::{UserBadgeInfo, VoteState};
use crate::domain::photography::photographs::Photograph;
use crate::domain::photography::social::PhotographCommentResponse;

/// Detail response for a single photograph: the row (incl. denormalized view +
/// vote counts), the caller's vote state, the enriched flat comment list, and
/// the photograph author's badge. Comments are threaded client-side via
/// `parent_photograph_comment_id`.
#[derive(serde_derive::Serialize, ToSchema)]
pub struct ReadPhotographResponse {
    pub photograph: Photograph,
    pub vote_state: VoteState,
    pub comments: Vec<PhotographCommentResponse>,
    pub user_badge_info: UserBadgeInfo,
}
