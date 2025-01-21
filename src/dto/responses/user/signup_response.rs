#[derive(serde_derive::Serialize)]
pub struct SignupResponse {
    pub user_name: String,
    pub user_email: String,
}
