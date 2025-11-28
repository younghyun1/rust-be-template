use diesel::prelude::{Queryable, QueryableByName};
use serde_derive::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{domain::auth::role::RoleType, schema::user_roles};

#[derive(Serialize, Deserialize, QueryableByName, Queryable)]
#[diesel(table_name = user_roles)]
pub struct UserRole {
    user_role_id: Uuid,
    user_id: Uuid,
    role_id: Uuid,
}

impl UserRole {
    pub fn into_role_type(self) -> anyhow::Result<RoleType> {
        RoleType::from_uuid(self.role_id)
    }
}
