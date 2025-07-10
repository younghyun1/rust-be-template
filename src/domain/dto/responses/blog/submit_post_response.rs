use chrono::{DateTime, Utc};
use serde_derive::Serialize;
use uuid::Uuid;

#[derive(Serialize)]
pub struct SubmitPostResponse {
    pub post_id: Uuid,
    pub post_title: String,
    pub post_slug: String,
    pub post_created_at: DateTime<Utc>,
    pub post_updated_at: DateTime<Utc>,
    pub post_is_published: bool,
}
