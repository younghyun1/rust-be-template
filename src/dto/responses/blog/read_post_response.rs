use crate::domain::blog::blog::{CommentResponse, Post, UserBadgeInfo, VoteState};

#[derive(serde_derive::Serialize)]
pub struct ReadPostResponse {
    pub post: Post,
    pub comments: Vec<CommentResponse>,
    pub vote_state: VoteState,
    pub user_badge_info: UserBadgeInfo,
}
