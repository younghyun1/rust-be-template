use std::sync::Arc;

use tracing::info;

use crate::{
    init::state::ServerState,
    jobs::{
        auth::{
            invalidate_sessions::invalidate_sessions,
            purge_nonverified_users::purge_nonverified_users,
        },
        job_funcs::every_hour::schedule_task_every_hour_at,
    },
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
            45, // minutes
            00, // seconds
        )
        .await
    });

    let coroutine_state = Arc::clone(&state);
    tokio::spawn(async move {
        schedule_task_every_hour_at(
            coroutine_state,
            move |coroutine_state: Arc<ServerState>| async move {
                purge_nonverified_users(coroutine_state).await
            },
            String::from("PURGE_NONVERIFIED_USERS"),
            15, // minutes
            00, // seconds
        )
        .await
    });

    Ok(())
}
