use diesel::{
    ExpressionMethods, Insertable, OptionalExtension, QueryDsl,
    prelude::{Queryable, QueryableByName},
};
use diesel_async::{AsyncPgConnection, RunQueryDsl};
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

    pub async fn role_for_user(
        conn: &mut AsyncPgConnection,
        user_id: Uuid,
    ) -> anyhow::Result<Option<RoleType>> {
        let role_id = match user_roles::table
            .filter(user_roles::user_id.eq(user_id))
            .select(user_roles::role_id)
            .first::<Uuid>(conn)
            .await
            .optional()
        {
            Ok(role_id) => role_id,
            Err(e) => return Err(anyhow::anyhow!("Failed to fetch user role: {}", e)),
        };

        match role_id {
            Some(role_id) => match RoleType::from_uuid(role_id) {
                Ok(role_type) => Ok(Some(role_type)),
                Err(e) => Err(e),
            },
            None => Ok(None),
        }
    }

    pub async fn role_for_user_or_insert_default(
        conn: &mut AsyncPgConnection,
        user_id: Uuid,
        default_role_type: RoleType,
    ) -> anyhow::Result<RoleType> {
        match Self::role_for_user(conn, user_id).await {
            Ok(Some(role_type)) => return Ok(role_type),
            Ok(None) => (),
            Err(e) => return Err(e),
        }

        match Self::insert_for_user(conn, user_id, default_role_type).await {
            Ok(()) => Ok(default_role_type),
            Err(diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::UniqueViolation,
                _,
            )) => match Self::role_for_user(conn, user_id).await {
                Ok(Some(role_type)) => Ok(role_type),
                Ok(None) => Err(anyhow::anyhow!(
                    "User role was created concurrently but not found"
                )),
                Err(e) => Err(e),
            },
            Err(e) => Err(anyhow::anyhow!("Failed to insert default user role: {}", e)),
        }
    }

    pub async fn insert_for_user(
        conn: &mut AsyncPgConnection,
        user_id: Uuid,
        role_type: RoleType,
    ) -> Result<(), diesel::result::Error> {
        let new_user_role = NewUserRole {
            user_id,
            role_id: role_type.id(),
        };

        match diesel::insert_into(user_roles::table)
            .values(new_user_role)
            .execute(conn)
            .await
        {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }

    pub async fn has_role(
        conn: &mut AsyncPgConnection,
        user_id: Uuid,
        role_type: RoleType,
    ) -> Result<bool, diesel::result::Error> {
        let role_exists = user_roles::table
            .filter(user_roles::user_id.eq(user_id))
            .filter(user_roles::role_id.eq(role_type.id()));

        match diesel::select(diesel::dsl::exists(role_exists))
            .get_result::<bool>(conn)
            .await
        {
            Ok(has_role) => Ok(has_role),
            Err(e) => Err(e),
        }
    }
}

#[derive(Insertable)]
#[diesel(table_name = user_roles)]
pub struct NewUserRole {
    user_id: Uuid,
    role_id: Uuid,
}
