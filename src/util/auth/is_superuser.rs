use std::sync::Arc;

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::domain::auth::user_roles::UserRole;
use crate::schema::user_roles;

use crate::{
    errors::code_error::{CodeError, code_err},
    init::state::ServerState,
};

pub async fn is_superuser(state: Arc<ServerState>, user_id: Uuid) -> anyhow::Result<bool> {
    let mut conn = state
        .get_conn()
        .await
        .map_err(|e| code_err(CodeError::POOL_ERROR, e))?;

    let role: UserRole = match user_roles::table
        .filter(user_roles::user_id.eq(user_id))
        .first::<UserRole>(&mut conn)
        .await
    {
        Ok(role) => role,
        Err(e) => match e {
            diesel::result::Error::NotFound => {
                drop(conn);
                return Ok(false);
            }
            _ => {
                drop(conn);
                return Err(anyhow::anyhow!("Failed to fetch user role"));
            }
        },
    };

    drop(conn);

    let role_type = role
        .into_role_type()
        .map_err(|e| anyhow::anyhow!("Failed to convert role type: {}", e))?;

    match role_type {
        crate::domain::auth::role::RoleType::Younghyun => Ok(true),
        crate::domain::auth::role::RoleType::Moderator => Ok(false),
        crate::domain::auth::role::RoleType::User => Ok(false),
        crate::domain::auth::role::RoleType::Guest => Ok(false),
    }
}
