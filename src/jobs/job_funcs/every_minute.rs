use std::sync::Arc;

use anyhow::{Result, anyhow};
use chrono::{Duration, SecondsFormat, Timelike, Utc};
use tracing::{error, info};

use crate::init::state::ServerState;
use crate::util::time::duration_formatter::format_duration;

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
        target_time += chrono::Duration::seconds(60);
    }

    Ok(target_time)
}

/// A helper that returns both (delay, next_mark).
/// It calculates how long until the next_scheduled_mark(...) from now.
pub fn next_scheduled_delay(
    _task_descriptor: &str,
    second_offset: u32,
    millisecond_offset: u32,
) -> Result<(tokio::time::Duration, chrono::DateTime<chrono::Utc>)> {
    let now = Utc::now();
    let next_mark = next_scheduled_mark(now, second_offset, millisecond_offset)?;

    // Convert that difference into std::time::Duration for tokio.
    let delay = next_mark - now;
    let delay = delay.to_std().map_err(|e| {
        anyhow!(
            "Could not schedule job at next_scheduled_mark(). Chrono->Std error: {:?}",
            e
        )
    })?;

    Ok((delay, next_mark))
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
    let mut initialized = false;
    let mut scheduled_run_time: Option<chrono::DateTime<chrono::Utc>> = None;

    loop {
        let (delay, next_mark) =
            match next_scheduled_delay(&task_descriptor, second_offset, millisecond_offset) {
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
                delay = %format!("{:?}", delay),
                "Scheduled task initialized. First run upcoming in {}",
                format_duration(delay)
            );
            initialized = true;
        }

        let this_run_time = match scheduled_run_time {
            Some(rt) => rt,
            None => next_mark,
        };

        tokio::time::sleep(delay).await;

        let start = tokio::time::Instant::now();
        task(Arc::clone(&state)).await;
        let elapsed = start.elapsed();

        // Efficiently compute the next run time by simply adding one minute
        let next_run_time = this_run_time + Duration::minutes(1);

        info!(
            task_name = %task_descriptor,
            next_run_time = %next_run_time.to_rfc3339_opts(SecondsFormat::AutoSi, true),
            duration = %format!("{:?}", elapsed),
            "Scheduled task ran! Next one running in {}",
            format_duration((next_run_time - Utc::now()).to_std().unwrap_or(std::time::Duration::from_secs(60)))
        );

        scheduled_run_time = Some(next_run_time);
    }
}
