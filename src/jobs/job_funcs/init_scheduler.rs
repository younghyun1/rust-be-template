use std::sync::Arc;

use tracing::info;

use crate::{
    init::state::ServerState,
    jobs::job_funcs::{
        every_minute::schedule_task_every_minute_at, every_second::schedule_task_every_second_at,
    },
};

pub async fn task_init(state: Arc<ServerState>) -> anyhow::Result<()> {
    // Use the functions in the scheduling_defs folder to spawn looping tasks that consume an Arc<ServerState>, which is all you need
    info!("Task scheduler running...");

    let coroutine_state = Arc::clone(&state);
    tokio::spawn(async move {
        schedule_task_every_second_at(
            coroutine_state,
            move |_| async move {
                info!("Task executed");
            },
            String::from("example_"),
            20,
            100,
        )
        .await
    });

    Ok(())
}
