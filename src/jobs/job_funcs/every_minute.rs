use std::sync::Arc;

use anyhow::{anyhow, Result};
use chrono::{Timelike, Utc};
use tracing::{error, info};

use crate::{init::state::ServerState, util::time::duration_formatter::format_dt_difference};

/// Calculate the next UTC DateTime that lands on the current/next minute,
/// with a specific "seconds + milliseconds" offset from the start of that minute.
///
/// For example, second_offset=30 and millisecond_offset=500 would schedule
/// time XX:YY:30.500 of the next minute that is still >= now.
pub fn next_scheduled_mark(
    now: chrono::DateTime<chrono::Utc>,
    second_offset: u32,
    millisecond_offset: u32,
) -> Result<chrono::DateTime<chrono::Utc>> {
    // 1) Truncate to the current minute boundary (floor).
    let truncated_to_minute = now
        .with_second(0)
        .and_then(|dt| dt.with_nanosecond(0))
        .ok_or_else(|| anyhow!("Could not truncate to minute."))?;

    // 2) Add the desired second + millisecond offset to get the target time this minute.
    let mut target_time = truncated_to_minute
        + chrono::Duration::seconds(second_offset as i64)
        + chrono::Duration::milliseconds(millisecond_offset as i64);

    // 3) If that target time is before 'now', then move it to the next minute by adding 60s.
    if target_time <= now {
        target_time = target_time + chrono::Duration::seconds(60);
    }

    Ok(target_time)
}

/// A helper that returns both (delay, human-readable schedule message).
/// It calculates how long until the next_scheduled_mark(...) from now.
pub fn next_scheduled_delay(
    task_descriptor: &str,
    second_offset: u32,
    millisecond_offset: u32,
) -> Result<(tokio::time::Duration, String)> {
    let now = Utc::now();
    let next_mark = next_scheduled_mark(now, second_offset, millisecond_offset)?;

    // We'll produce a user-friendly scheduling message.
    let schedule_msg = format!(
        "Task '{}' will run in {}",
        task_descriptor,
        format_dt_difference(now, next_mark)
    );

    // Convert that difference into std::time::Duration for tokio.
    let delay = next_mark - now;
    let delay = delay.to_std().map_err(|e| {
        anyhow!(
            "Could not schedule job at next_scheduled_mark(). Chrono->Std error: {:?}",
            e
        )
    })?;

    Ok((delay, schedule_msg))
}

/// Schedules a task to run once per minute, but at a specific
/// second+millisecond offset (e.g., 30s + 500ms into every minute).
pub async fn schedule_task_every_minute_at<F, Fut>(
    state: Arc<ServerState>,
    task: F,
    task_descriptor: String,
    second_offset: u32,
    millisecond_offset: u32,
) -> Result<()>
where
    F: Fn(Arc<ServerState>) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    loop {
        // Get how long until the next scheduled run.
        let (delay, schedule_message) =
            match next_scheduled_delay(&task_descriptor, second_offset, millisecond_offset) {
                Ok((d, m)) => (d, m),
                Err(e) => {
                    error!(
                        "Could not calculate next scheduled time for {}: {:?}",
                        task_descriptor, e
                    );
                    // If we fail to compute it, try again in a short while:
                    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                    continue;
                }
            };

        info!("{}", schedule_message);

        // Sleep until that time arrives.
        tokio::time::sleep(delay).await;

        // Run the actual task.
        task(Arc::clone(&state)).await;
        // After the task finishes, we'll loop back and schedule again
        // for the next minute's desired offset.
    }
}
