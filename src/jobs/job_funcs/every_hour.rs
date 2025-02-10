use std::sync::Arc;

use anyhow::{anyhow, Result};
use chrono::{SecondsFormat, Timelike, Utc};
use tracing::{error, info};

use crate::init::state::ServerState;

/// Calculate the next UTC DateTime that lands on the current/next hour,
/// with a specific "minute + second" offset from the start of that hour.
///
/// For example, minute_offset=15 and second_offset=30 will schedule
/// the time XX:15:30 of the next hour (if that time has already passed in this hour).
pub fn next_scheduled_hour_mark(
    now: chrono::DateTime<chrono::Utc>,
    minute_offset: u32,
    second_offset: u32,
) -> Result<chrono::DateTime<chrono::Utc>> {
    // 1) Truncate to the current hour boundary.
    let truncated_to_hour = now
        .with_minute(0)
        .and_then(|dt| dt.with_second(0))
        .and_then(|dt| dt.with_nanosecond(0))
        .ok_or_else(|| anyhow!("Could not truncate to hour."))?;

    // 2) Add the desired minute + second offset to get the target time within this hour.
    let mut target_time = truncated_to_hour
        + chrono::Duration::minutes(minute_offset as i64)
        + chrono::Duration::seconds(second_offset as i64);

    // 3) If that target time is before 'now', move it to the next hour.
    if target_time <= now {
        target_time += chrono::Duration::hours(1);
    }

    Ok(target_time)
}

/// A helper that returns both (delay, next_mark).
/// It calculates how long until the next scheduled mark (hourly) from now.
pub fn next_scheduled_hourly_delay(
    _task_descriptor: &str,
    minute_offset: u32,
    second_offset: u32,
) -> Result<(tokio::time::Duration, chrono::DateTime<chrono::Utc>)> {
    let now = Utc::now();
    let next_mark = next_scheduled_hour_mark(now, minute_offset, second_offset)?;

    // Convert the difference into a std::time::Duration for tokio.
    let delay = next_mark - now;
    let delay = delay.to_std().map_err(|e| {
        anyhow!(
            "Could not schedule job at next_scheduled_hour_mark(). Chrono->Std error: {:?}",
            e
        )
    })?;

    Ok((delay, next_mark))
}

/// Schedules a task to run once per hour, at a specific
/// minute+second offset (e.g., 15m + 30s into every hour).
pub async fn schedule_task_every_hour_at<F, Fut>(
    state: Arc<ServerState>,
    task: F,
    task_descriptor: String,
    minute_offset: u32,
    second_offset: u32,
) -> Result<()>
where
    F: Fn(Arc<ServerState>) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    loop {
        // Get how long until the next scheduled run.
        let (delay, next_mark) =
            match next_scheduled_hourly_delay(&task_descriptor, minute_offset, second_offset) {
                Ok((d, nm)) => (d, nm),
                Err(e) => {
                    error!(
                        "Could not calculate next scheduled time for {}: {:?}",
                        task_descriptor, e
                    );
                    // On computation failure, wait a short while before retrying.
                    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                    continue;
                }
            };

        info!(
            task_name = %task_descriptor,
            next_run_time = %next_mark.to_rfc3339_opts(SecondsFormat::AutoSi, true),
            "Scheduled task"
        );

        // Sleep until the scheduled time arrives.
        tokio::time::sleep(delay).await;

        // Run the actual task.
        task(Arc::clone(&state)).await;
        // After the task finishes, loop back to schedule it again in the next hour.
    }
}
