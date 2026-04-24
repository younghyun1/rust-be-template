use chrono::{DateTime, Utc};
use serde_derive::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PublicUserInfoResponse {
    pub user_id: Uuid,
    pub user_name: String,
    pub user_created_at: DateTime<Utc>,
    pub user_country_flag: Option<String>,
    pub user_profile_picture_url: Option<String>,
}
