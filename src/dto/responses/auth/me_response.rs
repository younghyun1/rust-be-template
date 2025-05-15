use crate::domain::user::{UserInfo, UserProfilePicture};
use serde_derive::Serialize;

#[derive(Serialize)]
pub struct MeResponse {
    pub user_info: UserInfo,
    pub user_profile_picture: Option<UserProfilePicture>,
}
