use anyhow::{Result, anyhow};
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use exif::{In, Tag};
use std::io::Cursor;

/// Extract the shot-at datetime from EXIF metadata, if available.
///
/// This looks for `DateTimeOriginal` and falls back to `DateTime`.
/// It returns a `chrono::DateTime<Utc>` if parsing succeeds, or `Ok(None)`
/// if the image has no usable EXIF datetime or if parsing fails in a
/// non-fatal way.
///
/// Notes:
/// - Typical EXIF datetime format: "YYYY:MM:DD HH:MM:SS"
/// - EXIF usually has no timezone; we treat the value as UTC.
pub fn extract_exif_shot_at(image_bytes: &[u8]) -> Result<Option<DateTime<Utc>>> {
    // Wrap the bytes in a cursor for the EXIF reader
    let mut cursor = Cursor::new(image_bytes);

    // If there is no EXIF or it's unreadable, return Ok(None)
    let exif_reader = match exif::Reader::new().read_from_container(&mut cursor) {
        Ok(r) => r,
        Err(_) => return Ok(None),
    };

    // Prefer DateTimeOriginal; fall back to DateTime if missing
    let field = exif_reader
        .get_field(Tag::DateTimeOriginal, In::PRIMARY)
        .or_else(|| exif_reader.get_field(Tag::DateTime, In::PRIMARY));

    let Some(field) = field else {
        return Ok(None);
    };

    // Typical EXIF datetime format: "YYYY:MM:DD HH:MM:SS"
    let raw = field.display_value().to_string();
    let parts: Vec<&str> = raw.split_whitespace().collect();
    if parts.len() != 2 {
        return Ok(None);
    }

    let date_parts: Vec<&str> = parts[0].split(':').collect();
    let time_parts: Vec<&str> = parts[1].split(':').collect();
    if date_parts.len() != 3 || time_parts.len() != 3 {
        return Ok(None);
    }

    let year: i32 = date_parts[0]
        .parse()
        .map_err(|_| anyhow!("Invalid EXIF year: {}", date_parts[0]))?;
    let month: u32 = date_parts[1]
        .parse()
        .map_err(|_| anyhow!("Invalid EXIF month: {}", date_parts[1]))?;
    let day: u32 = date_parts[2]
        .parse()
        .map_err(|_| anyhow!("Invalid EXIF day: {}", date_parts[2]))?;

    let hour: u32 = time_parts[0]
        .parse()
        .map_err(|_| anyhow!("Invalid EXIF hour: {}", time_parts[0]))?;
    let minute: u32 = time_parts[1]
        .parse()
        .map_err(|_| anyhow!("Invalid EXIF minute: {}", time_parts[1]))?;
    let second: u32 = time_parts[2]
        .parse()
        .map_err(|_| anyhow!("Invalid EXIF second: {}", time_parts[2]))?;

    // Build NaiveDate and NaiveTime
    let date = NaiveDate::from_ymd_opt(year, month, day)
        .ok_or_else(|| anyhow!("EXIF date out of range: {year}-{month}-{day}"))?;
    let time = NaiveTime::from_hms_opt(hour, minute, second)
        .ok_or_else(|| anyhow!("EXIF time out of range: {hour}:{minute}:{second}"))?;

    let naive_dt = NaiveDateTime::new(date, time);

    // EXIF doesn't store timezone normally; we treat it as UTC.
    let dt_utc: DateTime<Utc> = Utc.from_utc_datetime(&naive_dt);

    Ok(Some(dt_utc))
}
