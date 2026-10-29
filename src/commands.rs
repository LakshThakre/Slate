use std::io::{BufRead, Write};
use std::path::Path;

use chrono::NaiveDate;

use crate::cli::Commands;
use crate::model::Todo;

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

fn map_storage_error(e: Box<dyn std::error::Error>) -> CmdError {
    match e.downcast::<CmdError>() {
        Ok(cmd_err) => *cmd_err,
        Err(e) => CmdError::Runtime(e.to_string()),
    }
}

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

/// Resolves task IDs from 1-based indices or a `--match` query.
/// Validates all indices before returning.
pub fn resolve_target_task_ids(
    todos: &[Todo],
    date: NaiveDate,
    indices: &[usize],
    match_query: Option<&str>,
) -> Result<Vec<u64>, CmdError> {
    if indices.is_empty() && match_query.is_none() {
        return Err(CmdError::Usage(
            "must specify task index or --match <TEXT>".to_string(),
        ));
    }
    if !indices.is_empty() && match_query.is_some() {
        return Err(CmdError::Usage(
            "cannot specify both task indices and --match".to_string(),
        ));
    }

    if let Some(query) = match_query {
        let q = query.trim();
        if q.is_empty() {
            return Err(CmdError::Usage("--match query cannot be empty".to_string()));
        }
        match crate::store::match_task_by_text(todos, date, q) {
            Ok(Some(task)) => Ok(vec![task.id]),
            Ok(None) => Err(CmdError::NotFound(format!("no task matches '{q}'"))),
            Err(candidates) => {
                let mut msg = format!("multiple tasks match '{q}':\n");
                for (idx, text) in candidates {
                    msg.push_str(&format!("  {idx}  {text}\n"));
                }
                if msg.ends_with('\n') {
                    msg.pop();
                }
                Err(CmdError::NotFound(msg))
            }
        }
    } else {
        let day_tasks = crate::store::tasks_for_date(todos, date);
        let count = day_tasks.len();
        for &idx in indices {
            if idx == 0 {
                return Err(CmdError::NotFound(
                    "task index must be at least 1 (indices are 1-based)".to_string(),
                ));
            }
            if idx > count {
                return Err(CmdError::NotFound(format!(
                    "task index {idx} is out of range (day has {count} task{})",
                    if count == 1 { "" } else { "s" }
                )));
            }
        }
        let mut target_ids = Vec::with_capacity(indices.len());
        for &idx in indices {
            let task = day_tasks[idx - 1];
            if !target_ids.contains(&task.id) {
                target_ids.push(task.id);
            }
        }
        Ok(target_ids)
    }
}

pub fn handle_add_with_reader<R: BufRead>(
    text: Vec<String>,
    date_str: Option<String>,
    quiet: bool,
    data_path: &Path,
    today: NaiveDate,
    out: &mut dyn Write,
    reader: R,
) -> Result<(), CmdError> {
    let date = match date_str {
        Some(s) => parse_date(&s, today)?,
        None => today,
    };

    let tasks_to_add: Vec<String> = if text.len() == 1 && text[0] == "-" {
        let mut lines = Vec::new();
        for line in reader.lines() {
            let l = line.map_err(|e| CmdError::Runtime(format!("failed to read stdin: {e}")))?;
            let trimmed = l.trim().to_string();
            if trimmed.is_empty() {
                return Err(CmdError::Usage("task text cannot be empty".to_string()));
            }
            lines.push(trimmed);
        }
        if lines.is_empty() {
            return Err(CmdError::Usage("task text cannot be empty".to_string()));
        }
        lines
    } else {
        let joined = text.join(" ");
        let trimmed = joined.trim().to_string();
        if trimmed.is_empty() {
            return Err(CmdError::Usage("task text cannot be empty".to_string()));
        }
        vec![trimmed]
    };

    crate::storage::try_load_modify_save(data_path, |todos| {
        for t in &tasks_to_add {
            crate::store::add_task(todos, t, date);
        }
        Ok::<(), CmdError>(())
    })
    .map_err(map_storage_error)?;

    if !quiet {
        for t in &tasks_to_add {
            writeln!(out, "Added: {t}")
                .map_err(|e| CmdError::Runtime(format!("write error: {e}")))?;
        }
    }

    Ok(())
}

pub fn handle_add(
    text: Vec<String>,
    date_str: Option<String>,
    quiet: bool,
    data_path: &Path,
    today: NaiveDate,
    out: &mut dyn Write,
) -> Result<(), CmdError> {
    handle_add_with_reader(
        text,
        date_str,
        quiet,
        data_path,
        today,
        out,
        std::io::stdin().lock(),
    )
}

pub fn handle_done(
    indices: Vec<usize>,
    date_str: Option<String>,
    match_query: Option<String>,
    data_path: &Path,
    today: NaiveDate,
) -> Result<(), CmdError> {
    let date = match date_str {
        Some(s) => parse_date(&s, today)?,
        None => today,
    };
    let now = chrono::Local::now();

    crate::storage::try_load_modify_save(data_path, |todos| {
        let target_ids = resolve_target_task_ids(todos, date, &indices, match_query.as_deref())?;
        for id in target_ids {
            crate::store::set_task_done(todos, id, true, now);
        }
        Ok::<(), CmdError>(())
    })
    .map_err(map_storage_error)?;

    Ok(())
}

pub fn handle_undone(
    indices: Vec<usize>,
    date_str: Option<String>,
    match_query: Option<String>,
    data_path: &Path,
    today: NaiveDate,
) -> Result<(), CmdError> {
    let date = match date_str {
        Some(s) => parse_date(&s, today)?,
        None => today,
    };
    let now = chrono::Local::now();

    crate::storage::try_load_modify_save(data_path, |todos| {
        let target_ids = resolve_target_task_ids(todos, date, &indices, match_query.as_deref())?;
        for id in target_ids {
            crate::store::set_task_done(todos, id, false, now);
        }
        Ok::<(), CmdError>(())
    })
    .map_err(map_storage_error)?;

    Ok(())
}

pub fn handle_rm(
    indices: Vec<usize>,
    date_str: Option<String>,
    match_query: Option<String>,
    data_path: &Path,
    today: NaiveDate,
) -> Result<(), CmdError> {
    let date = match date_str {
        Some(s) => parse_date(&s, today)?,
        None => today,
    };

    crate::storage::try_load_modify_save(data_path, |todos| {
        let target_ids = resolve_target_task_ids(todos, date, &indices, match_query.as_deref())?;
        for id in target_ids {
            crate::store::delete_task(todos, id);
        }
        Ok::<(), CmdError>(())
    })
    .map_err(map_storage_error)?;

    Ok(())
}

/// Dispatches subcommands.
pub fn run(
    cmd: Commands,
    data_path: &Path,
    today: NaiveDate,
    out: &mut dyn Write,
) -> Result<(), CmdError> {
    match cmd {
        Commands::Add { text, date, quiet } => handle_add(text, date, quiet, data_path, today, out),
        Commands::Done {
            indices,
            date,
            match_query,
        } => handle_done(indices, date, match_query, data_path, today),
        Commands::Undone {
            indices,
            date,
            match_query,
        } => handle_undone(indices, date, match_query, data_path, today),
        Commands::Rm {
            indices,
            date,
            match_query,
        } => handle_rm(indices, date, match_query, data_path, today),
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
    use tempfile::NamedTempFile;

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

    #[test]
    fn test_resolve_target_task_ids() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();
        let todos = vec![
            Todo {
                id: 10,
                text: "Buy fresh milk".to_string(),
                created_date: today,
                done: false,
                done_at: None,
            },
            Todo {
                id: 20,
                text: "Drink almond milk".to_string(),
                created_date: today,
                done: false,
                done_at: None,
            },
            Todo {
                id: 30,
                text: "Review pull request".to_string(),
                created_date: today,
                done: false,
                done_at: None,
            },
        ];

        // 1-based indexing
        assert_eq!(
            resolve_target_task_ids(&todos, today, &[1, 3], None).unwrap(),
            vec![10, 30]
        );

        // Out of range index
        assert!(matches!(
            resolve_target_task_ids(&todos, today, &[4], None),
            Err(CmdError::NotFound(_))
        ));
        assert!(matches!(
            resolve_target_task_ids(&todos, today, &[0], None),
            Err(CmdError::NotFound(_))
        ));

        // Exactly one match
        assert_eq!(
            resolve_target_task_ids(&todos, today, &[], Some("review")).unwrap(),
            vec![30]
        );

        // No match
        assert!(matches!(
            resolve_target_task_ids(&todos, today, &[], Some("nonexistent")),
            Err(CmdError::NotFound(_))
        ));

        // Ambiguous match lists candidates
        match resolve_target_task_ids(&todos, today, &[], Some("milk")) {
            Err(CmdError::NotFound(msg)) => {
                assert!(msg.contains("multiple tasks match 'milk':"));
                assert!(msg.contains("1  Buy fresh milk"));
                assert!(msg.contains("2  Drink almond milk"));
            }
            other => panic!("expected ambiguous match NotFound, got: {other:?}"),
        }

        // Both indices and match query is usage error
        assert!(matches!(
            resolve_target_task_ids(&todos, today, &[1], Some("milk")),
            Err(CmdError::Usage(_))
        ));

        // Neither is usage error
        assert!(matches!(
            resolve_target_task_ids(&todos, today, &[], None),
            Err(CmdError::Usage(_))
        ));
    }

    #[test]
    fn test_add_done_undone_rm_cycle() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path();
        let today = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();
        let mut out = Vec::new();

        // 1. Add single task
        handle_add_with_reader(
            vec!["Buy".to_string(), "milk".to_string()],
            None,
            false,
            path,
            today,
            &mut out,
            std::io::empty(),
        )
        .unwrap();
        assert_eq!(String::from_utf8_lossy(&out), "Added: Buy milk\n");
        out.clear();

        // 2. Add via stdin lines
        let stdin_input = b"Task two\nTask three\n";
        handle_add_with_reader(
            vec!["-".to_string()],
            None,
            false,
            path,
            today,
            &mut out,
            &stdin_input[..],
        )
        .unwrap();
        assert_eq!(
            String::from_utf8_lossy(&out),
            "Added: Task two\nAdded: Task three\n"
        );
        out.clear();

        // 3. Mark done task 1 and 2
        handle_done(vec![1, 2], None, None, path, today).unwrap();
        let loaded = crate::storage::load_todos(path).unwrap().todos;
        assert_eq!(loaded.len(), 3);
        assert!(loaded[0].done && loaded[0].done_at.is_some());
        assert!(loaded[1].done && loaded[1].done_at.is_some());
        assert!(!loaded[2].done);

        // 4. Undone task 1
        handle_undone(vec![1], None, None, path, today).unwrap();
        let loaded = crate::storage::load_todos(path).unwrap().todos;
        assert!(!loaded[0].done && loaded[0].done_at.is_none());

        // 5. Rm tasks 1 and 2 (original first two)
        handle_rm(vec![1, 2], None, None, path, today).unwrap();
        let loaded = crate::storage::load_todos(path).unwrap().todos;
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].text, "Task three");
    }

    #[test]
    fn test_multi_index_atomicity_on_out_of_range() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path();
        let today = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();
        let mut out = Vec::new();

        handle_add_with_reader(
            vec!["Task 1".to_string()],
            None,
            true,
            path,
            today,
            &mut out,
            std::io::empty(),
        )
        .unwrap();

        // rm 1 5 fails and does not delete Task 1
        let res = handle_rm(vec![1, 5], None, None, path, today);
        assert!(matches!(res, Err(CmdError::NotFound(_))));

        let loaded = crate::storage::load_todos(path).unwrap().todos;
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].text, "Task 1");
    }
}
