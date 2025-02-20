#[derive(serde_derive::Deserialize)]
pub struct CheckIfUserExistsRequest {
    pub user_email: String,
}
