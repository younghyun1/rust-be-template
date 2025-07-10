#[derive(serde_derive::Deserialize)]
pub struct ResetPasswordRequest {
    pub user_email: String,
}
