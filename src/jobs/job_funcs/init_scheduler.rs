use std::sync::Arc;

use tracing::info;

use crate::{
    init::state::ServerState,
    jobs::{
        auth::{
            invalidate_sessions::invalidate_sessions,
            purge_nonverified_users::purge_nonverified_users,
            update_system_stats::update_system_stats,
        },
        job_funcs::{
            every_day::schedule_task_every_day_at, every_hour::schedule_task_every_hour_at,
            every_second::schedule_task_every_second_at,
        },
        maintenance::compress_logs::compress_old_logs,
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
            30, // minutes
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
            00, // minutes
            00, // seconds
        )
        .await
    });

    let coroutine_state = Arc::clone(&state);
    tokio::spawn(async move {
        schedule_task_every_second_at(
            coroutine_state,
            move |coroutine_state: Arc<ServerState>| async move {
                update_system_stats(coroutine_state).await
            },
            String::from("UPDATE_SYSTEM_STATS"),
            0,
            0,
        )
        .await
    });

    let coroutine_state = Arc::clone(&state);
    tokio::spawn(async move {
        schedule_task_every_day_at::<_, _>(
            coroutine_state,
            move |coroutine_state: Arc<ServerState>| async move {
                compress_old_logs(coroutine_state).await
            },
            String::from("COMPRESS_OLD_LOGS"),
            6,
            30,
            00,
        )
        .await
    });

    Ok(())
}
