use std::sync::Arc;

use uuid::Uuid;

use crate::init::state::ServerState;

pub async fn is_superuser(_state: Arc<ServerState>, _user_id: Uuid) -> anyhow::Result<bool> {
    // TODO: actual DB query logic for roles
    Ok(false)
}
