use utoipa::ToSchema;
use uuid::Uuid;

#[derive(serde_derive::Deserialize, ToSchema)]
pub struct ResetPasswordProcessRequest {
    pub password_reset_token: Uuid,
    pub new_password: String,
}
