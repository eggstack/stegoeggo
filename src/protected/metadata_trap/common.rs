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
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    date_parts_from_secs(now)
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
