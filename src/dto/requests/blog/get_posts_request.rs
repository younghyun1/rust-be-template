use utoipa::ToSchema;

#[derive(serde_derive::Deserialize, ToSchema)]
#[serde(default = "GetPostsRequest::default")]
pub struct GetPostsRequest {
    #[serde(default = "default_page")]
    pub page: usize,
    #[serde(default = "default_posts_per_page")]
    pub posts_per_page: usize,
}

impl Default for GetPostsRequest {
    fn default() -> Self {
        Self {
            page: default_page(),
            posts_per_page: default_posts_per_page(),
        }
    }
}

#[inline(always)]
fn default_page() -> usize {
    1
}

#[inline(always)]
fn default_posts_per_page() -> usize {
    20
}
