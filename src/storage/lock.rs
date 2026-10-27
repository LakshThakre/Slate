use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use fs2::FileExt;

#[derive(Debug)]
pub struct FileLock {
    _file: File,
    #[allow(dead_code)]
    pub path: PathBuf,
}

pub fn lock_path_for(data_path: &Path) -> PathBuf {
    PathBuf::from(format!("{}.lock", data_path.display()))
}

pub fn acquire_lock(data_path: &Path, timeout: Duration) -> Result<FileLock, String> {
    let lock_path = lock_path_for(data_path);
    if let Some(parent) = lock_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)
        .map_err(|e| format!("could not open lock file {}: {e}", lock_path.display()))?;

    let start = Instant::now();
    loop {
        match file.try_lock_exclusive() {
            Ok(()) => {
                return Ok(FileLock {
                    _file: file,
                    path: lock_path,
                });
            }
            Err(_) => {
                if start.elapsed() >= timeout {
                    return Err(format!(
                        "could not lock {}: another Slate process is writing",
                        data_path.display()
                    ));
                }
                std::thread::sleep(Duration::from_millis(25));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_lock_and_unlock_basic() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("todos.json");

        let lock1 = acquire_lock(&path, Duration::from_millis(100)).unwrap();
        // Second lock while lock1 held should timeout
        let lock2_err = acquire_lock(&path, Duration::from_millis(100));
        assert!(lock2_err.is_err());
        assert!(lock2_err
            .unwrap_err()
            .contains("another Slate process is writing"));

        // Drop lock1, now lock2 should succeed
        drop(lock1);
        let lock2 = acquire_lock(&path, Duration::from_millis(100));
        assert!(lock2.is_ok());
    }
}
