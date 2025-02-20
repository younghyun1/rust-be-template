#[derive(serde_derive::Deserialize)]
pub struct LoginRequest {
    pub user_email: String,
    pub user_password: String,
}
