#[derive(serde_derive::Deserialize)]
pub struct GetPostsRequest {
    pub page: usize,
    pub posts_per_page: usize,
}
