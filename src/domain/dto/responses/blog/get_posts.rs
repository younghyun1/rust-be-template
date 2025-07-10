use serde_derive::Serialize;

use crate::domain::blog::blog::PostInfoWithVote;

#[derive(Serialize)]
pub struct GetPostsResponse {
    pub posts: Vec<PostInfoWithVote>,
    pub available_pages: usize,
}
