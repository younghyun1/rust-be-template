#[derive(serde_derive::Deserialize)]
#[serde(default = "default_get_posts_request")]
pub struct GetPostsRequest {
    #[serde(default = "default_page")]
    pub page: usize,
    #[serde(default = "default_posts_per_page")]
    pub posts_per_page: usize,
}

fn default_get_posts_request() -> GetPostsRequest {
    GetPostsRequest {
        page: default_page(),
        posts_per_page: default_posts_per_page(),
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
