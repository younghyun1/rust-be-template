use chrono::{DateTime, Utc};
use utoipa::ToSchema;

#[derive(serde_derive::Serialize, ToSchema)]
pub struct ResetPasswordRequestResponse {
    pub user_email: String,
    pub verify_by: DateTime<Utc>,
}
