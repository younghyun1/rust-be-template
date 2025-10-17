use std::sync::Arc;

use crate::init::state::ServerState;

pub async fn update_system_stats(state: Arc<ServerState>) {
    state.system_info_state.update().await;
}
