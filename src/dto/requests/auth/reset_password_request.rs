use utoipa::ToSchema;

#[derive(serde_derive::Deserialize, ToSchema)]
pub struct ResetPasswordRequest {
    pub user_email: String,
}
