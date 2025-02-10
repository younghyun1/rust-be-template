use std::sync::Arc;

use tracing::info;

use crate::{
    init::state::ServerState,
    jobs::{
        auth::invalidate_sessions::invalidate_sessions,
        job_funcs::every_hour::schedule_task_every_hour_at,
    },
    // jobs::job_funcs::{
    //     every_minute::schedule_task_every_minute_at, every_second::schedule_task_every_second_at,
    // },
};

pub async fn task_init(state: Arc<ServerState>) -> anyhow::Result<()> {
    info!("Task scheduler running...");

    let coroutine_state = Arc::clone(&state);
    tokio::spawn(async move {
        schedule_task_every_hour_at(
            coroutine_state,
            move |coroutine_state: Arc<ServerState>| async move {
                invalidate_sessions(coroutine_state).await
            },
            String::from("INVALIDATE_EXPIRED_SESSIONS"),
            0,
            0,
        )
        .await
    });
    

    Ok(())
}
