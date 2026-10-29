use std::collections::BTreeSet;
use std::fs;
use std::io::{BufRead, Write};
use std::path::Path;

use chrono::{DateTime, Local, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::commands::CmdError;
use crate::model::Todo;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExchangeFormat {
    Json,
    Md,
    Csv,
    Txt,
}

impl ExchangeFormat {
    pub fn parse(s: &str) -> Result<Self, CmdError> {
        match s.to_lowercase().as_str() {
            "json" => Ok(Self::Json),
            "md" | "markdown" => Ok(Self::Md),
            "csv" => Ok(Self::Csv),
            "txt" | "text" => Ok(Self::Txt),
            _ => Err(CmdError::Usage(format!(
                "unknown format '{s}', expected json, md, csv, or txt"
            ))),
        }
    }

    pub fn infer_from_path(path: &Path) -> Result<Self, CmdError> {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some(ext) => match ext.to_lowercase().as_str() {
                "json" => Ok(Self::Json),
                "md" | "markdown" => Ok(Self::Md),
                "csv" => Ok(Self::Csv),
                "txt" | "text" => Ok(Self::Txt),
                _ => Err(CmdError::Usage(format!(
                    "cannot infer format from extension '.{ext}', please specify --format"
                ))),
            },
            None => Err(CmdError::Usage(
                "cannot infer format from file without extension, please specify --format"
                    .to_string(),
            )),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct JsonExchangeTask {
    #[serde(default)]
    pub id: Option<u64>,
    pub text: String,
    pub done: bool,
    #[serde(default)]
    pub done_at: Option<DateTime<Local>>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct JsonExchangeDay {
    pub date: NaiveDate,
    pub tasks: Vec<JsonExchangeTask>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct JsonExchangePayload {
    pub schema_version: u32,
    pub days: Vec<JsonExchangeDay>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawImportTask {
    pub date: NaiveDate,
    pub text: String,
    pub done: bool,
    pub done_at: Option<DateTime<Local>>,
}

/// Normalizes text input by stripping UTF-8 BOM and standardizing CRLF line endings.
pub fn normalize_input(input: &str) -> &str {
    input.strip_prefix('\u{feff}').unwrap_or(input)
}

// ==================== EXPORT IMPLEMENTATION ====================

pub fn export_to_string(
    format: ExchangeFormat,
    todos: &[Todo],
    from_date: Option<NaiveDate>,
    to_date: Option<NaiveDate>,
    target_date: Option<NaiveDate>,
    today: NaiveDate,
) -> Result<String, CmdError> {
    // 1. Filter tasks
    let filtered: Vec<&Todo> = todos
        .iter()
        .filter(|t| {
            if let Some(d) = target_date {
                return t.created_date == d;
            }
            if let Some(f) = from_date {
                if t.created_date < f {
                    return false;
                }
            }
            if let Some(to) = to_date {
                if t.created_date > to {
                    return false;
                }
            }
            true
        })
        .collect();

    match format {
        ExchangeFormat::Json => export_json(&filtered),
        ExchangeFormat::Md => export_markdown(&filtered),
        ExchangeFormat::Csv => export_csv(&filtered),
        ExchangeFormat::Txt => {
            let distinct_dates: BTreeSet<NaiveDate> =
                filtered.iter().map(|t| t.created_date).collect();
            if distinct_dates.len() > 1 {
                return Err(CmdError::Usage(
                    "txt export supports only a single day (use --date or export to md or csv)"
                        .to_string(),
                ));
            }
            let day = target_date
                .or_else(|| distinct_dates.iter().next().copied())
                .unwrap_or(today);
            export_txt(&filtered, day)
        }
    }
}

fn export_json(todos: &[&Todo]) -> Result<String, CmdError> {
    let dates: BTreeSet<NaiveDate> = todos.iter().map(|t| t.created_date).collect();
    // Sort chronological
    let days_list: Vec<NaiveDate> = dates.into_iter().collect();

    let mut days = Vec::new();
    for d in days_list {
        let tasks: Vec<JsonExchangeTask> = todos
            .iter()
            .filter(|t| t.created_date == d)
            .map(|t| JsonExchangeTask {
                id: Some(t.id),
                text: t.text.clone(),
                done: t.done,
                done_at: t.done_at,
            })
            .collect();
        days.push(JsonExchangeDay { date: d, tasks });
    }

    let payload = JsonExchangePayload {
        schema_version: 1,
        days,
    };
    serde_json::to_string_pretty(&payload)
        .map_err(|e| CmdError::Runtime(format!("JSON serialization error: {e}")))
}

fn export_markdown(todos: &[&Todo]) -> Result<String, CmdError> {
    let dates: BTreeSet<NaiveDate> = todos.iter().map(|t| t.created_date).collect();
    let mut out = String::new();

    for (idx, d) in dates.iter().enumerate() {
        if idx > 0 {
            out.push('\n');
        }
        out.push_str(&format!("## {}\n", d.format("%Y-%m-%d")));

        for t in todos.iter().filter(|t| t.created_date == *d) {
            let check = if t.done { "x" } else { " " };
            let time_comment = if let Some(dt) = t.done_at {
                format!(" <!-- done {} -->", dt.to_rfc3339())
            } else {
                String::new()
            };
            out.push_str(&format!("- [{check}] {}{time_comment}\n", t.text));
        }
    }

    Ok(out)
}

fn export_csv(todos: &[&Todo]) -> Result<String, CmdError> {
    let mut wtr = csv::Writer::from_writer(Vec::new());
    wtr.write_record(["date", "text", "done", "done_at"])
        .map_err(|e| CmdError::Runtime(format!("CSV write error: {e}")))?;

    for t in todos {
        let date_str = t.created_date.format("%Y-%m-%d").to_string();
        let done_str = if t.done { "true" } else { "false" };
        let done_at_str = t.done_at.map(|dt| dt.to_rfc3339()).unwrap_or_default();
        wtr.write_record([&date_str, &t.text, done_str, &done_at_str])
            .map_err(|e| CmdError::Runtime(format!("CSV write error: {e}")))?;
    }

    let bytes = wtr
        .into_inner()
        .map_err(|e| CmdError::Runtime(format!("CSV write error: {e}")))?;
    String::from_utf8(bytes).map_err(|e| CmdError::Runtime(format!("UTF-8 error: {e}")))
}

fn export_txt(todos: &[&Todo], day: NaiveDate) -> Result<String, CmdError> {
    let mut out = String::new();
    for t in todos.iter().filter(|t| t.created_date == day) {
        let check = if t.done { "x" } else { " " };
        out.push_str(&format!("[{check}] {}\n", t.text));
    }
    Ok(out)
}

// ==================== IMPORT PARSING ====================

pub fn parse_import_content(
    format: ExchangeFormat,
    content: &str,
    default_date: NaiveDate,
) -> Result<Vec<RawImportTask>, CmdError> {
    let clean = normalize_input(content);
    if clean.trim().is_empty() {
        return Ok(Vec::new());
    }

    match format {
        ExchangeFormat::Json => parse_import_json(clean),
        ExchangeFormat::Md => parse_import_markdown(clean, default_date),
        ExchangeFormat::Csv => parse_import_csv(clean),
        ExchangeFormat::Txt => parse_import_txt(clean, default_date),
    }
}

fn parse_import_json(content: &str) -> Result<Vec<RawImportTask>, CmdError> {
    let payload: JsonExchangePayload = serde_json::from_str(content).map_err(|e| {
        CmdError::Runtime(format!(
            "import error at line {}, column {}: invalid JSON payload ({})",
            e.line(),
            e.column(),
            e
        ))
    })?;

    if payload.schema_version != 1 {
        return Err(CmdError::Runtime(format!(
            "unsupported schema version {}, expected 1",
            payload.schema_version
        )));
    }

    let mut tasks = Vec::new();
    for day in payload.days {
        for t in day.tasks {
            let trimmed = t.text.trim();
            if trimmed.is_empty() {
                return Err(CmdError::Runtime("task text cannot be empty".to_string()));
            }
            tasks.push(RawImportTask {
                date: day.date,
                text: trimmed.to_string(),
                done: t.done,
                done_at: t.done_at,
            });
        }
    }
    Ok(tasks)
}

fn parse_import_markdown(
    content: &str,
    default_date: NaiveDate,
) -> Result<Vec<RawImportTask>, CmdError> {
    let mut current_date = default_date;
    let mut tasks = Vec::new();

    for (idx, line) in content.lines().enumerate() {
        let line_num = idx + 1;
        let trimmed = line.trim();

        if trimmed.starts_with('#') {
            let heading_text = trimmed.trim_start_matches('#').trim();
            match NaiveDate::parse_from_str(heading_text, "%Y-%m-%d") {
                Ok(d) => current_date = d,
                Err(_) => {
                    return Err(CmdError::Runtime(format!(
                        "import error at line {line_num}: invalid date heading '{heading_text}', expected YYYY-MM-DD"
                    )));
                }
            }
        } else if trimmed.starts_with("- [") || trimmed.starts_with("* [") {
            if trimmed.len() < 5 || &trimmed[4..5] != "]" {
                return Err(CmdError::Runtime(format!(
                    "import error at line {line_num}: malformed task checkbox"
                )));
            }

            let check_char = trimmed.as_bytes()[3];
            let done = match check_char {
                b' ' => false,
                b'x' | b'X' => true,
                _ => {
                    return Err(CmdError::Runtime(format!(
                        "import error at line {line_num}: invalid checkbox character '{}'",
                        check_char as char
                    )));
                }
            };

            let rest = trimmed[5..].trim();
            let (text, done_at) = if let Some(comment_start) = rest.find("<!-- done ") {
                let text_part = rest[..comment_start].trim();
                let comment_rest = &rest[comment_start + 10..];
                if let Some(comment_end) = comment_rest.find("-->") {
                    let ts_str = comment_rest[..comment_end].trim();
                    match DateTime::parse_from_rfc3339(ts_str) {
                        Ok(dt) => (text_part, Some(dt.with_timezone(&Local))),
                        Err(e) => {
                            return Err(CmdError::Runtime(format!(
                                "import error at line {line_num}: invalid RFC 3339 timestamp '{ts_str}' ({e})"
                            )));
                        }
                    }
                } else {
                    (text_part, None)
                }
            } else {
                (rest, None)
            };

            if text.is_empty() {
                return Err(CmdError::Runtime(format!(
                    "import error at line {line_num}: task text cannot be empty"
                )));
            }

            tasks.push(RawImportTask {
                date: current_date,
                text: text.to_string(),
                done,
                done_at,
            });
        }
    }

    Ok(tasks)
}

fn parse_import_csv(content: &str) -> Result<Vec<RawImportTask>, CmdError> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(content.as_bytes());

    let mut tasks = Vec::new();
    let mut line_num = 1; // header is line 1

    for result in rdr.records() {
        line_num += 1;
        let record = result.map_err(|e| {
            CmdError::Runtime(format!("import error at line {line_num}: CSV error ({e})"))
        })?;

        if record.len() < 3 {
            return Err(CmdError::Runtime(format!(
                "import error at line {line_num}: expected at least 3 columns (date,text,done)"
            )));
        }

        let date_str = record[0].trim();
        let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d").map_err(|_| {
            CmdError::Runtime(format!(
                "import error at line {line_num}: invalid date '{date_str}', expected YYYY-MM-DD"
            ))
        })?;

        let text = record[1].trim();
        if text.is_empty() {
            return Err(CmdError::Runtime(format!(
                "import error at line {line_num}: task text cannot be empty"
            )));
        }

        let done_str = record[2].trim().to_lowercase();
        let done = match done_str.as_str() {
            "true" | "1" | "x" => true,
            "false" | "0" | "" => false,
            _ => {
                return Err(CmdError::Runtime(format!(
                    "import error at line {line_num}: invalid done value '{done_str}', expected true/false/1/0/x"
                )));
            }
        };

        let done_at = if record.len() >= 4 {
            let ts_str = record[3].trim();
            if ts_str.is_empty() {
                None
            } else {
                match DateTime::parse_from_rfc3339(ts_str) {
                    Ok(dt) => Some(dt.with_timezone(&Local)),
                    Err(e) => {
                        return Err(CmdError::Runtime(format!(
                            "import error at line {line_num}: invalid timestamp '{ts_str}' ({e})"
                        )));
                    }
                }
            }
        } else {
            None
        };

        tasks.push(RawImportTask {
            date,
            text: text.to_string(),
            done,
            done_at,
        });
    }

    Ok(tasks)
}

fn parse_import_txt(
    content: &str,
    default_date: NaiveDate,
) -> Result<Vec<RawImportTask>, CmdError> {
    let mut tasks = Vec::new();

    for (idx, line) in content.lines().enumerate() {
        let line_num = idx + 1;
        let trimmed = line.trim();

        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let (done, text) = if let Some(rest) = trimmed.strip_prefix("[x] ") {
            (true, rest.trim())
        } else if let Some(rest) = trimmed.strip_prefix("[X] ") {
            (true, rest.trim())
        } else if let Some(rest) = trimmed.strip_prefix("[ ] ") {
            (false, rest.trim())
        } else {
            (false, trimmed)
        };

        if text.is_empty() {
            return Err(CmdError::Runtime(format!(
                "import error at line {line_num}: task text cannot be empty"
            )));
        }

        tasks.push(RawImportTask {
            date: default_date,
            text: text.to_string(),
            done,
            done_at: None,
        });
    }

    Ok(tasks)
}

// ==================== EXECUTE IMPORT & EXPORT ====================

#[allow(clippy::too_many_arguments)]
pub fn handle_export(
    format_opt: Option<String>,
    from_str: Option<String>,
    to_str: Option<String>,
    date_str: Option<String>,
    output_path: Option<&Path>,
    data_path: &Path,
    today: NaiveDate,
    out: &mut dyn Write,
) -> Result<(), CmdError> {
    let format = match format_opt {
        Some(f) => ExchangeFormat::parse(&f)?,
        None => match output_path {
            Some(p) => ExchangeFormat::infer_from_path(p)?,
            None => {
                return Err(CmdError::Usage(
                    "--format is required when exporting to stdout".to_string(),
                ));
            }
        },
    };

    let target_date = match date_str {
        Some(s) => Some(crate::commands::parse_date(&s, today)?),
        None => None,
    };
    let from_date = match from_str {
        Some(s) => Some(crate::commands::parse_date(&s, today)?),
        None => None,
    };
    let to_date = match to_str {
        Some(s) => Some(crate::commands::parse_date(&s, today)?),
        None => None,
    };

    let load_res =
        crate::storage::load_todos(data_path).map_err(|e| CmdError::Runtime(e.to_string()))?;
    let content = export_to_string(
        format,
        &load_res.todos,
        from_date,
        to_date,
        target_date,
        today,
    )?;

    if let Some(dest) = output_path {
        if let Some(parent) = dest.parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::write(dest, content.as_bytes())
            .map_err(|e| CmdError::Runtime(format!("failed to write {}: {e}", dest.display())))?;
    } else {
        out.write_all(content.as_bytes())
            .map_err(|e| CmdError::Runtime(format!("write error: {e}")))?;
        out.flush()
            .map_err(|e| CmdError::Runtime(format!("write error: {e}")))?;
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn handle_import(
    file_arg: &str,
    format_opt: Option<String>,
    date_str: Option<String>,
    replace: bool,
    yes: bool,
    dry_run: bool,
    data_path: &Path,
    today: NaiveDate,
    out: &mut dyn Write,
) -> Result<(), CmdError> {
    if replace && !yes {
        return Err(CmdError::Usage(
            "--replace requires --yes to confirm replacing the entire dataset".to_string(),
        ));
    }

    let is_stdin = file_arg == "-";
    let format = match format_opt {
        Some(f) => ExchangeFormat::parse(&f)?,
        None => {
            if is_stdin {
                return Err(CmdError::Usage(
                    "--format is required when reading from stdin".to_string(),
                ));
            }
            ExchangeFormat::infer_from_path(Path::new(file_arg))?
        }
    };

    let default_date = match date_str {
        Some(s) => crate::commands::parse_date(&s, today)?,
        None => today,
    };

    let content = if is_stdin {
        let mut buffer = String::new();
        let stdin = std::io::stdin();
        for line in stdin.lock().lines() {
            let l = line.map_err(|e| CmdError::Runtime(format!("failed to read stdin: {e}")))?;
            buffer.push_str(&l);
            buffer.push('\n');
        }
        buffer
    } else {
        fs::read_to_string(file_arg)
            .map_err(|e| CmdError::Runtime(format!("cannot read file '{file_arg}': {e}")))?
    };

    // Check empty file
    let clean = normalize_input(&content);
    if clean.trim().is_empty() {
        eprintln!("slate: warning: file is empty");
        return Ok(());
    }

    // Parse all-or-nothing
    let raw_tasks = parse_import_content(format, clean, default_date)?;

    // Concurrency lock and perform import
    if replace {
        if dry_run {
            writeln!(out, "would add {}, skip 0 duplicates", raw_tasks.len())
                .map_err(|e| CmdError::Runtime(format!("write error: {e}")))?;
            return Ok(());
        }

        // Backup existing file before replace
        if data_path.exists() {
            let bak_path = data_path.with_extension("bak");
            let _ = fs::copy(data_path, &bak_path);
        }

        crate::storage::try_load_modify_save(data_path, |todos| {
            todos.clear();
            for (next_id, t) in (1..).zip(raw_tasks.iter()) {
                todos.push(Todo {
                    id: next_id,
                    text: t.text.clone(),
                    created_date: t.date,
                    done: t.done,
                    done_at: t.done_at,
                });
            }
            Ok::<(), CmdError>(())
        })
        .map_err(|e| CmdError::Runtime(e.to_string()))?;

        writeln!(
            out,
            "Imported {} task{} (0 duplicates skipped)",
            raw_tasks.len(),
            if raw_tasks.len() == 1 { "" } else { "s" }
        )
        .map_err(|e| CmdError::Runtime(format!("write error: {e}")))?;
    } else {
        // Merge mode: compute added and duplicates
        let mut added_tasks = Vec::new();
        let mut duplicate_count = 0;

        let load_res =
            crate::storage::load_todos(data_path).map_err(|e| CmdError::Runtime(e.to_string()))?;
        let existing = load_res.todos;

        let mut existing_set: BTreeSet<(NaiveDate, String)> = existing
            .iter()
            .map(|t| (t.created_date, t.text.trim().to_string()))
            .collect();

        for t in raw_tasks {
            let key = (t.date, t.text.trim().to_string());
            if existing_set.contains(&key) {
                duplicate_count += 1;
            } else {
                existing_set.insert(key);
                added_tasks.push(t);
            }
        }

        if dry_run {
            let dup_word = if duplicate_count == 1 {
                "duplicate"
            } else {
                "duplicates"
            };
            writeln!(
                out,
                "would add {}, skip {} {dup_word}",
                added_tasks.len(),
                duplicate_count
            )
            .map_err(|e| CmdError::Runtime(format!("write error: {e}")))?;
            return Ok(());
        }

        let added_count = added_tasks.len();
        crate::storage::try_load_modify_save(data_path, |todos| {
            let start_id = crate::store::next_task_id(todos);
            for (next_id, t) in (start_id..).zip(added_tasks) {
                todos.push(Todo {
                    id: next_id,
                    text: t.text,
                    created_date: t.date,
                    done: t.done,
                    done_at: t.done_at,
                });
            }
            Ok::<(), CmdError>(())
        })
        .map_err(|e| CmdError::Runtime(e.to_string()))?;

        let task_word = if added_count == 1 { "task" } else { "tasks" };
        let dup_word = if duplicate_count == 1 {
            "duplicate"
        } else {
            "duplicates"
        };
        writeln!(
            out,
            "Imported {added_count} {task_word} ({duplicate_count} {dup_word} skipped)"
        )
        .map_err(|e| CmdError::Runtime(format!("write error: {e}")))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_round_trip() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 23).unwrap();
        let now = Local::now();
        let todos = vec![
            Todo {
                id: 1,
                text: "Buy milk".to_string(),
                created_date: today,
                done: true,
                done_at: Some(now),
            },
            Todo {
                id: 2,
                text: "Review PR".to_string(),
                created_date: today,
                done: false,
                done_at: None,
            },
        ];

        let exported =
            export_to_string(ExchangeFormat::Json, &todos, None, None, None, today).unwrap();
        let imported = parse_import_content(ExchangeFormat::Json, &exported, today).unwrap();

        assert_eq!(imported.len(), 2);
        assert_eq!(imported[0].text, "Buy milk");
        assert!(imported[0].done);
        assert!(imported[0].done_at.is_some());
        assert_eq!(imported[1].text, "Review PR");
        assert!(!imported[1].done);
    }

    #[test]
    fn test_markdown_round_trip() {
        let d1 = NaiveDate::from_ymd_opt(2026, 10, 23).unwrap();
        let d2 = NaiveDate::from_ymd_opt(2026, 10, 24).unwrap();
        let now = Local::now();
        let todos = vec![
            Todo {
                id: 1,
                text: "Buy bread".to_string(),
                created_date: d1,
                done: true,
                done_at: Some(now),
            },
            Todo {
                id: 2,
                text: "Drink water".to_string(),
                created_date: d2,
                done: false,
                done_at: None,
            },
        ];

        let exported = export_to_string(ExchangeFormat::Md, &todos, None, None, None, d1).unwrap();
        assert!(exported.contains("## 2026-10-23"));
        assert!(exported.contains("## 2026-10-24"));
        assert!(exported.contains("<!-- done "));

        let imported = parse_import_content(ExchangeFormat::Md, &exported, d1).unwrap();
        assert_eq!(imported.len(), 2);
        assert_eq!(imported[0].date, d1);
        assert_eq!(imported[0].text, "Buy bread");
        assert!(imported[0].done);
        assert!(imported[0].done_at.is_some());
        assert_eq!(imported[1].date, d2);
        assert_eq!(imported[1].text, "Drink water");
        assert!(!imported[1].done);
    }

    #[test]
    fn test_csv_round_trip_with_special_characters() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();
        let now = Local::now();
        let complex_text = "Task with \"quotes\", commas, and\nnewlines and 🚀 unicode";
        let todos = vec![Todo {
            id: 1,
            text: complex_text.to_string(),
            created_date: today,
            done: true,
            done_at: Some(now),
        }];

        let exported =
            export_to_string(ExchangeFormat::Csv, &todos, None, None, None, today).unwrap();
        let imported = parse_import_content(ExchangeFormat::Csv, &exported, today).unwrap();

        assert_eq!(imported.len(), 1);
        assert_eq!(imported[0].text, complex_text);
        assert!(imported[0].done);
        assert!(imported[0].done_at.is_some());
    }

    #[test]
    fn test_txt_round_trip() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();
        let todos = vec![
            Todo {
                id: 1,
                text: "Task 1".to_string(),
                created_date: today,
                done: true,
                done_at: None,
            },
            Todo {
                id: 2,
                text: "Task 2".to_string(),
                created_date: today,
                done: false,
                done_at: None,
            },
        ];

        let exported =
            export_to_string(ExchangeFormat::Txt, &todos, None, None, Some(today), today).unwrap();
        assert_eq!(exported, "[x] Task 1\n[ ] Task 2\n");

        let imported = parse_import_content(ExchangeFormat::Txt, &exported, today).unwrap();
        assert_eq!(imported.len(), 2);
        assert_eq!(imported[0].text, "Task 1");
        assert!(imported[0].done);
        assert_eq!(imported[1].text, "Task 2");
        assert!(!imported[1].done);
    }

    #[test]
    fn test_all_or_nothing_validation() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();
        let bad_csv =
            "date,text,done,done_at\n2026-10-29,Good task,true,\nnot-a-date,Bad date,false,\n";
        let err = parse_import_content(ExchangeFormat::Csv, bad_csv, today).unwrap_err();
        assert!(err.message().contains("import error at line 3"));
    }

    #[test]
    fn test_bom_and_crlf_normalization() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();
        let content_with_bom = "\u{feff}## 2026-10-29\r\n- [ ] Task with CRLF\r\n";
        let imported = parse_import_content(ExchangeFormat::Md, content_with_bom, today).unwrap();
        assert_eq!(imported.len(), 1);
        assert_eq!(imported[0].text, "Task with CRLF");
    }

    #[test]
    fn test_import_merge_duplicates_and_replace() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let path = file.path();
        let today = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();
        let mut out = Vec::new();

        // Initial task
        crate::storage::save_todos_unlocked(
            path,
            &[Todo {
                id: 1,
                text: "Existing task".to_string(),
                created_date: today,
                done: false,
                done_at: None,
            }],
            false,
            None,
            false,
            None,
        )
        .unwrap();

        // Create import file with 1 duplicate and 1 new task
        let import_file = tempfile::NamedTempFile::new().unwrap();
        fs::write(import_file.path(), "Existing task\nNew task\n").unwrap();

        // Dry-run merge
        handle_import(
            import_file.path().to_str().unwrap(),
            Some("txt".to_string()),
            None,
            false,
            false,
            true,
            path,
            today,
            &mut out,
        )
        .unwrap();
        assert_eq!(
            String::from_utf8_lossy(&out),
            "would add 1, skip 1 duplicate\n"
        );
        out.clear();

        // Real merge
        handle_import(
            import_file.path().to_str().unwrap(),
            Some("txt".to_string()),
            None,
            false,
            false,
            false,
            path,
            today,
            &mut out,
        )
        .unwrap();
        assert_eq!(
            String::from_utf8_lossy(&out),
            "Imported 1 task (1 duplicate skipped)\n"
        );
        out.clear();

        let loaded = crate::storage::load_todos(path).unwrap().todos;
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].text, "Existing task");
        assert_eq!(loaded[1].text, "New task");

        // Test replace mode with --yes
        let replace_file = tempfile::NamedTempFile::new().unwrap();
        fs::write(replace_file.path(), "Brand new task\n").unwrap();

        handle_import(
            replace_file.path().to_str().unwrap(),
            Some("txt".to_string()),
            None,
            true,
            true,
            false,
            path,
            today,
            &mut out,
        )
        .unwrap();
        assert_eq!(
            String::from_utf8_lossy(&out),
            "Imported 1 task (0 duplicates skipped)\n"
        );

        let loaded = crate::storage::load_todos(path).unwrap().todos;
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].text, "Brand new task");

        // Check backup file was created
        let bak = path.with_extension("bak");
        assert!(bak.exists());
    }
}
