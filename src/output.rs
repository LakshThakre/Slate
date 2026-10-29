use std::io::{self, Write};

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
