use anyhow::{Result, anyhow};
use chrono::{DateTime, Duration, SecondsFormat, Timelike, Utc};
use std::sync::Arc;
use tracing::{debug, error, info};

use crate::init::state::ServerState;
use crate::util::time::duration_formatter::format_duration;

/// Calculate the next UTC DateTime that lands on either the current or next second boundary,
/// with a sub-second offset of `millisecond_offset` and `microsecond_offset`.
///
/// For example, millisecond_offset=300 and microsecond_offset=500 means 0.300500s past the second.
/// If that time is already in the past, it will move to the next second.
fn next_scheduled_second_mark(
    now: chrono::DateTime<chrono::Utc>,
    millisecond_offset: u32,
    microsecond_offset: u32,
) -> Result<chrono::DateTime<chrono::Utc>> {
    // 1) Truncate "now" to the current second (drop sub-second components).
    let truncated_to_second = now
        .with_nanosecond(0)
        .ok_or_else(|| anyhow!("Could not truncate to second boundary."))?;

    // 2) Calculate total microseconds offset.
    let total_microseconds_offset = (millisecond_offset as i64 * 1_000) + microsecond_offset as i64;

    // 3) Create the target time by adding the offset to that truncated second.
    let mut target_time =
        truncated_to_second + chrono::Duration::microseconds(total_microseconds_offset);

    // 4) If that target time is behind the current time, add one whole second.
    if target_time <= now {
        target_time += chrono::Duration::seconds(1);
    }

    Ok(target_time)
}

/// Returns (delay, next_mark).
fn next_scheduled_second_delay(
    _task_descriptor: &str,
    millisecond_offset: u32,
    microsecond_offset: u32,
) -> Result<(tokio::time::Duration, DateTime<Utc>)> {
    let now = Utc::now();
    let next_mark = next_scheduled_second_mark(now, millisecond_offset, microsecond_offset)?;

    // Convert Chrono duration to std::time::Duration for Tokio.
    let delay_chrono = next_mark - now;
    let delay_std = delay_chrono.to_std().map_err(|e| {
        anyhow!(
            "Could not schedule job at next_scheduled_second_mark(): {:?}",
            e
        )
    })?;
    Ok((delay_std, next_mark))
}

/// Schedules a task to run once per second at the specified sub-second offset.
/// For example, millisecond_offset=300 and microsecond_offset=500 => runs at X.X300500 each second.
pub async fn schedule_task_every_second_at<F, Fut>(
    state: Arc<ServerState>,
    task: F,
    task_descriptor: String,
    millisecond_offset: u32,
    microsecond_offset: u32,
) -> Result<()>
where
    F: Fn(Arc<ServerState>) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    let mut initialized = false;
    let mut scheduled_run_time: Option<DateTime<Utc>> = None;

    loop {
        let (delay, next_mark) = match next_scheduled_second_delay(
            &task_descriptor,
            millisecond_offset,
            microsecond_offset,
        ) {
            Ok(info) => info,
            Err(e) => {
                error!(
                    "Could not calculate next second mark for '{}': {:?}",
                    task_descriptor, e
                );
                // If we fail, wait some short fallback time and try again.
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
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

        let this_run_time = scheduled_run_time.unwrap_or(next_mark);

        tokio::time::sleep(delay).await;

        let start = tokio::time::Instant::now();
        task(Arc::clone(&state)).await;
        let elapsed = start.elapsed();

        // Efficiently compute the next run time by simply adding one second to the previously scheduled (not actual) scheduled time.
        let next_run_time = this_run_time + Duration::seconds(1);

        debug!(
            task_name = %task_descriptor,
            next_run_time = %next_run_time.to_rfc3339_opts(SecondsFormat::AutoSi, true),
            duration = %format!("{:?}", elapsed),
            "Scheduled task ran! Next one running in {}",
            format_duration((next_run_time - Utc::now()).to_std().unwrap_or(std::time::Duration::from_millis(1000)))
        );

        scheduled_run_time = Some(next_run_time);
    }
}
