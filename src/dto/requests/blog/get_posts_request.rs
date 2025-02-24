#[derive(serde_derive::Deserialize)]
pub struct GetPostsRequest {
    pub page: u32,
    pub posts_per_page: u8,
    pub user_id: Option<uuid::Uuid>,
    pub post_slug: Option<String>,
    pub is_published: Option<bool>,
}
