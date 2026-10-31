use crate::model::{RecurrenceRule, Todo};
use chrono::{Datelike, Duration, NaiveDate, Weekday};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parsed {
    pub text: String,
    pub date: Option<NaiveDate>,
    pub priority: u8,
    pub tags: Vec<String>,
    pub recurrence: Option<RecurrenceRule>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    EmptyText,
    DateInPast(NaiveDate),
    ConflictingDates(NaiveDate, NaiveDate),
    RecurrenceDateConflict(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::EmptyText => write!(f, "task text cannot be empty"),
            ParseError::DateInPast(d) => write!(f, "date '{d}' cannot be in the past"),
            ParseError::ConflictingDates(d1, d2) => {
                write!(f, "conflicting dates: '{d1}' and '{d2}'")
            }
            ParseError::RecurrenceDateConflict(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for ParseError {}

/// Calculates the next occurrence of `target` weekday strictly after `today` (1..=7 days ahead).
pub fn next_weekday(today: NaiveDate, target: Weekday) -> NaiveDate {
    let cur = today.weekday().number_from_monday();
    let tgt = target.number_from_monday();
    let days = if tgt > cur { tgt - cur } else { tgt + 7 - cur };
    today + Duration::days(days as i64)
}

/// Calculates the previous occurrence of `target` weekday strictly before `today` (1..=7 days back).
pub fn prev_weekday(today: NaiveDate, target: Weekday) -> NaiveDate {
    let cur = today.weekday().number_from_monday();
    let tgt = target.number_from_monday();
    let days = if cur > tgt { cur - tgt } else { cur + 7 - tgt };
    today - Duration::days(days as i64)
}

/// Parses weekday from string name (case-insensitive, 3-char abbrev or full name).
fn parse_weekday(s: &str) -> Option<Weekday> {
    match s.to_ascii_lowercase().as_str() {
        "mon" | "monday" => Some(Weekday::Mon),
        "tue" | "tuesday" => Some(Weekday::Tue),
        "wed" | "wednesday" => Some(Weekday::Wed),
        "thu" | "thursday" => Some(Weekday::Thu),
        "fri" | "friday" => Some(Weekday::Fri),
        "sat" | "saturday" => Some(Weekday::Sat),
        "sun" | "sunday" => Some(Weekday::Sun),
        _ => None,
    }
}

/// Parses next occurrence of MM-DD relative to `today`.
fn parse_mm_dd(s: &str, today: NaiveDate) -> Option<NaiveDate> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 2 {
        return None;
    }
    let month: u32 = parts[0].parse().ok()?;
    let day: u32 = parts[1].parse().ok()?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }

    // Try current year
    if let Some(d) = NaiveDate::from_ymd_opt(today.year(), month, day) {
        if d >= today {
            return Some(d);
        }
    }

    // Try future years (handles leap year if Feb 29)
    for offset in 1..=4 {
        if let Some(d) = NaiveDate::from_ymd_opt(today.year() + offset, month, day) {
            return Some(d);
        }
    }

    None
}

/// Parses relative date offset like +3d or +2w (or -3d, -2w when allow_past is true; N from 1 to 3650).
fn parse_relative_offset_ext(s: &str, today: NaiveDate, allow_past: bool) -> Option<NaiveDate> {
    let (is_negative, rest) = if let Some(stripped) = s.strip_prefix('+') {
        (false, stripped)
    } else if allow_past {
        if let Some(stripped) = s.strip_prefix('-') {
            (true, stripped)
        } else {
            return None;
        }
    } else {
        return None;
    };

    if rest.len() < 2 {
        return None;
    }
    let unit = rest.chars().last()?.to_ascii_lowercase();
    let num_str = &rest[..rest.len() - 1];
    let n: i64 = num_str.parse().ok()?;
    let max_n = if allow_past { 3650 } else { 365 };
    if !(1..=max_n).contains(&n) {
        return None;
    }

    let days = match unit {
        'd' => n,
        'w' => n.checked_mul(7)?,
        _ => return None,
    };

    if is_negative {
        today.checked_sub_signed(Duration::days(days))
    } else {
        today.checked_add_signed(Duration::days(days))
    }
}

/// Parses relative date offset like +3d or +2w (N from 1 to 365).
fn parse_relative_offset(s: &str, today: NaiveDate) -> Option<NaiveDate> {
    parse_relative_offset_ext(s, today, false)
}

/// Checks if string matches date expression and parses it (reschedule mode, no past dates).
pub fn parse_date_expression(expr: &str, today: NaiveDate) -> Result<NaiveDate, String> {
    parse_date_expression_ext(expr, today, false)
}

/// Extended date expression parser.
/// When `allow_past` is true:
/// - Allows `yesterday`, `-Nd`, `-Nw` (N up to 3650), and past `YYYY-MM-DD`.
/// - `MM-DD` uses the current year (`today.year()`).
/// - Bare weekday names mean the most recent past occurrence (strictly before `today`).
pub fn parse_date_expression_ext(
    expr: &str,
    today: NaiveDate,
    allow_past: bool,
) -> Result<NaiveDate, String> {
    let s = expr.trim();
    let s = s.strip_prefix('@').unwrap_or(s);
    let s_lower = s.to_ascii_lowercase();

    if s_lower == "today" {
        return Ok(today);
    }
    if s_lower == "yesterday" {
        if allow_past {
            return today
                .pred_opt()
                .ok_or_else(|| "date underflow for yesterday".to_string());
        } else {
            return Err("date 'yesterday' cannot be in the past".to_string());
        }
    }
    if s_lower == "tomorrow" {
        return today
            .succ_opt()
            .ok_or_else(|| "date overflow for tomorrow".to_string());
    }
    if let Some(w) = parse_weekday(&s_lower) {
        if allow_past {
            return Ok(prev_weekday(today, w));
        } else {
            return Ok(next_weekday(today, w));
        }
    }
    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        if !allow_past && d < today {
            return Err(format!("date '{d}' cannot be in the past"));
        }
        return Ok(d);
    }
    if allow_past {
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() == 2 {
            if let (Ok(month), Ok(day)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                if (1..=12).contains(&month) && (1..=31).contains(&day) {
                    if let Some(d) = NaiveDate::from_ymd_opt(today.year(), month, day) {
                        return Ok(d);
                    } else {
                        return Err(format!("invalid date '{s}' for year {}", today.year()));
                    }
                }
            }
        }
    } else if let Some(d) = parse_mm_dd(s, today) {
        if d < today {
            return Err(format!("date '{d}' cannot be in the past"));
        }
        return Ok(d);
    }
    if let Some(d) = parse_relative_offset_ext(s, today, allow_past) {
        return Ok(d);
    }

    Err(format!("invalid date expression '{expr}'"))
}

/// Tries to parse a word as an `@` date token.
/// Returns:
/// - `Some(Ok(date))` if recognized date
/// - `Some(Err(ParseError::DateInPast))` if date is before today
/// - `None` if not a recognized date token (e.g. `@unknown`, or `@daily`)
fn try_parse_at_date(word: &str, today: NaiveDate) -> Option<Result<NaiveDate, ParseError>> {
    if !word.starts_with('@') {
        return None;
    }
    let s = &word[1..];
    let s_lower = s.to_ascii_lowercase();

    // Recurrence tokens are not dates
    if matches!(s_lower.as_str(), "daily" | "weekdays" | "weekly") {
        return None;
    }

    if s_lower == "today" {
        return Some(Ok(today));
    }
    if s_lower == "tomorrow" {
        return Some(today.succ_opt().ok_or(ParseError::DateInPast(today)));
    }
    if let Some(w) = parse_weekday(&s_lower) {
        return Some(Ok(next_weekday(today, w)));
    }
    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        if d < today {
            return Some(Err(ParseError::DateInPast(d)));
        }
        return Some(Ok(d));
    }
    if let Some(d) = parse_mm_dd(s, today) {
        if d < today {
            return Some(Err(ParseError::DateInPast(d)));
        }
        return Some(Ok(d));
    }
    if let Some(d) = parse_relative_offset(s, today) {
        return Some(Ok(d));
    }

    None
}

/// Tries to parse a word as an `@` recurrence token.
fn try_parse_at_recurrence(word: &str) -> Option<RecurrenceRule> {
    if !word.starts_with('@') {
        return None;
    }
    let s = &word[1..];
    match s.to_ascii_lowercase().as_str() {
        "daily" => Some(RecurrenceRule::Daily),
        "weekdays" => Some(RecurrenceRule::Weekdays),
        "weekly" => Some(RecurrenceRule::Weekly),
        _ => None,
    }
}

/// Checks if a word is a valid `#tag` token.
/// First char after `#` is a letter, subsequent chars are alphanumeric, `_`, or `-`.
/// Max 32 chars.
fn try_parse_tag(word: &str) -> Option<String> {
    if !word.starts_with('#') {
        return None;
    }
    let name = &word[1..];
    if name.is_empty() || name.chars().count() > 32 {
        return None;
    }
    let mut chars = name.chars();
    let first = chars.next()?;
    if !first.is_alphabetic() {
        return None;
    }
    if !chars.all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
        return None;
    }
    Some(name.to_lowercase())
}

/// Shared quick-add parser:
/// `parse_quick_add(input, today) -> Result<Parsed, ParseError>`
pub fn parse_quick_add(input: &str, today: NaiveDate) -> Result<Parsed, ParseError> {
    let mut text_words = Vec::new();
    let mut date: Option<NaiveDate> = None;
    let mut priority: u8 = 0;
    let mut tags: Vec<String> = Vec::new();
    let mut recurrence: Option<RecurrenceRule> = None;

    for word in input.split_whitespace() {
        // Check for literal escape: e.g. \@fri, \#tag, \!, \!!
        if let Some(escaped) = word.strip_prefix('\\') {
            if escaped.starts_with('@')
                || escaped.starts_with('#')
                || escaped == "!"
                || escaped == "!!"
            {
                text_words.push(escaped);
                continue;
            }
        }

        // 1. Priority: exactly "!" or "!!"
        if word == "!" {
            priority = priority.max(1);
            continue;
        }
        if word == "!!" {
            priority = 2;
            continue;
        }

        // 2. Tag: #name
        if let Some(tag) = try_parse_tag(word) {
            if tags.len() < 5 && !tags.contains(&tag) {
                tags.push(tag);
            }
            continue;
        }

        // 3. Recurrence: @daily, @weekdays, @weekly
        if let Some(rec) = try_parse_at_recurrence(word) {
            recurrence = Some(rec);
            continue;
        }

        // 4. Date: @today, @tomorrow, @fri, @2026-10-31, @+3d, etc.
        if let Some(res) = try_parse_at_date(word, today) {
            let parsed_d = res?;
            if let Some(existing) = date {
                if existing != parsed_d {
                    return Err(ParseError::ConflictingDates(existing, parsed_d));
                }
            } else {
                date = Some(parsed_d);
            }
            continue;
        }

        // Otherwise, plain text word (preserves unknown @words like @alice or a@b.com or #12)
        text_words.push(word);
    }

    if text_words.is_empty() {
        return Err(ParseError::EmptyText);
    }

    // Validate recurrence vs date conflicts:
    // "recurrence implies the start date is the next matching day, so @fri @weekly is allowed
    // but @daily @fri is not: say so clearly"
    if let Some(ref rec) = recurrence {
        if let Some(d) = date {
            match rec {
                RecurrenceRule::Weekly => {
                    // Allowed: @fri @weekly repeats weekly on that weekday
                }
                RecurrenceRule::Daily => {
                    return Err(ParseError::RecurrenceDateConflict(format!(
                        "date + recurrence conflict: @daily implies the start date; cannot combine with specific date '{d}'"
                    )));
                }
                RecurrenceRule::Weekdays => {
                    return Err(ParseError::RecurrenceDateConflict(format!(
                        "date + recurrence conflict: @weekdays implies the start date; cannot combine with specific date '{d}'"
                    )));
                }
            }
        }
    }

    Ok(Parsed {
        text: text_words.join(" "),
        date,
        priority,
        tags,
        recurrence,
    })
}

/// Re-serializes a todo's metadata as quick-add tokens for the edit prompt (`e`).
pub fn re_serialize_tokens(todo: &Todo, today: NaiveDate) -> String {
    let mut parts = vec![todo.text.clone()];

    // Priority
    match todo.priority {
        1 => parts.push("!".to_string()),
        2 => parts.push("!!".to_string()),
        _ => {}
    }

    // Tags
    for tag in &todo.tags {
        parts.push(format!("#{tag}"));
    }

    // Date (only if future top-level task)
    if todo.parent.is_none() && todo.created_date > today {
        parts.push(format!("@{}", todo.created_date));
    }

    parts.join(" ")
}

/// Parses filter prompt input: space-separated `#tag` words (all must match) and/or `!` (priority >= 1) or `!!` (priority >= 2).
/// Empty input clears the filter.
pub fn parse_filter(input: &str) -> crate::model::Filter {
    let mut tags = Vec::new();
    let mut min_priority = 0;

    for word in input.split_whitespace() {
        if word == "!" {
            min_priority = min_priority.max(1);
        } else if word == "!!" {
            min_priority = 2;
        } else {
            let tag = word.strip_prefix('#').unwrap_or(word).to_lowercase();
            if !tag.is_empty() && !tags.contains(&tag) {
                tags.push(tag);
            }
        }
    }

    crate::model::Filter { tags, min_priority }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plain_task() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        let p = parse_quick_add("Buy groceries", today).unwrap();
        assert_eq!(p.text, "Buy groceries");
        assert_eq!(p.date, None);
        assert_eq!(p.priority, 0);
        assert!(p.tags.is_empty());
        assert_eq!(p.recurrence, None);
    }

    #[test]
    fn test_whitespace_collapsing() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        let p = parse_quick_add("   buy    groceries   and   milk   ", today).unwrap();
        assert_eq!(p.text, "buy groceries and milk");
    }

    #[test]
    fn test_priority_parsing() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        let p1 = parse_quick_add("task !", today).unwrap();
        assert_eq!(p1.priority, 1);
        assert_eq!(p1.text, "task");

        let p2 = parse_quick_add("! task !!", today).unwrap();
        assert_eq!(p2.priority, 2);
        assert_eq!(p2.text, "task");

        let p_not = parse_quick_add("Great! task", today).unwrap();
        assert_eq!(p_not.priority, 0);
        assert_eq!(p_not.text, "Great! task");
    }

    #[test]
    fn test_tag_parsing() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        let p = parse_quick_add("task #work #Home #work_stuff #12 #invalid-!?", today).unwrap();
        assert_eq!(p.tags, vec!["work", "home", "work_stuff"]);
        // #12 and #invalid-!? stay as text
        assert_eq!(p.text, "task #12 #invalid-!?");
    }

    #[test]
    fn test_tag_dedup_and_max_5() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        let p = parse_quick_add("task #a #b #c #d #e #f #a", today).unwrap();
        assert_eq!(p.tags, vec!["a", "b", "c", "d", "e"]);
    }

    #[test]
    fn test_dates_today_tomorrow_weekday() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap(); // Friday
        let p_today = parse_quick_add("task @today", today).unwrap();
        assert_eq!(p_today.date, Some(today));

        let p_tom = parse_quick_add("task @tomorrow", today).unwrap();
        assert_eq!(p_tom.date, Some(today + Duration::days(1)));

        // Today is Friday (2026-10-30). Next Friday is +7 days (2026-11-06)
        let p_fri = parse_quick_add("task @fri", today).unwrap();
        assert_eq!(
            p_fri.date,
            Some(NaiveDate::from_ymd_opt(2026, 11, 6).unwrap())
        );

        // Next Saturday is +1 day (2026-10-31)
        let p_sat = parse_quick_add("task @saturday", today).unwrap();
        assert_eq!(
            p_sat.date,
            Some(NaiveDate::from_ymd_opt(2026, 10, 31).unwrap())
        );
    }

    #[test]
    fn test_dates_iso_and_relative() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        let p_iso = parse_quick_add("task @2026-11-15", today).unwrap();
        assert_eq!(
            p_iso.date,
            Some(NaiveDate::from_ymd_opt(2026, 11, 15).unwrap())
        );

        let p_rel_d = parse_quick_add("task @+3d", today).unwrap();
        assert_eq!(p_rel_d.date, Some(today + Duration::days(3)));

        let p_rel_w = parse_quick_add("task @+2w", today).unwrap();
        assert_eq!(p_rel_w.date, Some(today + Duration::days(14)));
    }

    #[test]
    fn test_dates_mm_dd() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        let p1 = parse_quick_add("task @11-05", today).unwrap();
        assert_eq!(p1.date, Some(NaiveDate::from_ymd_opt(2026, 11, 5).unwrap()));

        // Already passed this year (Oct 15) -> rolls to next year
        let p2 = parse_quick_add("task @10-15", today).unwrap();
        assert_eq!(
            p2.date,
            Some(NaiveDate::from_ymd_opt(2027, 10, 15).unwrap())
        );
    }

    #[test]
    fn test_date_in_past_error() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        let err = parse_quick_add("task @2026-10-25", today).unwrap_err();
        assert!(matches!(err, ParseError::DateInPast(_)));
    }

    #[test]
    fn test_conflicting_dates_error() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        let err = parse_quick_add("task @tomorrow @+5d", today).unwrap_err();
        assert!(matches!(err, ParseError::ConflictingDates(_, _)));
    }

    #[test]
    fn test_literal_escapes_and_unknown() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        let p = parse_quick_add(r"task \@fri \#work \! @unknown user@domain.com", today).unwrap();
        assert_eq!(p.text, "task @fri #work ! @unknown user@domain.com");
        assert_eq!(p.date, None);
        assert_eq!(p.priority, 0);
        assert!(p.tags.is_empty());
    }

    #[test]
    fn test_empty_text_error() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        let err = parse_quick_add("@today ! #work", today).unwrap_err();
        assert_eq!(err, ParseError::EmptyText);
    }

    #[test]
    fn test_recurrence_and_conflicts() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        // @fri @weekly is allowed
        let p = parse_quick_add("task @fri @weekly", today).unwrap();
        assert_eq!(p.recurrence, Some(RecurrenceRule::Weekly));
        assert!(p.date.is_some());

        // @daily @fri is rejected
        let err = parse_quick_add("task @daily @fri", today).unwrap_err();
        assert!(matches!(err, ParseError::RecurrenceDateConflict(_)));

        // @weekdays @tomorrow is rejected
        let err2 = parse_quick_add("task @weekdays @tomorrow", today).unwrap_err();
        assert!(matches!(err2, ParseError::RecurrenceDateConflict(_)));
    }

    #[test]
    fn test_re_serialize_tokens() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        let todo = Todo {
            id: 1,
            text: "Buy groceries".to_string(),
            done: false,
            created_date: NaiveDate::from_ymd_opt(2026, 11, 5).unwrap(),
            priority: 2,
            tags: vec!["home".to_string(), "food".to_string()],
            ..Default::default()
        };
        let s = re_serialize_tokens(&todo, today);
        assert_eq!(s, "Buy groceries !! #home #food @2026-11-05");
    }

    #[test]
    fn test_parse_filter() {
        let f_empty = parse_filter("   ");
        assert!(f_empty.is_empty());

        let f_tags = parse_filter("#work #HOME #work");
        assert_eq!(f_tags.tags, vec!["work", "home"]);
        assert_eq!(f_tags.min_priority, 0);
        assert_eq!(f_tags.format_header(), "filter: #work #home");

        let f_prio1 = parse_filter("#work !");
        assert_eq!(f_prio1.tags, vec!["work"]);
        assert_eq!(f_prio1.min_priority, 1);
        assert_eq!(f_prio1.format_header(), "filter: #work !");

        let f_prio2 = parse_filter("!! #urgent");
        assert_eq!(f_prio2.tags, vec!["urgent"]);
        assert_eq!(f_prio2.min_priority, 2);
        assert_eq!(f_prio2.format_header(), "filter: #urgent !!");
    }

    #[test]
    fn test_parse_date_expression_ext_jump_forms() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap(); // Friday

        // today, yesterday, tomorrow
        assert_eq!(
            parse_date_expression_ext("today", today, true).unwrap(),
            today
        );
        assert_eq!(
            parse_date_expression_ext("yesterday", today, true).unwrap(),
            today - Duration::days(1)
        );
        assert_eq!(
            parse_date_expression_ext("tomorrow", today, true).unwrap(),
            today + Duration::days(1)
        );

        // ISO format (past and future)
        let past_iso = NaiveDate::from_ymd_opt(2025, 4, 12).unwrap();
        assert_eq!(
            parse_date_expression_ext("2025-04-12", today, true).unwrap(),
            past_iso
        );
        // But with allow_past=false, past ISO fails:
        assert!(parse_date_expression_ext("2025-04-12", today, false).is_err());

        // MM-DD with allow_past=true uses current year (2026)
        let mm_dd = parse_date_expression_ext("05-15", today, true).unwrap();
        assert_eq!(mm_dd, NaiveDate::from_ymd_opt(2026, 5, 15).unwrap());

        // Relative offsets +Nd, -Nd, +Nw, -Nw
        assert_eq!(
            parse_date_expression_ext("+3d", today, true).unwrap(),
            today + Duration::days(3)
        );
        assert_eq!(
            parse_date_expression_ext("-5d", today, true).unwrap(),
            today - Duration::days(5)
        );
        assert_eq!(
            parse_date_expression_ext("+2w", today, true).unwrap(),
            today + Duration::days(14)
        );
        assert_eq!(
            parse_date_expression_ext("-3w", today, true).unwrap(),
            today - Duration::days(21)
        );
        assert_eq!(
            parse_date_expression_ext("-3650d", today, true).unwrap(),
            today - Duration::days(3650)
        );

        // Bare weekday names in jump (allow_past = true) mean MOST RECENT PAST occurrence (strictly before today)
        // Today is Friday (2026-10-30)
        // "fri" -> last Friday (2026-10-23, 7 days ago)
        assert_eq!(
            parse_date_expression_ext("fri", today, true).unwrap(),
            NaiveDate::from_ymd_opt(2026, 10, 23).unwrap()
        );
        // "thu" -> yesterday (2026-10-29, 1 day ago)
        assert_eq!(
            parse_date_expression_ext("thursday", today, true).unwrap(),
            NaiveDate::from_ymd_opt(2026, 10, 29).unwrap()
        );
        // "sat" -> last Saturday (2026-10-24, 6 days ago)
        assert_eq!(
            parse_date_expression_ext("sat", today, true).unwrap(),
            NaiveDate::from_ymd_opt(2026, 10, 24).unwrap()
        );

        // Invalid expressions
        assert!(parse_date_expression_ext("not-a-date", today, true).is_err());
        assert!(parse_date_expression_ext("-0d", today, true).is_err());
        assert!(parse_date_expression_ext("-4000d", today, true).is_err());
    }
}
