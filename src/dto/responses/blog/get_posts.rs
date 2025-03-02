use serde_derive::Serialize;

use crate::domain::blog::PostInfo;

#[derive(Serialize)]
pub struct GetPostsResponse {
    pub posts: Vec<PostInfo>, 
    pub available_pages: usize,
}
