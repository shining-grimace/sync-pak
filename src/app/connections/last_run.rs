use chrono::{DateTime, Datelike, Local, TimeZone};

pub(crate) fn label(timestamp: Option<u64>) -> String {
    timestamp
        .and_then(|seconds| i64::try_from(seconds).ok())
        .and_then(|seconds| Local.timestamp_opt(seconds, 0).single())
        .map(|time| format!("Last run {}", display_time(time)))
        .unwrap_or_else(|| "Not run yet".into())
}

fn display_time<T: TimeZone>(time: DateTime<T>) -> String
where
    T::Offset: std::fmt::Display,
{
    let day = time.day();
    let suffix = match (day % 100, day % 10) {
        (11..=13, _) => "th",
        (_, 1) => "st",
        (_, 2) => "nd",
        (_, 3) => "rd",
        _ => "th",
    };
    format!(
        "{} {day}{suffix}, {}, at {}",
        time.format("%B"),
        time.format("%Y"),
        time.format("%-I:%M %p"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn formats_the_requested_date_and_twelve_hour_time() {
        let time = Utc.with_ymd_and_hms(2026, 10, 25, 16, 13, 0).unwrap();
        assert_eq!(display_time(time), "October 25th, 2026, at 4:13 PM");
        assert_eq!(label(None), "Not run yet");
    }

    #[test]
    fn formats_ordinals_and_midnight_and_noon() {
        for (day, suffix) in [
            (1, "st"),
            (2, "nd"),
            (3, "rd"),
            (11, "th"),
            (12, "th"),
            (13, "th"),
            (21, "st"),
            (22, "nd"),
            (23, "rd"),
            (31, "st"),
        ] {
            let time = Utc.with_ymd_and_hms(2026, 10, day, 0, 5, 0).unwrap();
            assert_eq!(
                display_time(time),
                format!("October {day}{suffix}, 2026, at 12:05 AM")
            );
        }
        let noon = Utc.with_ymd_and_hms(2026, 10, 25, 12, 0, 0).unwrap();
        assert_eq!(display_time(noon), "October 25th, 2026, at 12:00 PM");
    }
}
