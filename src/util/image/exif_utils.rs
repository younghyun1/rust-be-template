use anyhow::{Result, anyhow};
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use exif::{In, Tag};
use std::io::Cursor;
use tracing::{debug, warn};

pub fn extract_exif_shot_at(image_bytes: &[u8]) -> Result<Option<DateTime<Utc>>> {
    let mut cursor = Cursor::new(image_bytes);

    let exif_reader = match exif::Reader::new().read_from_container(&mut cursor) {
        Ok(r) => r,
        Err(e) => {
            debug!(error = %e, "Could not read standard EXIF container");
            return Ok(None);
        }
    };

    let field = exif_reader
        .get_field(Tag::DateTimeOriginal, In::PRIMARY)
        .or_else(|| exif_reader.get_field(Tag::DateTime, In::PRIMARY));

    let Some(field) = field else {
        warn!("EXIF present but no DateTime tags found");
        return Ok(None);
    };

    let raw = field.display_value().to_string();
    debug!(raw_exif_datetime = %raw, "Found raw EXIF metadata string");

    let clean_raw = raw
        .trim_matches('"')
        .replace('T', " ")
        .replace(['-', '/'], ":");

    // Now the string should look like "2024:11:27 23:39:00" regardless of input
    let parts: Vec<&str> = clean_raw.split_whitespace().collect();

    if parts.len() != 2 {
        warn!(
            clean_raw = %clean_raw,
            raw_exif_datetime = %raw,
            part_count = parts.len(),
            "Parsed datetime string does not have 2 parts"
        );
        return Ok(None);
    }

    let date_parts: Vec<&str> = parts[0].split(':').collect();
    let time_parts: Vec<&str> = parts[1].split(':').collect();

    if date_parts.len() != 3 || time_parts.len() != 3 {
        // This is where your error was happening.
        // Now that we replaced '-' with ':', date_parts.len() should be 3.
        warn!(
            date_part_count = date_parts.len(),
            time_part_count = time_parts.len(),
            clean_raw = %clean_raw,
            "EXIF datetime parts invalid structure"
        );
        return Ok(None);
    }

    // Attempt to parse numbers
    let year: i32 = date_parts[0].parse().map_err(|_| anyhow!("Invalid year"))?;
    let month: u32 = date_parts[1]
        .parse()
        .map_err(|_| anyhow!("Invalid month"))?;
    let day: u32 = date_parts[2].parse().map_err(|_| anyhow!("Invalid day"))?;
    let hour: u32 = time_parts[0].parse().map_err(|_| anyhow!("Invalid hour"))?;
    let minute: u32 = time_parts[1]
        .parse()
        .map_err(|_| anyhow!("Invalid minute"))?;
    let second: u32 = time_parts[2]
        .parse()
        .map_err(|_| anyhow!("Invalid second"))?;

    let date =
        NaiveDate::from_ymd_opt(year, month, day).ok_or_else(|| anyhow!("Date out of range"))?;
    let time = NaiveTime::from_hms_opt(hour, minute, second)
        .ok_or_else(|| anyhow!("Time out of range"))?;

    let naive_dt = NaiveDateTime::new(date, time);
    let dt_utc: DateTime<Utc> = Utc.from_utc_datetime(&naive_dt);

    Ok(Some(dt_utc))
}
