#[derive(serde_derive::Serialize)]
pub struct EmailValidateResponse {
    pub user_email: String,
    pub verified_at: chrono::DateTime<chrono::Utc>,
}
