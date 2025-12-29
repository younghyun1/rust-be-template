use utoipa::ToSchema;
use uuid::Uuid;

#[derive(serde_derive::Serialize, ToSchema)]
pub struct LoginResponse {
    pub message: String,
    pub user_id: Uuid,
}
