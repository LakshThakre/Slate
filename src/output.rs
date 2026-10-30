use std::io::{self, IsTerminal, Write};

use chrono::NaiveDate;

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

#[allow(clippy::type_complexity)]
pub fn write_list_human(
    days_and_tasks: &[(NaiveDate, Vec<(String, usize, &Todo)>)],
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

    let sub_connector = if use_unicode { "└ " } else { "` " };

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

        for (idx, depth, task) in tasks {
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

            if *depth > 0 {
                if use_color {
                    if task.done {
                        writeln!(
                            out,
                            "    {:>4}  \x1b[2m{}\x1b[0m\x1b[32m{}\x1b[0m  \x1b[2m{}{}\x1b[0m",
                            idx, sub_connector, symbol, task.text, time_suffix
                        )?;
                    } else {
                        writeln!(
                            out,
                            "    {:>4}  \x1b[2m{}\x1b[0m\x1b[36m{}\x1b[0m  {}",
                            idx, sub_connector, symbol, task.text
                        )?;
                    }
                } else {
                    writeln!(
                        out,
                        "    {:>4}  {}{}  {}{}",
                        idx, sub_connector, symbol, task.text, time_suffix
                    )?;
                }
            } else if use_color {
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

pub fn write_today_human(
    date: NaiveDate,
    done: usize,
    total: usize,
    tasks: &[(String, usize, &Todo)],
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
}
