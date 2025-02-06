use std::sync::Arc;

use tracing::info;

use crate::{
    init::state::ServerState,
    jobs::{
        auth::invalidate_sessions::invalidate_sessions,
        job_funcs::every_minute::schedule_task_every_minute_at,
    },
    // jobs::job_funcs::{
    //     every_minute::schedule_task_every_minute_at, every_second::schedule_task_every_second_at,
    // },
};

pub async fn task_init(state: Arc<ServerState>) -> anyhow::Result<()> {
    info!("Task scheduler running...");

    tokio::spawn(async move {
        let state_1 = Arc::clone(&state);
        let state_2 = Arc::clone(&state);
        schedule_task_every_minute_at(
            state_1,
            move |_| {
                let task_state = state_2.clone();
                async move {
                    // Clone inside the closure so that invalidate_sessions gets its own Arc copy.
                    invalidate_sessions(Arc::clone(&task_state)).await;
                }
            },
            String::from("INVALIDATE_EXPIRED_SESSIONS"),
            0,
            0,
        )
        .await
    });

    Ok(())
}
