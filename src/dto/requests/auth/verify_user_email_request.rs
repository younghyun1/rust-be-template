#[derive(serde_derive::Deserialize)]
pub struct EmailValidationToken {
    pub email_validation_token_id: uuid::Uuid,
}
