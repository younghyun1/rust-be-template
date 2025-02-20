use uuid::Uuid;

#[derive(serde_derive::Serialize)]
pub struct LoginResponse {
    pub message: String,
    pub user_id: Uuid,
}
