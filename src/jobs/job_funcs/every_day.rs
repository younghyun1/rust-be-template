use std::sync::Arc;

use anyhow::{Result, anyhow};
use chrono::{Duration, SecondsFormat, Timelike, Utc};
use tracing::{error, info};

use crate::{init::state::ServerState, util::time::duration_formatter::format_duration};

/// Calculate the next UTC DateTime that lands on the current or next day,
/// with a specific "hour + minute + second" offset from the start of that day.
///
/// For example, hour_offset=2, minute_offset=15, second_offset=30 will schedule
/// XX-XX-XX 02:15:30 on the next day, if that time has already passed today.
pub fn next_scheduled_day_mark(
    now: chrono::DateTime<chrono::Utc>,
    hour_offset: u32,
    minute_offset: u32,
    second_offset: u32,
) -> Result<chrono::DateTime<chrono::Utc>> {
    // 1) Truncate to the current day boundary.
    let truncated_to_day = now
        .with_hour(0)
        .and_then(|dt| dt.with_minute(0))
        .and_then(|dt| dt.with_second(0))
        .and_then(|dt| dt.with_nanosecond(0));
    if truncated_to_day.is_none() {
        tracing::error!(%now, "Failed to truncate to start-of-day for scheduling.");
        panic!("Failed to truncate to start-of-day for scheduling. Input: {now}");
    }
    let truncated_to_day = truncated_to_day.unwrap();

    // 2) Add the desired hour + minute + second offset to get the target time within this day.
    // Sanity check offsets
    if hour_offset > 23 || minute_offset > 59 || second_offset > 59 {
        tracing::error!(
            hour_offset,
            minute_offset,
            second_offset,
            "Bad schedule time: hour/minute/second out of range"
        );
        panic!(
            "Invalid offset for daily schedule: hour_offset={hour_offset}, minute_offset={minute_offset}, second_offset={second_offset}",
        );
    }
    let mut target_time = truncated_to_day
        + chrono::Duration::hours(hour_offset as i64)
        + chrono::Duration::minutes(minute_offset as i64)
        + chrono::Duration::seconds(second_offset as i64);

    // 3) If that target time is before 'now', move it to the next day.
    if target_time <= now {
        target_time += chrono::Duration::days(1);
    }

    Ok(target_time)
}

/// A helper that returns both (delay, next_mark).
/// It calculates how long until the next scheduled mark (daily) from now.
pub fn next_scheduled_daily_delay(
    _task_descriptor: &str,
    hour_offset: u32,
    minute_offset: u32,
    second_offset: u32,
) -> Result<(tokio::time::Duration, chrono::DateTime<chrono::Utc>)> {
    let now = Utc::now();
    let next_mark = next_scheduled_day_mark(now, hour_offset, minute_offset, second_offset)?;

    // Convert the difference into a std::time::Duration for tokio.
    let delay = next_mark - now;
    let delay = delay.to_std().map_err(|e| {
        anyhow!(
            "Could not schedule job at next_scheduled_day_mark(). Chrono->Std error: {:?}",
            e
        )
    })?;

    Ok((delay, next_mark))
}

/// Schedules a task to run once per day, at a specific
/// hour+minute+second offset (e.g., 02:15:30 UTC every day).
pub async fn schedule_task_every_day_at<F, Fut>(
    state: Arc<ServerState>,
    task: F,
    task_descriptor: String,
    hour_offset: u32,
    minute_offset: u32,
    second_offset: u32,
) -> Result<()>
where
    F: Fn(Arc<ServerState>) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    let mut initialized: bool = false;
    let mut scheduled_run_time: Option<chrono::DateTime<chrono::Utc>> = None;
    loop {
        let (delay, next_mark) = match next_scheduled_daily_delay(
            &task_descriptor,
            hour_offset,
            minute_offset,
            second_offset,
        ) {
            Ok((d, nm)) => (d, nm),
            Err(e) => {
                error!(
                    "Could not calculate next scheduled time for {}: {:?}",
                    task_descriptor, e
                );
                tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                continue;
            }
        };

        if !initialized {
            info!(
                task_name = %task_descriptor,
                initial_run_time = %next_mark.to_rfc3339_opts(SecondsFormat::AutoSi, true),
                ?delay,
                "Scheduled task initialized. First run upcoming in {}",
                format_duration(delay)
            );
            initialized = true;
        }

        let this_run_time = scheduled_run_time.unwrap_or(next_mark);

        tokio::time::sleep(delay).await;

        let start = tokio::time::Instant::now();
        task(Arc::clone(&state)).await;
        let elapsed = start.elapsed();

        // Efficient: just add one day to the already-calculated scheduled time.
        let next_run_time = this_run_time + Duration::days(1);

        info!(
            task_name = %task_descriptor,
            next_run_time = %next_run_time.to_rfc3339_opts(SecondsFormat::AutoSi, true),
            duration=?elapsed,
            "Scheduled task ran! Next one running in {}",
            format_duration((next_run_time - Utc::now()).to_std().unwrap_or(std::time::Duration::from_secs(86400)))
        );

        scheduled_run_time = Some(next_run_time);
    }
}
