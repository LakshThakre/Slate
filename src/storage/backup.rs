use std::fs;
use std::path::{Path, PathBuf};

use chrono::Local;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupReason {
    ExternalChange,
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
    Ok(target_path)
}
