/// Manual date computation from Unix epoch.
/// Intentionally avoids adding a `chrono` dependency for this simple use case.
pub(super) fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

pub(super) fn date_parts_from_secs(now: u64) -> (i32, usize, u64, u64) {
    let mut remaining_days = now / 86400;
    let mut year = 1970i32;

    loop {
        let year_days: u64 = if is_leap_year(year) { 366 } else { 365 };
        if remaining_days < year_days {
            break;
        }
        remaining_days -= year_days;
        let Some(next_year) = year.checked_add(1) else {
            break;
        };
        year = next_year;
    }

    let month_lengths = [
        31u64,
        if is_leap_year(year) { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];

    let mut month = 0usize;
    for &days_in_month in &month_lengths {
        if remaining_days < days_in_month {
            break;
        }
        remaining_days -= days_in_month;
        month += 1;
    }

    (year, month + 1, remaining_days + 1, now % 86400)
}

pub(super) fn current_date_parts() -> (i32, usize, u64, u64) {
    let now = current_unix_seconds();
    date_parts_from_secs(now)
}

pub(super) fn current_unix_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub(super) fn unix_seconds_from_timestamp(value: &str) -> Option<u64> {
    let (year, month, day, hour, minute, second, offset_seconds) = match value.len() {
        10 if value.as_bytes().get(4) == Some(&b'-') && value.as_bytes().get(7) == Some(&b'-') => (
            value[0..4].parse::<i64>().ok()?,
            value[5..7].parse::<i64>().ok()?,
            value[8..10].parse::<i64>().ok()?,
            0,
            0,
            0,
            0,
        ),
        20 if value.as_bytes().get(4) == Some(&b'-')
            && value.as_bytes().get(7) == Some(&b'-')
            && value.as_bytes().get(10) == Some(&b'T')
            && value.as_bytes().get(13) == Some(&b':')
            && value.as_bytes().get(16) == Some(&b':')
            && value.as_bytes().get(19) == Some(&b'Z') =>
        {
            (
                value[0..4].parse::<i64>().ok()?,
                value[5..7].parse::<i64>().ok()?,
                value[8..10].parse::<i64>().ok()?,
                value[11..13].parse::<i64>().ok()?,
                value[14..16].parse::<i64>().ok()?,
                value[17..19].parse::<i64>().ok()?,
                0,
            )
        }
        25 if value.as_bytes().get(4) == Some(&b'-')
            && value.as_bytes().get(7) == Some(&b'-')
            && value.as_bytes().get(10) == Some(&b'T')
            && value.as_bytes().get(13) == Some(&b':')
            && value.as_bytes().get(16) == Some(&b':')
            && value
                .as_bytes()
                .get(19)
                .is_some_and(|b| *b == b'+' || *b == b'-')
            && value.as_bytes().get(22) == Some(&b':') =>
        {
            let sign = if value.as_bytes()[19] == b'+' { 1 } else { -1 };
            let offset_hour = value[20..22].parse::<i64>().ok()?;
            let offset_minute = value[23..25].parse::<i64>().ok()?;
            (
                value[0..4].parse::<i64>().ok()?,
                value[5..7].parse::<i64>().ok()?,
                value[8..10].parse::<i64>().ok()?,
                value[11..13].parse::<i64>().ok()?,
                value[14..16].parse::<i64>().ok()?,
                value[17..19].parse::<i64>().ok()?,
                sign * (offset_hour * 3600 + offset_minute * 60),
            )
        }
        _ => return None,
    };

    if !(1..=9999).contains(&year)
        || !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || hour > 23
        || minute > 59
        || second > 59
    {
        return None;
    }

    let adjusted_year = year - i64::from(month <= 2);
    let era = if adjusted_year >= 0 {
        adjusted_year / 400
    } else {
        (adjusted_year - 399) / 400
    };
    let year_of_era = adjusted_year - era * 400;
    let month_prime = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * month_prime + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days_since_epoch = era * 146097 + day_of_era - 719468;
    let seconds = days_since_epoch * 86400 + hour * 3600 + minute * 60 + second - offset_seconds;
    Some(seconds.max(0) as u64)
}

#[cfg(test)]
pub(super) fn timestamp_iso8601_from_secs(secs: u64) -> String {
    let (year, month, day, day_secs) = date_parts_from_secs(secs);
    let hours = day_secs / 3600;
    let minutes = (day_secs % 3600) / 60;
    let seconds = day_secs % 60;
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hours, minutes, seconds
    )
}

#[cfg(test)]
pub(super) fn current_date_iso() -> String {
    let (year, month, day, _) = current_date_parts();
    format!("{:04}-{:02}-{:02}", year, month, day)
}

pub(crate) fn current_timestamp_iso8601() -> String {
    let (year, month, day, day_secs) = current_date_parts();
    let hours = day_secs / 3600;
    let minutes = (day_secs % 3600) / 60;
    let seconds = day_secs % 60;

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hours, minutes, seconds
    )
}

#[cfg(test)]
mod tests {
    use super::unix_seconds_from_timestamp;

    #[test]
    fn unix_timestamp_conversion_matches_supported_notice_formats() {
        assert_eq!(unix_seconds_from_timestamp("1970-01-01"), Some(0));
        assert_eq!(
            unix_seconds_from_timestamp("2025-01-01T00:00:00Z"),
            Some(1_735_689_600)
        );
        assert_eq!(
            unix_seconds_from_timestamp("2025-01-01T05:30:00+05:30"),
            Some(1_735_689_600)
        );
        assert_eq!(unix_seconds_from_timestamp("not-a-timestamp"), None);
    }
}
