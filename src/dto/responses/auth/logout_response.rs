use utoipa::ToSchema;

#[derive(serde_derive::Serialize, ToSchema)]
pub struct LogoutResponse {
    pub message: String,
}
