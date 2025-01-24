use chrono::{DateTime, Utc};
use tokio::time::Duration;

const SECONDS_PER_MINUTE: u64 = 60;
const SECONDS_PER_HOUR: u64 = 3_600;
const SECONDS_PER_DAY: u64 = 86_400;
const SECONDS_PER_YEAR: u64 = 31_536_000;

// std/tokio::time::Duration
pub fn format_duration(duration: Duration) -> String {
    let mut total_secs = duration.as_secs();
    let millis = duration.subsec_millis() as u64;

    let years = total_secs / SECONDS_PER_YEAR;
    total_secs %= SECONDS_PER_YEAR;

    let days = total_secs / SECONDS_PER_DAY;
    total_secs %= SECONDS_PER_DAY;

    let hours = total_secs / SECONDS_PER_HOUR;
    total_secs %= SECONDS_PER_HOUR;

    let minutes = total_secs / SECONDS_PER_MINUTE;
    total_secs %= SECONDS_PER_MINUTE;

    let seconds = total_secs;

    let mut parts = Vec::new();

    if years > 0 {
        parts.push(format!(
            "{} year{}",
            years,
            if years == 1 { "" } else { "s" }
        ));
    }

    if days > 0 {
        parts.push(format!("{} day{}", days, if days == 1 { "" } else { "s" }));
    }

    if hours > 0 {
        parts.push(format!(
            "{} hour{}",
            hours,
            if hours == 1 { "" } else { "s" }
        ));
    }

    if minutes > 0 {
        parts.push(format!(
            "{} minute{}",
            minutes,
            if minutes == 1 { "" } else { "s" }
        ));
    }

    if seconds > 0 {
        parts.push(format!(
            "{} second{}",
            seconds,
            if seconds == 1 { "" } else { "s" }
        ));
    }

    if millis > 0 {
        parts.push(format!(
            "{} millisecond{}",
            millis,
            if millis == 1 { "" } else { "s" }
        ));
    }

    if parts.is_empty() {
        return "0 milliseconds".to_string();
    }

    parts.join(", ")
}

// chrono::TimeDelta
pub fn format_dt_difference(start_time: DateTime<Utc>, end_time: DateTime<Utc>) -> String {
    let differential = end_time - start_time;

    format!(
        "{} days, {} hours, {} minutes, {} seconds, {} milliseconds",
        differential.num_days(),
        differential.num_hours() % 24,
        differential.num_minutes() % 60,
        differential.num_seconds() % 60,
        differential.num_milliseconds() % 1000,
    )
}
