use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use chrono::Local;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupReason {
    FirstSaveOfSession,
    Periodic,
    ExternalChange,
    Forced,
}

impl std::fmt::Display for BackupReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackupReason::FirstSaveOfSession => write!(f, "first save of session"),
            BackupReason::Periodic => write!(f, "periodic backup"),
            BackupReason::ExternalChange => write!(f, "external change detected"),
            BackupReason::Forced => write!(f, "manual forced backup"),
        }
    }
}

pub fn should_backup(
    is_first_save: bool,
    forced: bool,
    last_backup_time: Option<SystemTime>,
    now: SystemTime,
) -> Option<BackupReason> {
    if forced {
        return Some(BackupReason::Forced);
    }
    if is_first_save {
        return Some(BackupReason::FirstSaveOfSession);
    }
    if let Some(last_time) = last_backup_time {
        if let Ok(elapsed) = now.duration_since(last_time) {
            if elapsed >= std::time::Duration::from_secs(600) {
                return Some(BackupReason::Periodic);
            }
        }
    } else {
        return Some(BackupReason::FirstSaveOfSession);
    }
    None
}

pub fn find_newest_backup(backups_dir: &Path) -> Option<PathBuf> {
    let entries = fs::read_dir(backups_dir).ok()?;
    let mut snapshots = Vec::new();
    for entry in entries.flatten() {
        let p = entry.path();
        if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
            if name.starts_with("todos-") && name.ends_with(".json") {
                snapshots.push(p);
            }
        }
    }
    snapshots.sort();
    snapshots.pop()
}

pub fn prune_backups(backups_dir: &Path, keep_count: usize) {
    let Ok(entries) = fs::read_dir(backups_dir) else {
        return;
    };

    let mut snapshots = Vec::new();
    for entry in entries.flatten() {
        let p = entry.path();
        if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
            if name.starts_with("todos-") && name.ends_with(".json") {
                snapshots.push(p);
            }
        }
    }

    snapshots.sort();

    if snapshots.len() > keep_count {
        let to_remove = snapshots.len() - keep_count;
        for path in snapshots.iter().take(to_remove) {
            let _ = fs::remove_file(path);
        }
    }
}

pub fn backup_now(path: &Path, _reason: BackupReason) -> Result<PathBuf, std::io::Error> {
    if !path.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Target file does not exist",
        ));
    }

    let parent = path
        .parent()
        .ok_or_else(|| std::io::Error::other("Invalid path parent"))?;
    let backups_dir = parent.join("backups");
    fs::create_dir_all(&backups_dir)?;

    let content = fs::read(path)?;

    // Skip snapshot if the content is byte-identical to the newest one
    if let Some(newest_path) = find_newest_backup(&backups_dir) {
        if let Ok(newest_bytes) = fs::read(&newest_path) {
            if newest_bytes == content {
                return Ok(newest_path);
            }
        }
    }

    let now = Local::now();
    let base_name = format!("todos-{}.json", now.format("%Y%m%d-%H%M%S"));
    let mut target_path = backups_dir.join(&base_name);

    let mut collision_seq = 1;
    while target_path.exists() {
        target_path = backups_dir.join(format!(
            "todos-{}.{}.json",
            now.format("%Y%m%d-%H%M%S"),
            collision_seq
        ));
        collision_seq += 1;
    }

    fs::write(&target_path, content)?;
    prune_backups(&backups_dir, 10);
    Ok(target_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tempfile::tempdir;

    #[test]
    fn test_backup_creates_snapshot_and_prunes_to_10() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("todos.json");
        let backups_dir = dir.path().join("backups");
        fs::write(&path, "initial content 0").unwrap();

        // Take 12 backups with distinct content
        for i in 1..=12 {
            fs::write(&path, format!("content {i}")).unwrap();
            backup_now(&path, BackupReason::Forced).unwrap();
        }

        // Exactly 10 snapshots kept
        let entries: Vec<_> = fs::read_dir(&backups_dir)
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .collect();
        assert_eq!(entries.len(), 10);
    }

    #[test]
    fn test_backup_skips_identical_content() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("todos.json");
        let backups_dir = dir.path().join("backups");
        fs::write(&path, "same content").unwrap();

        let b1 = backup_now(&path, BackupReason::FirstSaveOfSession).unwrap();
        let b2 = backup_now(&path, BackupReason::Periodic).unwrap();
        assert_eq!(b1, b2);

        let count = fs::read_dir(&backups_dir).unwrap().count();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_should_backup_timing_rules() {
        let now = SystemTime::now();

        // First save should backup
        assert_eq!(
            should_backup(true, false, None, now),
            Some(BackupReason::FirstSaveOfSession)
        );

        // Within 10 minutes (e.g. 5m elapsed) should NOT backup
        let five_mins_ago = now - Duration::from_secs(300);
        assert_eq!(should_backup(false, false, Some(five_mins_ago), now), None);

        // After 10 minutes (e.g. 11m elapsed) SHOULD backup
        let eleven_mins_ago = now - Duration::from_secs(660);
        assert_eq!(
            should_backup(false, false, Some(eleven_mins_ago), now),
            Some(BackupReason::Periodic)
        );

        // Forced backup always triggers
        assert_eq!(
            should_backup(false, true, Some(five_mins_ago), now),
            Some(BackupReason::Forced)
        );
    }
}
