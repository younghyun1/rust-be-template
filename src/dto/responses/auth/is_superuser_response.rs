use serde_derive::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct IsSuperuserResponse {
    pub is_superuser: bool,
}
