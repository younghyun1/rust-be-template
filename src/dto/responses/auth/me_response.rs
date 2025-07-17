use crate::domain::user::{UserInfo, UserProfilePicture};
use serde_derive::Serialize;

#[derive(Serialize)]
pub struct MeResponse {
    pub user_info: Option<UserInfo>,
    pub user_profile_picture: Option<UserProfilePicture>,
    pub build_time: &'static str,
    pub axum_version: String,
}
