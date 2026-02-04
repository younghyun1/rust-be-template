use crate::domain::auth::user::{UserInfo, UserProfilePicture};
use serde_derive::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct MeResponse {
    pub user_info: Option<UserInfo>,
    pub user_profile_picture: Option<UserProfilePicture>,
    pub build_time: &'static str,
    pub axum_version: String,
    pub rust_version: &'static str,
}
