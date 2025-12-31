use utoipa::ToSchema;

#[derive(serde_derive::Deserialize, ToSchema)]
pub struct CheckIfUserExistsRequest {
    pub user_email: String,
}
