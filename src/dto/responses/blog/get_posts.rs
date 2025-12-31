use serde_derive::Serialize;
use utoipa::ToSchema;

use crate::domain::blog::blog::PostInfoWithVote;

#[derive(Serialize, ToSchema)]
pub struct GetPostsResponse {
    pub posts: Vec<PostInfoWithVote>,
    pub available_pages: usize,
}
