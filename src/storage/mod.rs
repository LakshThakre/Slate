pub mod backup;
pub mod lock;
pub mod migrate;

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

use crate::model::Todo;
#[allow(unused_imports)]
pub use backup::{backup_now, BackupReason};
use migrate::{detect_version, migrate_to_latest, CURRENT_SCHEMA_VERSION};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StorageData {
    pub version: u32,
    pub todos: Vec<Todo>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileFingerprint {
    pub mtime: SystemTime,
    pub len: u64,
}

impl FileFingerprint {
    pub fn of(path: &Path) -> Option<Self> {
        let meta = fs::metadata(path).ok()?;
        Some(Self {
            mtime: meta.modified().unwrap_or(SystemTime::UNIX_EPOCH),
            len: meta.len(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadResult {
    pub todos: Vec<Todo>,
    pub was_legacy: bool,
    pub fingerprint: Option<FileFingerprint>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveResult {
    pub external_change: bool,
    pub new_fingerprint: Option<FileFingerprint>,
    pub backup_warning: Option<String>,
}

#[derive(Debug)]
pub enum LoadError {
    Io(std::io::Error),
    ParseError {
        path: PathBuf,
        line: usize,
        column: usize,
        message: String,
        backup_path: Option<PathBuf>,
    },
    VersionTooNew {
        path: PathBuf,
        file_version: u32,
        supported_version: u32,
    },
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::Io(e) => write!(f, "I/O error: {e}"),
            LoadError::ParseError {
                path,
                line,
                column,
                message,
                backup_path,
            } => {
                write!(
                    f,
                    "Error loading {}: parse error at line {}, column {}: {}",
                    path.display(),
                    line,
                    column,
                    message
                )?;
                if let Some(bak) = backup_path {
                    write!(f, "\nNewest backup available: {}", bak.display())?;
                }
                Ok(())
            }
            LoadError::VersionTooNew {
                path,
                file_version,
                supported_version,
            } => {
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("todos.json");
                write!(
                    f,
                    "{name} was written by a newer Slate (version {file_version}, this build supports {supported_version}). Upgrade Slate."
                )
            }
        }
    }
}

impl std::error::Error for LoadError {}

pub fn get_storage_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let data_dir = dirs::data_dir().ok_or("Could not determine user data directory")?;
    Ok(data_dir.join("slate").join("todos.json"))
}

pub fn find_newest_backup(path: &Path) -> Option<PathBuf> {
    let parent = path.parent()?;
    let mut newest: Option<(SystemTime, PathBuf)> = None;

    // Check backups/ directory
    let backups_dir = parent.join("backups");
    if let Some(p) = backup::find_newest_backup(&backups_dir) {
        if let Ok(meta) = p.metadata() {
            let mtime = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
            newest = Some((mtime, p));
        }
    }

    // Check todos.v0.bak
    let v0_bak = path.with_extension("v0.bak");
    if v0_bak.exists() {
        if let Ok(meta) = v0_bak.metadata() {
            let mtime = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
            if newest.as_ref().is_none_or(|(t, _)| mtime > *t) {
                newest = Some((mtime, v0_bak));
            }
        }
    }

    newest.map(|(_, p)| p)
}

pub fn clean_stale_temp_files(data_path: &Path) {
    let Some(parent) = data_path.parent() else {
        return;
    };
    let Ok(entries) = fs::read_dir(parent) else {
        return;
    };

    let base_name = data_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("todos.json");
    let prefix = format!(".{base_name}.tmp.");

    for entry in entries.flatten() {
        let p = entry.path();
        if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
            if name.starts_with(&prefix) {
                let _ = fs::remove_file(p);
            }
        }
    }
}

pub fn load_todos(path: &Path) -> Result<LoadResult, LoadError> {
    if !path.exists() {
        return Ok(LoadResult {
            todos: Vec::new(),
            was_legacy: false,
            fingerprint: None,
        });
    }

    let data = fs::read_to_string(path).map_err(LoadError::Io)?;
    let fp = FileFingerprint::of(path);

    if data.trim().is_empty() {
        return Ok(LoadResult {
            todos: Vec::new(),
            was_legacy: false,
            fingerprint: fp,
        });
    }

    let value: serde_json::Value = serde_json::from_str(&data).map_err(|e| {
        let backup_path = find_newest_backup(path);
        LoadError::ParseError {
            path: path.to_path_buf(),
            line: e.line(),
            column: e.column(),
            message: e.to_string(),
            backup_path,
        }
    })?;

    let version = detect_version(&value).map_err(|msg| LoadError::ParseError {
        path: path.to_path_buf(),
        line: 1,
        column: 1,
        message: msg,
        backup_path: find_newest_backup(path),
    })?;

    if version > CURRENT_SCHEMA_VERSION {
        return Err(LoadError::VersionTooNew {
            path: path.to_path_buf(),
            file_version: version,
            supported_version: CURRENT_SCHEMA_VERSION,
        });
    }

    let (migrated_val, was_legacy) =
        migrate_to_latest(value).map_err(|msg| LoadError::ParseError {
            path: path.to_path_buf(),
            line: 1,
            column: 1,
            message: msg,
            backup_path: find_newest_backup(path),
        })?;

    let storage_data: StorageData =
        serde_json::from_value(migrated_val).map_err(|e| LoadError::ParseError {
            path: path.to_path_buf(),
            line: e.line(),
            column: e.column(),
            message: e.to_string(),
            backup_path: find_newest_backup(path),
        })?;

    Ok(LoadResult {
        todos: storage_data.todos,
        was_legacy,
        fingerprint: fp,
    })
}

#[cfg(unix)]
fn sync_parent_dir(path: &Path) {
    if let Some(parent) = path.parent() {
        if let Ok(dir) = fs::File::open(parent) {
            let _ = dir.sync_all();
        }
    }
}

#[cfg(not(unix))]
fn sync_parent_dir(_path: &Path) {}

fn rename_with_retries(from: &Path, to: &Path) -> Result<(), std::io::Error> {
    let mut attempts = 0;
    loop {
        match fs::rename(from, to) {
            Ok(()) => return Ok(()),
            Err(e) => {
                attempts += 1;
                if attempts >= 5 {
                    return Err(e);
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        }
    }
}

pub fn save_todos(
    path: &Path,
    todos: &[Todo],
    is_legacy: bool,
    expected_fingerprint: Option<FileFingerprint>,
    is_first_save: bool,
    last_backup_time: Option<SystemTime>,
) -> Result<SaveResult, Box<dyn std::error::Error>> {
    // 1. Acquire advisory sidecar lock (held only for the duration of this save)
    let _lock = lock::acquire_lock(path, Duration::from_secs(3))?;
    save_todos_unlocked(
        path,
        todos,
        is_legacy,
        expected_fingerprint,
        is_first_save,
        last_backup_time,
    )
}

#[allow(dead_code)]
pub fn load_modify_save<F, R>(
    path: &Path,
    f: F,
) -> Result<(R, SaveResult), Box<dyn std::error::Error>>
where
    F: FnOnce(&mut Vec<Todo>) -> R,
{
    // Hold lock for the duration of load-modify-save
    let _lock = lock::acquire_lock(path, Duration::from_secs(3))?;
    let load_res = load_todos(path)?;
    let mut todos = load_res.todos;
    let r = f(&mut todos);
    let save_res = save_todos_unlocked(
        path,
        &todos,
        load_res.was_legacy,
        load_res.fingerprint,
        false,
        None,
    )?;
    Ok((r, save_res))
}

pub fn try_load_modify_save<F, R, E>(
    path: &Path,
    f: F,
) -> Result<(R, SaveResult), Box<dyn std::error::Error>>
where
    F: FnOnce(&mut Vec<Todo>) -> Result<R, E>,
    E: Into<Box<dyn std::error::Error>>,
{
    // Hold lock for the duration of load-modify-save
    let _lock = lock::acquire_lock(path, Duration::from_secs(3))?;
    let load_res = load_todos(path)?;
    let mut todos = load_res.todos;
    let r = f(&mut todos).map_err(Into::into)?;
    let save_res = save_todos_unlocked(
        path,
        &todos,
        load_res.was_legacy,
        load_res.fingerprint,
        false,
        None,
    )?;
    Ok((r, save_res))
}

pub fn save_todos_unlocked(
    path: &Path,
    todos: &[Todo],
    is_legacy: bool,
    expected_fingerprint: Option<FileFingerprint>,
    is_first_save: bool,
    last_backup_time: Option<SystemTime>,
) -> Result<SaveResult, Box<dyn std::error::Error>> {
    let parent = path
        .parent()
        .ok_or("Cannot determine parent data directory")?;
    fs::create_dir_all(parent)?;

    let mut external_change = false;
    let mut backup_warning = None;

    // 2. Check external change under lock
    if path.exists() {
        if let Some(expected) = expected_fingerprint {
            if let Some(actual) = FileFingerprint::of(path) {
                if actual != expected {
                    external_change = true;
                    // Force backup of on-disk version bypassing throttle
                    if let Err(e) = backup::backup_now(path, BackupReason::ExternalChange) {
                        backup_warning = Some(format!("Backup failed: {e}"));
                    }
                }
            }
        }

        // 3. Rolling backups (if not already backed up by external change)
        if !external_change {
            if let Some(reason) =
                backup::should_backup(is_first_save, false, last_backup_time, SystemTime::now())
            {
                if let Err(e) = backup::backup_now(path, reason) {
                    backup_warning = Some(format!("Backup failed: {e}"));
                }
            }
        }

        // 4. One-time legacy backup to todos.v0.bak
        if is_legacy {
            let bak_path = path.with_extension("v0.bak");
            if !bak_path.exists() {
                let _ = fs::copy(path, &bak_path);
            }
        }
    }

    // 5. Atomic write: write to unique temp file, sync, rename, sync parent
    let storage_data = StorageData {
        version: CURRENT_SCHEMA_VERSION,
        todos: todos.to_vec(),
    };
    let json_str = serde_json::to_string_pretty(&storage_data)?;

    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("todos.json");
    let tmp_path = parent.join(format!(
        ".{}.tmp.{}.{}",
        file_name,
        std::process::id(),
        chrono::Local::now().timestamp_nanos_opt().unwrap_or(0)
    ));

    {
        let mut file = fs::File::create(&tmp_path)?;
        file.write_all(json_str.as_bytes())?;
        file.sync_all()?;
    }

    rename_with_retries(&tmp_path, path)?;
    sync_parent_dir(path);

    let new_fp = FileFingerprint::of(path);

    Ok(SaveResult {
        external_change,
        new_fingerprint: new_fp,
        backup_warning,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use std::sync::Arc;
    use tempfile::tempdir;

    #[test]
    fn test_load_missing_file_returns_empty_vec() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("non_existent_todos.json");
        let result = load_todos(&path).unwrap();
        assert!(result.todos.is_empty());
        assert!(!result.was_legacy);
    }

    #[test]
    fn test_load_empty_file_returns_empty_vec() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("empty_todos.json");
        fs::write(&path, "   \n").unwrap();
        let result = load_todos(&path).unwrap();
        assert!(result.todos.is_empty());
        assert!(!result.was_legacy);
    }

    #[test]
    fn test_save_then_load_round_trip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("todos.json");

        let todos = vec![
            Todo {
                id: 1,
                text: "Test round trip 1".to_string(),
                created_date: NaiveDate::from_ymd_opt(2026, 10, 27).unwrap(),
                done: false,
                done_at: None,
            },
            Todo {
                id: 2,
                text: "Test round trip 2".to_string(),
                created_date: NaiveDate::from_ymd_opt(2026, 10, 26).unwrap(),
                done: true,
                done_at: None,
            },
        ];

        save_todos(&path, &todos, false, None, true, None).unwrap();
        let loaded = load_todos(&path).unwrap();
        assert_eq!(loaded.todos, todos);
        assert!(!loaded.was_legacy);
    }

    #[test]
    fn test_save_leaves_no_temp_file_behind() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("todos.json");

        let todos = vec![Todo {
            id: 1,
            text: "Clean temp test".to_string(),
            created_date: NaiveDate::from_ymd_opt(2026, 10, 27).unwrap(),
            done: false,
            done_at: None,
        }];

        save_todos(&path, &todos, false, None, true, None).unwrap();
        assert!(path.exists());

        // Check parent directory contains no .tmp files
        for entry in fs::read_dir(dir.path()).unwrap() {
            let entry = entry.unwrap();
            let file_name = entry.file_name().to_string_lossy().to_string();
            assert!(
                !file_name.contains(".tmp"),
                "Found unexpected temp file: {file_name}"
            );
        }
    }

    #[test]
    fn test_stale_temp_cleanup_removes_only_matching_files() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("todos.json");

        // Slate's own temp files
        let slate_tmp1 = dir.path().join(".todos.json.tmp.1234.5678");
        let slate_tmp2 = dir.path().join(".todos.json.tmp.9999.0000");
        fs::write(&slate_tmp1, "stale").unwrap();
        fs::write(&slate_tmp2, "stale").unwrap();

        // Other non-matching files
        let other_tmp = dir.path().join("other.tmp");
        let important_file = dir.path().join("notes.txt");
        fs::write(&other_tmp, "keep me").unwrap();
        fs::write(&important_file, "keep me").unwrap();

        clean_stale_temp_files(&path);

        assert!(!slate_tmp1.exists());
        assert!(!slate_tmp2.exists());
        assert!(other_tmp.exists());
        assert!(important_file.exists());
    }

    #[test]
    fn test_external_change_detected_and_backed_up() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("todos.json");

        let initial_todos = vec![Todo {
            id: 1,
            text: "Initial".to_string(),
            created_date: NaiveDate::from_ymd_opt(2026, 10, 27).unwrap(),
            done: false,
            done_at: None,
        }];
        let res = save_todos(&path, &initial_todos, false, None, true, None).unwrap();
        let fp1 = res.new_fingerprint;

        // Simulate external edit to file on disk
        std::thread::sleep(Duration::from_millis(50));
        fs::write(&path, r#"{"version": 1, "todos": [{"id": 99, "text": "External edit", "created_date": "2026-10-27", "done": false}]}"#).unwrap();

        let my_new_todos = vec![Todo {
            id: 1,
            text: "My update".to_string(),
            created_date: NaiveDate::from_ymd_opt(2026, 10, 27).unwrap(),
            done: true,
            done_at: None,
        }];

        // Save with old expected fingerprint
        let save_res = save_todos(&path, &my_new_todos, false, fp1, false, None).unwrap();
        assert!(save_res.external_change);

        // Verify backups directory contains the external edit
        let backups_dir = dir.path().join("backups");
        let backup_file = backup::find_newest_backup(&backups_dir).unwrap();
        let backup_content = fs::read_to_string(backup_file).unwrap();
        assert!(backup_content.contains("External edit"));
    }

    #[test]
    fn test_lock_timeout_returns_error() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("todos.json");

        // Hold sidecar lock externally
        let _held_lock = lock::acquire_lock(&path, Duration::from_millis(500)).unwrap();

        // Trying to save should fail with lock timeout error
        let err = save_todos(&path, &[], false, None, false, None);
        assert!(err.is_err());
        assert!(err
            .unwrap_err()
            .to_string()
            .contains("another Slate process is writing"));
    }

    #[test]
    fn test_concurrent_threads_locked_load_modify_save() {
        let dir = tempdir().unwrap();
        let path = Arc::new(dir.path().join("todos.json"));

        // Initialize empty file
        save_todos(&path, &[], false, None, true, None).unwrap();

        let mut handles = Vec::new();
        for thread_idx in 0..2 {
            let path_clone = Arc::clone(&path);
            handles.push(std::thread::spawn(move || {
                for i in 0..50 {
                    loop {
                        let res = load_modify_save(&path_clone, |todos| {
                            let next_id = (thread_idx * 1000 + i + 1) as u64;
                            todos.push(Todo {
                                id: next_id,
                                text: format!("Thread {thread_idx} task {i}"),
                                created_date: NaiveDate::from_ymd_opt(2026, 10, 27).unwrap(),
                                done: false,
                                done_at: None,
                            });
                        });
                        if res.is_ok() {
                            break;
                        }
                        std::thread::sleep(Duration::from_millis(5));
                    }
                }
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        let final_loaded = load_todos(&path).unwrap();
        assert_eq!(final_loaded.todos.len(), 100);
    }

    #[test]
    #[cfg(unix)]
    fn test_readonly_backups_dir_does_not_block_save() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempdir().unwrap();
        let path = dir.path().join("todos.json");
        fs::write(&path, "initial content").unwrap();

        let backups_dir = dir.path().join("backups");
        fs::create_dir_all(&backups_dir).unwrap();

        // Make backups dir read-only
        fs::set_permissions(&backups_dir, fs::Permissions::from_mode(0o444)).unwrap();

        let todos = vec![Todo {
            id: 1,
            text: "Save despite backup failure".to_string(),
            created_date: NaiveDate::from_ymd_opt(2026, 10, 27).unwrap(),
            done: false,
            done_at: None,
        }];

        // Save should succeed despite backup error
        let save_res = save_todos(&path, &todos, false, None, true, None).unwrap();
        assert!(save_res.backup_warning.is_some());
        assert!(save_res.backup_warning.unwrap().contains("Backup failed:"));

        // File was saved successfully
        let loaded = load_todos(&path).unwrap();
        assert_eq!(loaded.todos.len(), 1);

        // Restore permissions so tempdir cleanup succeeds
        let _ = fs::set_permissions(&backups_dir, fs::Permissions::from_mode(0o755));
    }
}
