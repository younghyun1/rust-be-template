use chrono::TimeZone;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use chrono::{Datelike, SecondsFormat, Utc};
use tracing::{error, info};

use crate::{init::state::ServerState, util::time::duration_formatter::format_duration};

/// Calculate the next UTC DateTime that lands on the given day of the month
/// with provided hour/minute/second offsets.
/// - If the current month's scheduled time has already passed, schedules next month.
/// - Day is clamped to last day of month if requested day > maximum.
pub fn next_scheduled_month_mark(
    now: chrono::DateTime<chrono::Utc>,
    day_offset: u32, // 1-based day of month (1..31)
    hour_offset: u32,
    minute_offset: u32,
    second_offset: u32,
) -> Result<chrono::DateTime<chrono::Utc>> {
    let year = now.year();
    let month = now.month();

    // Check for out-of-bounds date/time values up front and panic/tracing if invalid
    if !(1..=12).contains(&month) {
        tracing::error!(month, "Invalid month value");
        panic!("Invalid month for monthly schedule: month={month}");
    }
    if hour_offset > 23 || minute_offset > 59 || second_offset > 59 {
        tracing::error!(
            hour_offset,
            minute_offset,
            second_offset,
            "Bad schedule time: hour/minute/second out of range"
        );
        panic!(
            "Invalid offset for monthly schedule: hour_offset={hour_offset}, minute_offset={minute_offset}, second_offset={second_offset}",
        );
    }

    let start_of_month = chrono::naive::NaiveDate::from_ymd_opt(year, month, 1);
    if start_of_month.is_none() {
        tracing::error!(year, month, "Invalid year/month when creating NaiveDate");
        panic!("Could not construct NaiveDate for year/month: {year}/{month}");
    }
    let start_of_month = start_of_month.unwrap();

    // Clamp day_offset to actual valid days in this month
    let next_month_start = if month == 12 {
        chrono::naive::NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        chrono::naive::NaiveDate::from_ymd_opt(year, month + 1, 1)
    };
    if next_month_start.is_none() {
        tracing::error!(year, month, "Invalid next month start NaiveDate");
        panic!(
            "Could not construct NaiveDate for next month: {}/{}",
            if month == 12 { year + 1 } else { year },
            if month == 12 { 1 } else { month + 1 }
        );
    }
    let next_month_start = next_month_start.unwrap();
    let days_in_month = (next_month_start - start_of_month).num_days();
    if days_in_month < 1 {
        tracing::error!(
            year,
            month,
            "Current month has less than 1 day, which is impossible."
        );
        panic!("Current month has less than 1 day, this is an impossible state.");
    }
    if !(1..=31).contains(&day_offset) {
        tracing::error!(
            day_offset,
            month,
            year,
            "Day offset is not in 1..=31 (illegal param)"
        );
        panic!("Bad day_offset for schedule: {day_offset}; should be 1..=31");
    }

    let day = day_offset.min(days_in_month as u32);

    // Compose candidate
    let candidate = chrono::Utc
        .with_ymd_and_hms(year, month, day, hour_offset, minute_offset, second_offset)
        .single();
    if candidate.is_none() {
        tracing::error!(
            year,
            month,
            day,
            hour_offset,
            minute_offset,
            second_offset,
            "Could not construct candidate month marker"
        );
        panic!(
            "Could not construct candidate marker for {year}/{month}/{day} {hour_offset}:{minute_offset}:{second_offset}"
        );
    }
    let mut candidate = candidate.unwrap();

    if candidate <= now {
        // Next month: Careful for year/month wrap
        let (next_year, next_month) = if month == 12 {
            (year + 1, 1)
        } else {
            (year, month + 1)
        };
        let start_of_next_month = chrono::naive::NaiveDate::from_ymd_opt(next_year, next_month, 1);
        if start_of_next_month.is_none() {
            tracing::error!(
                next_year,
                next_month,
                "Could not create NaiveDate for next month"
            );
            panic!("Could not construct NaiveDate for next month: {next_year}/{next_month}");
        }
        let start_of_next_month = start_of_next_month.unwrap();
        let next_next_month_start = if next_month == 12 {
            chrono::naive::NaiveDate::from_ymd_opt(next_year + 1, 1, 1)
        } else {
            chrono::naive::NaiveDate::from_ymd_opt(next_year, next_month + 1, 1)
        };
        if next_next_month_start.is_none() {
            tracing::error!(
                next_year,
                next_month,
                "Could not create NaiveDate for the month after"
            );
            panic!(
                "Could not construct NaiveDate for the month after: {}/{}",
                next_year,
                if next_month == 12 { 1 } else { next_month + 1 }
            );
        }
        let next_next_month_start = next_next_month_start.unwrap();
        let days_in_next_month = (next_next_month_start - start_of_next_month).num_days();
        let next_day = day_offset.min(days_in_next_month as u32);
        let candidate2 = chrono::Utc
            .with_ymd_and_hms(
                next_year,
                next_month,
                next_day,
                hour_offset,
                minute_offset,
                second_offset,
            )
            .single();
        if candidate2.is_none() {
            tracing::error!(
                next_year,
                next_month,
                next_day,
                hour_offset,
                minute_offset,
                second_offset,
                "Could not construct next month marker"
            );
            panic!(
                "Could not construct next month candidate marker for {next_year}/{next_month}/{next_day} {hour_offset}:{minute_offset}:{second_offset}"
            );
        }
        candidate = candidate2.unwrap();
    }

    Ok(candidate)
}

/// Returns (delay, next_mark) for the next monthly occurrence.
pub fn next_scheduled_monthly_delay(
    _task_descriptor: &str,
    day_offset: u32,
    hour_offset: u32,
    minute_offset: u32,
    second_offset: u32,
) -> Result<(tokio::time::Duration, chrono::DateTime<chrono::Utc>)> {
    let now = Utc::now();
    let next_mark =
        next_scheduled_month_mark(now, day_offset, hour_offset, minute_offset, second_offset)?;

    let delay = next_mark - now;
    let delay = delay.to_std().map_err(|e| {
        anyhow!(
            "Could not schedule job at next_scheduled_month_mark(). Chrono->Std error: {:?}",
            e
        )
    })?;

    Ok((delay, next_mark))
}

/// Schedules a task to run once per month, at a specific
/// day+hour+minute+second offset (e.g., 10th day 02:15:30 UTC every month).
/// Day is clamped to last day of month if too high.
pub async fn schedule_task_every_month_at<F, Fut>(
    state: Arc<ServerState>,
    task: F,
    task_descriptor: String,
    day_offset: u32,
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
        let (delay, next_mark) = match next_scheduled_monthly_delay(
            &task_descriptor,
            day_offset,
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

        // For next run, must advance 1 month from previous scheduled run,
        // preserving day clamping.
        let mut year = this_run_time.year();
        let mut month = this_run_time.month();
        if month == 12 {
            year += 1;
            month = 1;
        } else {
            month += 1;
        }
        // Clamp day as before
        let days_in_next_month = chrono::naive::NaiveDate::from_ymd_opt(year, month, 1)
            .ok_or_else(|| anyhow!("Invalid next month"))?
            .with_day(1)
            .unwrap()
            .with_month((month % 12) + 1)
            .unwrap_or_else(|| chrono::naive::NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap())
            .signed_duration_since(chrono::naive::NaiveDate::from_ymd_opt(year, month, 1).unwrap())
            .num_days();
        let next_day = day_offset.min(days_in_next_month as u32).max(1);
        let next_run_time = chrono::Utc
            .with_ymd_and_hms(
                year,
                month,
                next_day,
                hour_offset,
                minute_offset,
                second_offset,
            )
            .single()
            .ok_or_else(|| anyhow!("Could not construct following month marker"))?;

        info!(
            task_name = %task_descriptor,
            next_run_time = %next_run_time.to_rfc3339_opts(SecondsFormat::AutoSi, true),
            duration=?elapsed,
            "Scheduled task ran! Next one running in {}",
            format_duration((next_run_time - Utc::now()).to_std().unwrap_or(std::time::Duration::from_secs(2_592_000)))
        );

        scheduled_run_time = Some(next_run_time);
    }
}
