use std::io::Write;
use std::path::Path;

use chrono::NaiveDate;

use crate::cli::Commands;

#[derive(Debug, PartialEq, Eq)]
pub enum CmdError {
    Runtime(String),
    Usage(String),
    NotFound(String),
}

impl CmdError {
    pub fn exit_code(&self) -> i32 {
        match self {
            CmdError::Runtime(_) => 1,
            CmdError::Usage(_) => 2,
            CmdError::NotFound(_) => 3,
        }
    }

    pub fn message(&self) -> &str {
        match self {
            CmdError::Runtime(m) | CmdError::Usage(m) | CmdError::NotFound(m) => m,
        }
    }
}

impl std::fmt::Display for CmdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message())
    }
}

impl std::error::Error for CmdError {}

/// Parses a date string (`today`, `yesterday`, `tomorrow`, or `YYYY-MM-DD`).
pub fn parse_date(input: &str, today: NaiveDate) -> Result<NaiveDate, CmdError> {
    match input.to_lowercase().as_str() {
        "today" => Ok(today),
        "yesterday" => today
            .pred_opt()
            .ok_or_else(|| CmdError::Usage("date underflow for yesterday".to_string())),
        "tomorrow" => today
            .succ_opt()
            .ok_or_else(|| CmdError::Usage("date overflow for tomorrow".to_string())),
        _ => NaiveDate::parse_from_str(input, "%Y-%m-%d").map_err(|_| {
            CmdError::Usage(format!(
                "invalid date '{input}', expected YYYY-MM-DD, today, yesterday, or tomorrow"
            ))
        }),
    }
}

/// Dispatches subcommands.
pub fn run(
    cmd: Commands,
    data_path: &Path,
    _today: NaiveDate,
    out: &mut dyn Write,
) -> Result<(), CmdError> {
    match cmd {
        Commands::Path => {
            writeln!(out, "{}", data_path.display())
                .map_err(|e| CmdError::Runtime(format!("write error: {e}")))?;
            Ok(())
        }
        _ => Err(CmdError::Runtime("not implemented yet".to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_date_keywords_and_iso() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();
        assert_eq!(parse_date("today", today).unwrap(), today);
        assert_eq!(
            parse_date("yesterday", today).unwrap(),
            NaiveDate::from_ymd_opt(2026, 10, 28).unwrap()
        );
        assert_eq!(
            parse_date("tomorrow", today).unwrap(),
            NaiveDate::from_ymd_opt(2026, 10, 30).unwrap()
        );
        assert_eq!(
            parse_date("2026-05-15", today).unwrap(),
            NaiveDate::from_ymd_opt(2026, 5, 15).unwrap()
        );
        assert!(matches!(
            parse_date("invalid-date", today),
            Err(CmdError::Usage(_))
        ));
    }
}
