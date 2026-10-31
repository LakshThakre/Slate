use chrono::{Datelike, Duration, NaiveDate, Weekday};

use crate::config::WeekStart;

/// Calculates the start date of the week containing `date`, respecting `week_start`.
pub fn week_start_of(date: NaiveDate, week_start: WeekStart) -> NaiveDate {
    let offset = match week_start {
        WeekStart::Monday => date.weekday().num_days_from_monday(),
        WeekStart::Sunday => date.weekday().num_days_from_sunday(),
        WeekStart::Saturday => match date.weekday() {
            Weekday::Sat => 0,
            Weekday::Sun => 1,
            Weekday::Mon => 2,
            Weekday::Tue => 3,
            Weekday::Wed => 4,
            Weekday::Thu => 5,
            Weekday::Fri => 6,
        },
    };
    date - Duration::days(offset as i64)
}

/// Returns the 7 days of the week starting from `start`.
pub fn week_days(start: NaiveDate) -> [NaiveDate; 7] {
    [
        start,
        start + Duration::days(1),
        start + Duration::days(2),
        start + Duration::days(3),
        start + Duration::days(4),
        start + Duration::days(5),
        start + Duration::days(6),
    ]
}

/// Returns the weeks overlapping the given year and month (up to 6 rows).
/// Days outside the month are represented as `None`.
pub fn month_weeks(year: i32, month: u32, week_start: WeekStart) -> Vec<[Option<NaiveDate>; 7]> {
    let first_day = match NaiveDate::from_ymd_opt(year, month, 1) {
        Some(d) => d,
        None => return Vec::new(),
    };

    let next_month_first = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    };
    let last_day = match next_month_first {
        Some(d) => d.pred_opt().unwrap_or(first_day),
        None => first_day,
    };

    let mut weeks = Vec::new();
    let mut cur_week_start = week_start_of(first_day, week_start);

    while cur_week_start <= last_day {
        let days = week_days(cur_week_start);
        let mut row = [None; 7];
        for (i, d) in days.into_iter().enumerate() {
            if d.year() == year && d.month() == month {
                row[i] = Some(d);
            }
        }
        weeks.push(row);
        cur_week_start += Duration::days(7);
    }

    weeks
}

/// Returns the earliest and latest week start containing any task or `today`.
pub fn week_period_bounds(
    tasks_dates: impl Iterator<Item = NaiveDate>,
    today: NaiveDate,
    week_start: WeekStart,
) -> (NaiveDate, NaiveDate) {
    let mut min_date = today;
    let mut max_date = today;
    for d in tasks_dates {
        if d < min_date {
            min_date = d;
        }
        if d > max_date {
            max_date = d;
        }
    }
    (
        week_start_of(min_date, week_start),
        week_start_of(max_date, week_start),
    )
}

/// Returns the earliest and latest (year, month) containing any task or `today`.
pub fn month_period_bounds(
    tasks_dates: impl Iterator<Item = NaiveDate>,
    today: NaiveDate,
) -> ((i32, u32), (i32, u32)) {
    let mut min_date = today;
    let mut max_date = today;
    for d in tasks_dates {
        if d < min_date {
            min_date = d;
        }
        if d > max_date {
            max_date = d;
        }
    }
    (
        (min_date.year(), min_date.month()),
        (max_date.year(), max_date.month()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_week_start_of_all_starts_and_weekdays() {
        // 2026-10-26 is Monday, 2026-10-30 is Friday, 2026-10-31 is Saturday, 2026-11-01 is Sunday
        let mon = NaiveDate::from_ymd_opt(2026, 10, 26).unwrap();
        let fri = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        let sat = NaiveDate::from_ymd_opt(2026, 10, 31).unwrap();
        let sun = NaiveDate::from_ymd_opt(2026, 11, 1).unwrap();

        // Monday start
        assert_eq!(week_start_of(fri, WeekStart::Monday), mon);
        assert_eq!(week_start_of(mon, WeekStart::Monday), mon);
        assert_eq!(week_start_of(sun, WeekStart::Monday), mon);

        // Sunday start
        let sun_prev = NaiveDate::from_ymd_opt(2026, 10, 25).unwrap();
        assert_eq!(week_start_of(fri, WeekStart::Sunday), sun_prev);
        assert_eq!(week_start_of(mon, WeekStart::Sunday), sun_prev);
        assert_eq!(week_start_of(sun, WeekStart::Sunday), sun);

        // Saturday start
        let sat_prev = NaiveDate::from_ymd_opt(2026, 10, 24).unwrap();
        assert_eq!(week_start_of(fri, WeekStart::Saturday), sat_prev);
        assert_eq!(week_start_of(mon, WeekStart::Saturday), sat_prev);
        assert_eq!(week_start_of(sat, WeekStart::Saturday), sat);
    }

    #[test]
    fn test_week_start_year_boundary_and_leap_day() {
        // Jan 1, 2025 is Wednesday -> Monday week start is Dec 30, 2024
        let jan1_2025 = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        assert_eq!(
            week_start_of(jan1_2025, WeekStart::Monday),
            NaiveDate::from_ymd_opt(2024, 12, 30).unwrap()
        );

        // Leap day: 2024-02-29 is Thursday -> Monday start is 2024-02-26
        let leap_day = NaiveDate::from_ymd_opt(2024, 2, 29).unwrap();
        assert_eq!(
            week_start_of(leap_day, WeekStart::Monday),
            NaiveDate::from_ymd_opt(2024, 2, 26).unwrap()
        );
    }

    #[test]
    fn test_week_days() {
        let start = NaiveDate::from_ymd_opt(2026, 10, 26).unwrap();
        let days = week_days(start);
        assert_eq!(days.len(), 7);
        assert_eq!(days[0], start);
        assert_eq!(days[6], NaiveDate::from_ymd_opt(2026, 11, 1).unwrap());
    }

    #[test]
    fn test_month_weeks_row_counts_and_leap_years() {
        // February 2021: starts on Monday (Feb 1), exactly 28 days -> 4 weeks (with Monday start)
        let weeks_feb2021 = month_weeks(2021, 2, WeekStart::Monday);
        assert_eq!(weeks_feb2021.len(), 4);
        assert!(weeks_feb2021[0][0].is_some());
        assert!(weeks_feb2021[3][6].is_some());

        // February 2024 (leap year, 29 days): starts on Thursday -> 5 weeks
        let weeks_feb2024 = month_weeks(2024, 2, WeekStart::Monday);
        assert_eq!(weeks_feb2024.len(), 5);

        // August 2026 (31 days): starts on Saturday -> 6 weeks (with Monday start)
        let weeks_aug2026 = month_weeks(2026, 8, WeekStart::Monday);
        assert_eq!(weeks_aug2026.len(), 6);
        // First row Mon-Fri are None, Sat is Aug 1
        assert!(weeks_aug2026[0][0].is_none());
        assert_eq!(
            weeks_aug2026[0][5],
            Some(NaiveDate::from_ymd_opt(2026, 8, 1).unwrap())
        );

        // October 2026 (31 days): starts on Thursday -> 5 weeks
        let weeks_oct2026 = month_weeks(2026, 10, WeekStart::Monday);
        assert_eq!(weeks_oct2026.len(), 5);
    }

    #[test]
    fn test_period_bounds() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 31).unwrap();
        let tasks = [
            NaiveDate::from_ymd_opt(2026, 9, 15).unwrap(),
            NaiveDate::from_ymd_opt(2026, 11, 10).unwrap(),
        ];
        let (min_w, max_w) = week_period_bounds(tasks.iter().copied(), today, WeekStart::Monday);
        assert_eq!(min_w, NaiveDate::from_ymd_opt(2026, 9, 14).unwrap());
        assert_eq!(max_w, NaiveDate::from_ymd_opt(2026, 11, 9).unwrap());

        let (min_m, max_m) = month_period_bounds(tasks.iter().copied(), today);
        assert_eq!(min_m, (2026, 9));
        assert_eq!(max_m, (2026, 11));
    }
}
