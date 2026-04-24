use std::sync::Arc;

use anyhow::{Result, anyhow};
use chrono::{Datelike, SecondsFormat, TimeZone, Utc};
use tracing::{error, info};

use crate::{init::state::ServerState, util::time::duration_formatter::format_duration};

fn validate_yearly_offsets(
    month_offset: u32,
    day_offset: u32,
    hour_offset: u32,
    minute_offset: u32,
    second_offset: u32,
) -> Result<()> {
    if !(1..=12).contains(&month_offset) {
        error!(month_offset, "Invalid month value");
        return Err(anyhow!("Invalid month for yearly schedule"));
    }
    if !(1..=31).contains(&day_offset) {
        error!(day_offset, month_offset, "Day offset is not in 1..=31");
        return Err(anyhow!("Invalid day offset for yearly schedule"));
    }
    if hour_offset > 23 || minute_offset > 59 || second_offset > 59 {
        error!(
            hour_offset,
            minute_offset, second_offset, "Bad schedule time: hour/minute/second out of range"
        );
        return Err(anyhow!("Invalid time offset for yearly schedule"));
    }
    Ok(())
}

fn next_month(year: i32, month: u32) -> (i32, u32) {
    if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    }
}

fn days_in_month(year: i32, month: u32) -> Result<u32> {
    let start_of_month = match chrono::naive::NaiveDate::from_ymd_opt(year, month, 1) {
        Some(start_of_month) => start_of_month,
        None => {
            error!(
                year,
                month, "Invalid year/month when creating yearly schedule"
            );
            return Err(anyhow!("Invalid year/month for yearly schedule"));
        }
    };
    let (next_year, next_month_value) = next_month(year, month);
    let next_month_start =
        match chrono::naive::NaiveDate::from_ymd_opt(next_year, next_month_value, 1) {
            Some(next_month_start) => next_month_start,
            None => {
                error!(
                    year = next_year,
                    month = next_month_value,
                    "Invalid next month while creating yearly schedule"
                );
                return Err(anyhow!("Invalid next month for yearly schedule"));
            }
        };
    let days = (next_month_start - start_of_month).num_days();
    if days < 1 {
        error!(year, month, days, "Month day count was invalid");
        return Err(anyhow!("Invalid month day count"));
    }
    match u32::try_from(days) {
        Ok(days) => Ok(days),
        Err(e) => {
            error!(year, month, days, error = ?e, "Could not convert month day count");
            Err(anyhow!("Invalid month day count"))
        }
    }
}

fn build_year_mark(
    year: i32,
    month_offset: u32,
    day_offset: u32,
    hour_offset: u32,
    minute_offset: u32,
    second_offset: u32,
) -> Result<chrono::DateTime<chrono::Utc>> {
    let days = days_in_month(year, month_offset)?;
    let day = day_offset.min(days);
    match Utc
        .with_ymd_and_hms(
            year,
            month_offset,
            day,
            hour_offset,
            minute_offset,
            second_offset,
        )
        .single()
    {
        Some(candidate) => Ok(candidate),
        None => {
            error!(
                year,
                month_offset,
                day,
                hour_offset,
                minute_offset,
                second_offset,
                "Could not construct yearly schedule marker"
            );
            Err(anyhow!("Could not construct yearly schedule marker"))
        }
    }
}

/// Calculate the next UTC DateTime for the given year at month, day, hour, minute, and second.
/// If the target time this year has passed, schedules for next year.
/// Month is 1-based (1=January) and day is 1-based.
/// Day is clamped to last day of month if out of range (e.g. Feb 30 -> Feb 28/29).
pub fn next_scheduled_year_mark(
    now: chrono::DateTime<chrono::Utc>,
    month_offset: u32,
    day_offset: u32,
    hour_offset: u32,
    minute_offset: u32,
    second_offset: u32,
) -> Result<chrono::DateTime<chrono::Utc>> {
    validate_yearly_offsets(
        month_offset,
        day_offset,
        hour_offset,
        minute_offset,
        second_offset,
    )?;

    let candidate = build_year_mark(
        now.year(),
        month_offset,
        day_offset,
        hour_offset,
        minute_offset,
        second_offset,
    )?;
    if candidate > now {
        return Ok(candidate);
    }

    build_year_mark(
        now.year() + 1,
        month_offset,
        day_offset,
        hour_offset,
        minute_offset,
        second_offset,
    )
}

/// Returns (delay, next_mark) for the next yearly occurrence.
pub fn next_scheduled_yearly_delay(
    _task_descriptor: &str,
    month_offset: u32,
    day_offset: u32,
    hour_offset: u32,
    minute_offset: u32,
    second_offset: u32,
) -> Result<(tokio::time::Duration, chrono::DateTime<chrono::Utc>)> {
    let now = Utc::now();
    let next_mark = next_scheduled_year_mark(
        now,
        month_offset,
        day_offset,
        hour_offset,
        minute_offset,
        second_offset,
    )?;

    let delay = next_mark - now;
    let delay = delay.to_std().map_err(|e| {
        anyhow!(
            "Could not schedule job at next_scheduled_year_mark(). Chrono->Std error: {:?}",
            e
        )
    })?;

    Ok((delay, next_mark))
}

/// Schedules a task to run once per year at the specific
/// month+day+hour+minute+second offset (e.g., March 5th 02:15:30 UTC each year).
/// Day is clamped to last day of month if out of range.
#[allow(clippy::too_many_arguments)]
pub async fn schedule_task_every_year_at<F, Fut>(
    state: Arc<ServerState>,
    task: F,
    task_descriptor: String,
    month_offset: u32,
    day_offset: u32,
    hour_offset: u32,
    minute_offset: u32,
    second_offset: u32,
) -> Result<()>
where
    F: Fn(Arc<ServerState>) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    let mut initialized = false;
    let mut scheduled_run_time: Option<chrono::DateTime<chrono::Utc>> = None;
    loop {
        let (delay, next_mark) = match next_scheduled_yearly_delay(
            &task_descriptor,
            month_offset,
            day_offset,
            hour_offset,
            minute_offset,
            second_offset,
        ) {
            Ok((delay, next_mark)) => (delay, next_mark),
            Err(e) => {
                error!(
                    task_name = %task_descriptor,
                    error = ?e,
                    "Could not calculate next scheduled time"
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
                delay_human = %format_duration(delay),
                "Scheduled task initialized"
            );
            initialized = true;
        }

        let this_run_time = match scheduled_run_time {
            Some(scheduled_run_time) => scheduled_run_time,
            None => next_mark,
        };

        tokio::time::sleep(delay).await;

        let start = tokio::time::Instant::now();
        task(Arc::clone(&state)).await;
        let elapsed = start.elapsed();

        let next_run_time = match next_scheduled_year_mark(
            this_run_time,
            month_offset,
            day_offset,
            hour_offset,
            minute_offset,
            second_offset,
        ) {
            Ok(next_run_time) => next_run_time,
            Err(e) => {
                error!(
                    task_name = %task_descriptor,
                    error = ?e,
                    "Could not calculate following yearly scheduled time"
                );
                continue;
            }
        };
        let next_delay = match (next_run_time - Utc::now()).to_std() {
            Ok(next_delay) => next_delay,
            Err(e) => {
                error!(task_name = %task_descriptor, error = ?e, "Scheduled task next delay was negative");
                std::time::Duration::from_secs(31_536_000)
            }
        };

        info!(
            task_name = %task_descriptor,
            next_run_time = %next_run_time.to_rfc3339_opts(SecondsFormat::AutoSi, true),
            duration=?elapsed,
            next_delay_human = %format_duration(next_delay),
            "Scheduled task ran"
        );

        scheduled_run_time = Some(next_run_time);
    }
}
