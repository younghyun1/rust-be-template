use std::sync::Arc;

use anyhow::{Result, anyhow};
use chrono::{Datelike, Duration, SecondsFormat, Timelike, Utc, Weekday};
use tracing::{error, info};

use crate::{init::state::ServerState, util::time::duration_formatter::format_duration};

/// Calculate the next UTC DateTime that lands on the specified weekday, hour, minute, and second,
/// starting from 'now'. If the target time this week has already passed, schedule for the following week.
///
/// For example, target_weekday=Weekday::Mon, hour_offset=2, minute_offset=15, second_offset=30
/// will find the next Monday at 02:15:30 UTC that is still >= now.
pub fn next_scheduled_week_mark(
    now: chrono::DateTime<chrono::Utc>,
    target_weekday: Weekday,
    hour_offset: u32,
    minute_offset: u32,
    second_offset: u32,
) -> Result<chrono::DateTime<chrono::Utc>> {
    // 1) Truncate today to 00:00:00.
    let today_midnight = now
        .with_hour(0)
        .and_then(|dt| dt.with_minute(0))
        .and_then(|dt| dt.with_second(0))
        .and_then(|dt| dt.with_nanosecond(0));
    if today_midnight.is_none() {
        tracing::error!(%now, "Failed to truncate to start-of-day for scheduling.");
        panic!("Failed to truncate to start-of-day for scheduling. Input: {now}");
    }
    let today_midnight = today_midnight.unwrap();

    // 2) Find day difference (how many days to the next target weekday).
    let current_weekday = now.weekday();
    let mut days_ahead = (target_weekday.number_from_monday() as i64
        - current_weekday.number_from_monday() as i64)
        .rem_euclid(7);
    // Sanity check offsets
    if hour_offset > 23 || minute_offset > 59 || second_offset > 59 {
        tracing::error!(
            hour_offset,
            minute_offset,
            second_offset,
            "Bad schedule time: hour/minute/second out of range"
        );
        panic!(
            "Invalid offset for weekly schedule: hour_offset={hour_offset}, minute_offset={minute_offset}, second_offset={second_offset}",
        );
    }
    let target_time_today = today_midnight
        + chrono::Duration::hours(hour_offset as i64)
        + chrono::Duration::minutes(minute_offset as i64)
        + chrono::Duration::seconds(second_offset as i64);

    if days_ahead == 0 && target_time_today <= now {
        // If today but time has passed, schedule to next week.
        days_ahead = 7;
    } else if days_ahead == 0 && target_time_today > now {
        // Today and time is in the future
        // Keep days_ahead = 0;
    } // else days_ahead > 0, so the target day is later this week

    // 3) Compose the next scheduled datetime
    let target_time = today_midnight
        + chrono::Duration::days(days_ahead)
        + chrono::Duration::hours(hour_offset as i64)
        + chrono::Duration::minutes(minute_offset as i64)
        + chrono::Duration::seconds(second_offset as i64);

    Ok(target_time)
}

/// Returns (delay, next_mark) for the next scheduled weekly occurrence.
pub fn next_scheduled_weekly_delay(
    _task_descriptor: &str,
    weekday: Weekday,
    hour_offset: u32,
    minute_offset: u32,
    second_offset: u32,
) -> Result<(tokio::time::Duration, chrono::DateTime<chrono::Utc>)> {
    let now = Utc::now();
    let next_mark =
        next_scheduled_week_mark(now, weekday, hour_offset, minute_offset, second_offset)?;

    let delay = next_mark - now;
    let delay = delay.to_std().map_err(|e| {
        anyhow!(
            "Could not schedule job at next_scheduled_week_mark(). Chrono->Std error: {:?}",
            e
        )
    })?;

    Ok((delay, next_mark))
}

/// Schedules a task to run once per week, at a specific
/// weekday+hour+minute+second offset (e.g., Monday 02:15:30 UTC every week).
/// Pass the desired chrono::Weekday directly as the weekday argument.
pub async fn schedule_task_every_week_at<F, Fut>(
    state: Arc<ServerState>,
    task: F,
    task_descriptor: String,
    weekday: Weekday,
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
        let (delay, next_mark) = match next_scheduled_weekly_delay(
            &task_descriptor,
            weekday,
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

        // Add one week to the previously scheduled run time.
        let next_run_time = this_run_time + Duration::weeks(1);

        info!(
            task_name = %task_descriptor,
            next_run_time = %next_run_time.to_rfc3339_opts(SecondsFormat::AutoSi, true),
            duration=?elapsed,
            "Scheduled task ran! Next one running in {}",
            format_duration((next_run_time - Utc::now()).to_std().unwrap_or(std::time::Duration::from_secs(604800)))
        );

        scheduled_run_time = Some(next_run_time);
    }
}
