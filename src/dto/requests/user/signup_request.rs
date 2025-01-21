#[derive(serde_derive::Deserialize)]
pub struct SignupRequest {
    pub user_name: String,
    pub user_email: String,
    pub user_password: String,
}
