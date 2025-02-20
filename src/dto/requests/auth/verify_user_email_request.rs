#[derive(serde_derive::Deserialize)]
pub struct VerifyUserEmailRequest {
    pub email_verification_token: uuid::Uuid,
}
