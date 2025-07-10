use chrono::{DateTime, Utc};

#[derive(serde_derive::Serialize)]
pub struct ResetPasswordResponse {
    pub user_id: uuid::Uuid,
    pub user_name: String,
    pub user_email: String,
    pub user_updated_at: DateTime<Utc>,
}
