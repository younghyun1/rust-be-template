use utoipa::ToSchema;

#[derive(serde_derive::Deserialize, ToSchema)]
pub struct EmailValidationToken {
    pub email_validation_token_id: uuid::Uuid,
}
