use diesel::{Queryable, QueryableByName};
use serde_derive::{Deserialize, Serialize};
use uuid::Uuid;

use crate::schema::roles;

#[repr(u8)]
pub enum RoleType {
    Younghyun = 0,
    Moderator = 1,
    User = 2,
    Guest = 3,
}

// 019a6c86-8bca-7b91-b9c0-1d4cc96b3263
const ROLE_YOUNGHYUN: u128 = 2131042872073453539493660941469037155;
// 019a6c86-b163-7452-aa70-5997736b0434
const ROLE_MODERATOR: u128 = 2131042883709330333470894399469323316;
// 019a6c86-bfa6-7903-9176-dc5f66f729fe
const ROLE_USER: u128 = 2131042888123140653623930835701279230;
// 019a6c86-d66b-7223-97ef-a8a26551a080
const ROLE_GUEST: u128 = 2131042895169936790354381715792830592;

#[derive(Serialize, Deserialize, QueryableByName, Queryable)]
#[diesel(table_name = roles)]
pub struct Role {
    role_id: Uuid,
    role_name: String,
    role_description: String,
}

impl Role {
    pub fn get_from_role_id(self) -> anyhow::Result<RoleType> {
        match self.role_id.as_u128() {
            ROLE_YOUNGHYUN => Ok(RoleType::Younghyun),
            ROLE_MODERATOR => Ok(RoleType::Moderator),
            ROLE_USER => Ok(RoleType::User),
            ROLE_GUEST => Ok(RoleType::Guest),
            _ => Err(anyhow::anyhow!("Invalid role ID")),
        }
    }
}
