use uuid::Uuid;

#[derive(serde_derive::Deserialize)]
pub struct ResetPasswordProcessRequest {
    pub password_reset_token_id: Uuid,
    pub new_password: String,
}
