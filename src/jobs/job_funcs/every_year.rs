use std::sync::Arc;

use anyhow::{Result, anyhow};
use chrono::{Datelike, SecondsFormat, TimeZone, Utc};
use tracing::{error, info};

use crate::{init::state::ServerState, util::time::duration_formatter::format_duration};

/// Calculate the next UTC DateTime for the given year at month, day, hour, minute, and second.
/// If the target time this year has passed, schedules for next year.
/// Month is 1-based (1=January) and day is 1-based.
/// Day is clamped to last day of month if out of range (e.g. Feb 30 -> Feb 28/29).
pub fn next_scheduled_year_mark(
    now: chrono::DateTime<chrono::Utc>,
    month_offset: u32, // 1-based (1=Jan..12=Dec)
    day_offset: u32,   // 1-based
    hour_offset: u32,
    minute_offset: u32,
    second_offset: u32,
) -> Result<chrono::DateTime<chrono::Utc>> {
    let year = now.year();

    // Check inputs for valid range and log/panic on error
    if !(1..=12).contains(&month_offset) {
        tracing::error!(month_offset, "Invalid month value");
        panic!("Invalid month for yearly schedule: {month_offset:?}");
    }
    if hour_offset > 23 || minute_offset > 59 || second_offset > 59 {
        tracing::error!(
            hour_offset,
            minute_offset,
            second_offset,
            "Bad schedule time: hour/minute/second out of range"
        );
        panic!(
            "Invalid offset for yearly schedule: hour={hour_offset}, min={minute_offset}, sec={second_offset}",
        );
    }
    if !(1..=31).contains(&day_offset) {
        tracing::error!(
            day_offset,
            month_offset,
            "Day offset is not in 1..=31 (illegal param)"
        );
        panic!(
            "Bad day_offset for yearly schedule: {day_offset}; should be 1..=31"
        );
    }

    let start_of_month = chrono::naive::NaiveDate::from_ymd_opt(year, month_offset, 1)
        .unwrap_or_else(|| {
            tracing::error!(
                year,
                month_offset,
                "Could not create NaiveDate for year/month"
            );
            panic!(
                "Could not create NaiveDate for year/month: {year}/{month_offset}"
            )
        });
    let next_month_start = if month_offset == 12 {
        chrono::naive::NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        chrono::naive::NaiveDate::from_ymd_opt(year, month_offset + 1, 1)
    }
    .unwrap_or_else(|| {
        tracing::error!(
            year,
            month_offset,
            "Could not create NaiveDate for next month"
        );
        panic!(
            "Could not construct NaiveDate for next month: {}/{}",
            if month_offset == 12 { year + 1 } else { year },
            if month_offset == 12 {
                1
            } else {
                month_offset + 1
            }
        );
    });
    let days_in_month = (next_month_start - start_of_month).num_days();
    if days_in_month < 1 {
        tracing::error!(
            year,
            month_offset,
            "Current month has less than 1 day, which is impossible."
        );
        panic!("Current month has less than 1 day, impossible.");
    }
    let day = day_offset.min(days_in_month as u32);

    // Build candidate date
    let candidate = chrono::Utc
        .with_ymd_and_hms(
            year,
            month_offset,
            day,
            hour_offset,
            minute_offset,
            second_offset,
        )
        .single();
    if candidate.is_none() {
        tracing::error!(
            year,
            month_offset,
            day,
            hour_offset,
            minute_offset,
            second_offset,
            "Could not construct candidate year marker"
        );
        panic!(
            "Could not construct candidate year marker for {year}/{month_offset}/{day} {hour_offset}:{minute_offset}:{second_offset}"
        );
    }
    let mut candidate = candidate.unwrap();

    if candidate <= now {
        // Move to next year, same month/day/hms, re-clamp day just in case.
        let next_year = year + 1;
        let start_of_next_year_month =
            chrono::naive::NaiveDate::from_ymd_opt(next_year, month_offset, 1).unwrap_or_else(
                || {
                    tracing::error!(
                        next_year,
                        month_offset,
                        "Could not create NaiveDate for next year/month"
                    );
                    panic!(
                        "Could not create NaiveDate for next year/month: {next_year}/{month_offset}"
                    );
                },
            );
        let next_next_month_start = if month_offset == 12 {
            chrono::naive::NaiveDate::from_ymd_opt(next_year + 1, 1, 1)
        } else {
            chrono::naive::NaiveDate::from_ymd_opt(next_year, month_offset + 1, 1)
        }
        .unwrap_or_else(|| {
            tracing::error!(
                next_year,
                month_offset,
                "Could not create NaiveDate for the month after"
            );
            panic!(
                "Could not create NaiveDate for following month: {}/{}",
                if month_offset == 12 {
                    next_year + 1
                } else {
                    next_year
                },
                if month_offset == 12 {
                    1
                } else {
                    month_offset + 1
                }
            );
        });
        let days_in_next_month = (next_next_month_start - start_of_next_year_month).num_days();
        let next_day = day_offset.min(days_in_next_month as u32);
        let candidate2 = chrono::Utc
            .with_ymd_and_hms(
                next_year,
                month_offset,
                next_day,
                hour_offset,
                minute_offset,
                second_offset,
            )
            .single();
        if candidate2.is_none() {
            tracing::error!(
                next_year,
                month_offset,
                next_day,
                hour_offset,
                minute_offset,
                second_offset,
                "Could not construct next year marker"
            );
            panic!(
                "Could not construct next year marker for {next_year}/{month_offset}/{next_day} {hour_offset}:{minute_offset}:{second_offset}"
            );
        }
        candidate = candidate2.unwrap();
    }

    Ok(candidate)
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
    let mut initialized: bool = false;
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

        // Calculate next run time: add one year, re-clamp the day to month
        let next_year = this_run_time.year() + 1;
        let month = this_run_time.month();
        let days_in_month = chrono::naive::NaiveDate::from_ymd_opt(next_year, month, 1)
            .ok_or_else(|| anyhow!("Invalid month in next year for next run"))?
            .with_day(1)
            .unwrap()
            .with_month((month % 12) + 1)
            .unwrap_or_else(|| chrono::naive::NaiveDate::from_ymd_opt(next_year + 1, 1, 1).unwrap())
            .signed_duration_since(
                chrono::naive::NaiveDate::from_ymd_opt(next_year, month, 1).unwrap(),
            )
            .num_days();
        let next_day = day_offset.min(days_in_month as u32).max(1);
        let next_run_time = chrono::Utc
            .with_ymd_and_hms(
                next_year,
                month,
                next_day,
                hour_offset,
                minute_offset,
                second_offset,
            )
            .single()
            .ok_or_else(|| anyhow!("Could not construct next year marker"))?;

        info!(
            task_name = %task_descriptor,
            next_run_time = %next_run_time.to_rfc3339_opts(SecondsFormat::AutoSi, true),
            duration=?elapsed,
            "Scheduled task ran! Next one running in {}",
            format_duration((next_run_time - Utc::now()).to_std().unwrap_or(std::time::Duration::from_secs(31_536_000)))
        );

        scheduled_run_time = Some(next_run_time);
    }
}
