#[derive(serde_derive::Deserialize)]
pub struct SignupRequest {
    pub user_name: String,
    pub user_email: String,
    pub user_password: String,
    pub user_country: i32,
    pub user_language: i32,
    pub user_subdivision: Option<i32>,
}
