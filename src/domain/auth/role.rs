use serde_derive::{Deserialize, Serialize};
use uuid::Uuid;

pub enum Roles {
    Younghyun,
    Moderator,
    User,
    Guest,
}

const ROLE_YOUNGHYUN: u128 = 2131042872073453539493660941469037155;
const ROLE_MODERATOR: u128 = 2131042883709330333470894399469323316;
const ROLE_USER: u128 = 2131042888123140653623930835701279230;
const ROLE_GUEST: u128 = 2131042895169936790354381715792830592;

impl Roles {
    pub fn get_role(role_id: Uuid) -> anyhow::Result<Self> {
        match role_id.as_u128() {
            ROLE_YOUNGHYUN => Ok(Roles::Younghyun),
            ROLE_MODERATOR => Ok(Roles::Moderator),
            ROLE_USER => Ok(Roles::User),
            ROLE_GUEST => Ok(Roles::Guest),
            _ => Err(anyhow::anyhow!("Invalid role ID")),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Role {
    id: Uuid,
    name: String,
    description: String,
}
