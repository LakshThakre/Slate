use std::io::{self, IsTerminal, Write};

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::model::Todo;

/// SafeWriter wraps an inner `io::Write` and treats `io::ErrorKind::BrokenPipe` as success.
/// This prevents panics when downstream consumers close the pipe early (e.g. `slate list | head -1`).
pub struct SafeWriter<W: Write> {
    inner: W,
    broken_pipe: bool,
}

impl<W: Write> SafeWriter<W> {
    pub fn new(inner: W) -> Self {
        Self {
            inner,
            broken_pipe: false,
        }
    }

    pub fn is_broken_pipe(&self) -> bool {
        self.broken_pipe
    }

    pub fn write_str(&mut self, s: &str) -> io::Result<()> {
        if self.broken_pipe {
            return Ok(());
        }
        if let Err(e) = self.inner.write_all(s.as_bytes()) {
            if e.kind() == io::ErrorKind::BrokenPipe {
                self.broken_pipe = true;
                return Ok(());
            }
            return Err(e);
        }
        Ok(())
    }

    pub fn write_line(&mut self, s: &str) -> io::Result<()> {
        if self.broken_pipe {
            return Ok(());
        }
        if let Err(e) = writeln!(self.inner, "{s}") {
            if e.kind() == io::ErrorKind::BrokenPipe {
                self.broken_pipe = true;
                return Ok(());
            }
            return Err(e);
        }
        Ok(())
    }

    pub fn flush(&mut self) -> io::Result<()> {
        if self.broken_pipe {
            return Ok(());
        }
        if let Err(e) = self.inner.flush() {
            if e.kind() == io::ErrorKind::BrokenPipe {
                self.broken_pipe = true;
                return Ok(());
            }
            return Err(e);
        }
        Ok(())
    }
}

impl<W: Write> Write for SafeWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.broken_pipe {
            return Ok(buf.len());
        }
        match self.inner.write(buf) {
            Ok(n) => Ok(n),
            Err(e) if e.kind() == io::ErrorKind::BrokenPipe => {
                self.broken_pipe = true;
                Ok(buf.len())
            }
            Err(e) => Err(e),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        if self.broken_pipe {
            return Ok(());
        }
        match self.inner.flush() {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::BrokenPipe => {
                self.broken_pipe = true;
                Ok(())
            }
            Err(e) => Err(e),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct JsonTask {
    pub index: usize,
    pub text: String,
    pub done: bool,
    pub done_at: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct JsonDay {
    pub date: String,
    pub tasks: Vec<JsonTask>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct JsonListDay {
    pub schema_version: u32,
    pub date: String,
    pub tasks: Vec<JsonTask>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct JsonListAll {
    pub schema_version: u32,
    pub days: Vec<JsonDay>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct JsonToday {
    pub schema_version: u32,
    pub date: String,
    pub done: usize,
    pub total: usize,
    pub percent: u64,
    pub tasks: Vec<JsonTask>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct WaybarOutput {
    pub text: String,
    pub tooltip: String,
    pub class: String,
    pub percentage: u64,
}

pub fn sanitize_plain_text(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c == '\t' || c == '\n' || c == '\r' {
                ' '
            } else {
                c
            }
        })
        .collect()
}

pub fn should_use_color() -> bool {
    io::stdout().is_terminal() && std::env::var("NO_COLOR").is_err()
}

pub fn should_use_unicode(config: &Config) -> bool {
    if !io::stdout().is_terminal() {
        return false;
    }
    if let Ok(val) = std::env::var("SLATE_ASCII") {
        let trimmed = val.trim();
        if trimmed == "1" || trimmed.eq_ignore_ascii_case("true") {
            return false;
        }
        if trimmed == "0" || trimmed.eq_ignore_ascii_case("false") {
            return true;
        }
    }
    config.ui.symbols != crate::config::SymbolsMode::Ascii
}

pub fn write_list_plain(
    days_and_tasks: &[(NaiveDate, Vec<(usize, &Todo)>)],
    is_all: bool,
    out: &mut dyn Write,
) -> io::Result<()> {
    for (date, tasks) in days_and_tasks {
        let date_str = date.format("%Y-%m-%d").to_string();
        for (idx, task) in tasks {
            let status = if task.done { "x" } else { "-" };
            let clean = sanitize_plain_text(&task.text);
            if is_all {
                writeln!(out, "{date_str}\t{idx}\t{status}\t{clean}")?;
            } else {
                writeln!(out, "{idx}\t{status}\t{clean}")?;
            }
        }
    }
    Ok(())
}

pub fn write_list_json(
    days_and_tasks: &[(NaiveDate, Vec<(usize, &Todo)>)],
    is_all: bool,
    out: &mut dyn Write,
) -> io::Result<()> {
    if is_all {
        let days = days_and_tasks
            .iter()
            .map(|(date, tasks)| JsonDay {
                date: date.format("%Y-%m-%d").to_string(),
                tasks: tasks
                    .iter()
                    .map(|(idx, t)| JsonTask {
                        index: *idx,
                        text: t.text.clone(),
                        done: t.done,
                        done_at: t.done_at.map(|dt| dt.to_rfc3339()),
                    })
                    .collect(),
            })
            .collect();

        let payload = JsonListAll {
            schema_version: 1,
            days,
        };
        let s = serde_json::to_string(&payload)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        writeln!(out, "{s}")?;
    } else {
        let (date, tasks) = days_and_tasks
            .first()
            .cloned()
            .unwrap_or_else(|| (chrono::Local::now().date_naive(), Vec::new()));
        let json_tasks = tasks
            .iter()
            .map(|(idx, t)| JsonTask {
                index: *idx,
                text: t.text.clone(),
                done: t.done,
                done_at: t.done_at.map(|dt| dt.to_rfc3339()),
            })
            .collect();

        let payload = JsonListDay {
            schema_version: 1,
            date: date.format("%Y-%m-%d").to_string(),
            tasks: json_tasks,
        };
        let s = serde_json::to_string(&payload)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        writeln!(out, "{s}")?;
    }
    Ok(())
}

pub fn write_list_human(
    days_and_tasks: &[(NaiveDate, Vec<(usize, &Todo)>)],
    is_all: bool,
    config: &Config,
    out: &mut dyn Write,
) -> io::Result<()> {
    let use_color = should_use_color();
    let use_unicode = should_use_unicode(config);

    let (check_todo, check_done) = if use_unicode {
        ("○", "✓")
    } else {
        ("[ ]", "[x]")
    };

    for (day_idx, (date, tasks)) in days_and_tasks.iter().enumerate() {
        if is_all {
            let date_str = date.format("%Y-%m-%d").to_string();
            if day_idx > 0 {
                writeln!(out)?;
            }
            if use_color {
                writeln!(out, "\x1b[2m{date_str}\x1b[0m")?;
            } else {
                writeln!(out, "{date_str}")?;
            }
        }

        for (idx, task) in tasks {
            let symbol = if task.done { check_done } else { check_todo };
            let time_suffix = if task.done {
                match task.done_at {
                    Some(dt) => {
                        format!(
                            "           done {}",
                            crate::ui::format_time(&dt, config.time_format)
                        )
                    }
                    None => "           done".to_string(),
                }
            } else {
                String::new()
            };

            if use_color {
                if task.done {
                    writeln!(
                        out,
                        "  {:>2}  \x1b[32m{}\x1b[0m  \x1b[2m{}{}\x1b[0m",
                        idx, symbol, task.text, time_suffix
                    )?;
                } else {
                    writeln!(
                        out,
                        "  {:>2}  \x1b[36m{}\x1b[0m  {}",
                        idx, symbol, task.text
                    )?;
                }
            } else {
                writeln!(
                    out,
                    "  {:>2}  {}  {}{}",
                    idx, symbol, task.text, time_suffix
                )?;
            }
        }
    }
    Ok(())
}

pub fn write_today_plain(done: usize, total: usize, out: &mut dyn Write) -> io::Result<()> {
    writeln!(out, "{done}/{total}")
}

pub fn write_today_json(
    date: NaiveDate,
    done: usize,
    total: usize,
    tasks: &[(usize, &Todo)],
    out: &mut dyn Write,
) -> io::Result<()> {
    let percent = (done * 100).checked_div(total).unwrap_or(0) as u64;

    let json_tasks = tasks
        .iter()
        .map(|(idx, t)| JsonTask {
            index: *idx,
            text: t.text.clone(),
            done: t.done,
            done_at: t.done_at.map(|dt| dt.to_rfc3339()),
        })
        .collect();

    let payload = JsonToday {
        schema_version: 1,
        date: date.format("%Y-%m-%d").to_string(),
        done,
        total,
        percent,
        tasks: json_tasks,
    };

    let s = serde_json::to_string(&payload)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    writeln!(out, "{s}")
}

pub fn write_today_waybar(
    done: usize,
    total: usize,
    tasks: &[(usize, &Todo)],
    out: &mut dyn Write,
) -> io::Result<()> {
    let class = if total == 0 {
        "empty".to_string()
    } else if done == total {
        "done".to_string()
    } else {
        "pending".to_string()
    };

    let percentage = (done * 100).checked_div(total).unwrap_or(0) as u64;

    let tooltip = if tasks.is_empty() {
        "No tasks today".to_string()
    } else {
        tasks
            .iter()
            .map(|(_, t)| format!("{} {}", if t.done { "✓" } else { "○" }, t.text))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let payload = WaybarOutput {
        text: format!("{done}/{total}"),
        tooltip,
        class,
        percentage,
    };

    let s = serde_json::to_string(&payload)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    writeln!(out, "{s}")
}

pub fn write_today_human(
    date: NaiveDate,
    done: usize,
    total: usize,
    tasks: &[(usize, &Todo)],
    config: &Config,
    out: &mut dyn Write,
) -> io::Result<()> {
    let use_color = should_use_color();
    let use_unicode = should_use_unicode(config);

    let (fill_ch, empty_ch) = if use_unicode {
        ('█', '░')
    } else {
        ('#', '-')
    };

    let bar_width: usize = 10;
    let filled_len = if total == 0 {
        0
    } else {
        ((done as f64 / total as f64) * bar_width as f64).round() as usize
    };
    let filled_str: String = std::iter::repeat_n(fill_ch, filled_len).collect();
    let empty_str: String =
        std::iter::repeat_n(empty_ch, bar_width.saturating_sub(filled_len)).collect();
    let bar = format!("{filled_str}{empty_str}");

    let date_str = date.format(&config.date_format).to_string();
    if use_color {
        writeln!(out, "\x1b[1m{date_str}\x1b[0m")?;
        writeln!(out, "{done} of {total} done  \x1b[36m{bar}\x1b[0m")?;
    } else {
        writeln!(out, "{date_str}")?;
        writeln!(out, "{done} of {total} done  {bar}")?;
    }

    if !tasks.is_empty() {
        writeln!(out)?;
        let days_and_tasks = vec![(date, tasks.to_vec())];
        write_list_human(&days_and_tasks, false, config, out)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct BrokenPipeWriter;
    impl Write for BrokenPipeWriter {
        fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "broken pipe"))
        }
        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "broken pipe"))
        }
    }

    #[test]
    fn test_safe_writer_swallows_broken_pipe() {
        let mut writer = SafeWriter::new(BrokenPipeWriter);
        assert!(!writer.is_broken_pipe());
        assert!(writer.write_line("hello world").is_ok());
        assert!(writer.is_broken_pipe());
        assert!(writer.write_str("another").is_ok());
        assert!(writer.flush().is_ok());
    }

    #[test]
    fn test_safe_writer_normal() {
        let mut buf = Vec::new();
        {
            let mut writer = SafeWriter::new(&mut buf);
            writer.write_line("hello").unwrap();
            writer.write_str("world\n").unwrap();
            writer.flush().unwrap();
        }
        assert_eq!(String::from_utf8(buf).unwrap(), "hello\nworld\n");
    }

    #[test]
    fn test_sanitize_plain_text() {
        assert_eq!(
            sanitize_plain_text("hello\tworld\nnewline\rtest"),
            "hello world newline test"
        );
    }

    #[test]
    fn test_json_output_schemas() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 27).unwrap();
        let todo = Todo {
            id: 1,
            text: "Buy milk".to_string(),
            created_date: today,
            done: false,
            done_at: None,
        };
        let tasks = vec![(1, &todo)];

        let mut buf = Vec::new();
        write_list_json(&[(today, tasks.clone())], false, &mut buf).unwrap();
        let json_str = String::from_utf8(buf).unwrap();
        assert!(json_str.ends_with('\n'));
        let parsed: JsonListDay = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed.schema_version, 1);
        assert_eq!(parsed.date, "2026-10-27");
        assert_eq!(parsed.tasks.len(), 1);
        assert_eq!(parsed.tasks[0].index, 1);
        assert_eq!(parsed.tasks[0].text, "Buy milk");
        assert!(!parsed.tasks[0].done);
        assert_eq!(parsed.tasks[0].done_at, None);

        let mut buf = Vec::new();
        write_today_json(today, 2, 3, &tasks, &mut buf).unwrap();
        let today_json = String::from_utf8(buf).unwrap();
        let parsed_today: JsonToday = serde_json::from_str(&today_json).unwrap();
        assert_eq!(parsed_today.schema_version, 1);
        assert_eq!(parsed_today.percent, 66);
        assert_eq!(parsed_today.done, 2);
        assert_eq!(parsed_today.total, 3);

        let mut buf = Vec::new();
        write_today_waybar(2, 3, &tasks, &mut buf).unwrap();
        let waybar_json = String::from_utf8(buf).unwrap();
        let parsed_waybar: WaybarOutput = serde_json::from_str(&waybar_json).unwrap();
        assert_eq!(parsed_waybar.text, "2/3");
        assert_eq!(parsed_waybar.class, "pending");
        assert_eq!(parsed_waybar.percentage, 66);
    }
}
