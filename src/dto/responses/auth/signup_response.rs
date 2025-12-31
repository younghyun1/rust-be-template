use utoipa::ToSchema;

#[derive(serde_derive::Serialize, ToSchema)]
pub struct SignupResponse {
    pub user_name: String,
    pub user_email: String,
    pub verify_by: chrono::DateTime<chrono::Utc>,
}
