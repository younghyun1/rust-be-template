use chrono::{DateTime, Utc};

#[derive(serde_derive::Serialize)]
pub struct ResetPasswordRequestResponse {
    pub user_email: String,
    pub verify_by: DateTime<Utc>,
}
