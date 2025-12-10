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
            debug!("Could not read standard EXIF container: {}", e);
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
    debug!("Found raw EXIF/Metadata string: '{}'", raw);

    let clean_raw = raw
        .trim_matches('"')
        .replace('T', " ")
        .replace(['-', '/'], ":");

    // Now the string should look like "2024:11:27 23:39:00" regardless of input
    let parts: Vec<&str> = clean_raw.split_whitespace().collect();

    if parts.len() != 2 {
        warn!(
            "Parsed datetime string '{}' (from '{}') does not have 2 parts",
            clean_raw, raw
        );
        return Ok(None);
    }

    let date_parts: Vec<&str> = parts[0].split(':').collect();
    let time_parts: Vec<&str> = parts[1].split(':').collect();

    if date_parts.len() != 3 || time_parts.len() != 3 {
        // This is where your error was happening.
        // Now that we replaced '-' with ':', date_parts.len() should be 3.
        warn!(
            "EXIF datetime parts invalid structure: date_parts len={}, time_parts len={} (raw: {})",
            date_parts.len(),
            time_parts.len(),
            clean_raw
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
