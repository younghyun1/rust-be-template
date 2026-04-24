use std::sync::Arc;

use uuid::Uuid;

use crate::{
    domain::auth::{role::RoleType, user_roles::UserRole},
    errors::code_error::{CodeError, code_err},
    init::state::ServerState,
};

pub async fn is_superuser(state: Arc<ServerState>, user_id: Uuid) -> anyhow::Result<bool> {
    let mut conn = match state.get_conn().await {
        Ok(conn) => conn,
        Err(e) => return Err(anyhow::anyhow!(code_err(CodeError::POOL_ERROR, e))),
    };

    match UserRole::has_role(&mut conn, user_id, RoleType::Younghyun).await {
        Ok(has_role) => Ok(has_role),
        Err(e) => Err(anyhow::anyhow!("Failed to check superuser role: {}", e)),
    }
}
