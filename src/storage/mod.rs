pub mod backup;
pub mod lock;
pub mod migrate;

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

use crate::model::{Recurring, Todo};
#[allow(unused_imports)]
pub use backup::{backup_now, BackupReason};
use migrate::{detect_version, migrate_to_latest, CURRENT_SCHEMA_VERSION};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StorageData {
    pub version: u32,
    pub next_id: u64,
    pub todos: Vec<Todo>,
    #[serde(default)]
    pub recurring: Vec<Recurring>,
    #[serde(default)]
    pub focus: Option<u64>,
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
    pub next_id: u64,
    pub recurring: Vec<Recurring>,
    pub focus: Option<u64>,
    pub original_version: u32,
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

    // Check todos.v1.bak
    let v1_bak = path.with_extension("v1.bak");
    if v1_bak.exists() {
        if let Ok(meta) = v1_bak.metadata() {
            let mtime = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
            if newest.as_ref().is_none_or(|(t, _)| mtime > *t) {
                newest = Some((mtime, v1_bak));
            }
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

pub fn repair_in_memory(todos: &mut [Todo], focus: &mut Option<u64>) -> Result<(), String> {
    // 1. Check duplicate IDs
    let mut seen_ids = std::collections::HashSet::new();
    for todo in todos.iter() {
        if !seen_ids.insert(todo.id) {
            return Err(format!("duplicate task id: {}", todo.id));
        }
    }

    // 2. Flatten depth beyond one level & promote orphan subtasks
    let parent_map: std::collections::HashMap<u64, (Option<u64>, chrono::NaiveDate)> = todos
        .iter()
        .map(|t| (t.id, (t.parent, t.created_date)))
        .collect();

    for todo in todos.iter_mut() {
        if let Some(pid) = todo.parent {
            if pid == todo.id {
                todo.parent = None;
                continue;
            }
            match parent_map.get(&pid) {
                Some(&(grandparent, parent_date)) => {
                    if parent_date != todo.created_date {
                        // Parent on different date -> promote to top-level
                        todo.parent = None;
                    } else if let Some(gp_id) = grandparent {
                        // Depth beyond one level -> flatten to root ancestor
                        let mut root_id = gp_id;
                        let mut current_id = gp_id;
                        let mut depth = 0;
                        while let Some(&(next_parent, root_date)) = parent_map.get(&current_id) {
                            if root_date != todo.created_date || depth > 100 {
                                break;
                            }
                            root_id = current_id;
                            if let Some(np) = next_parent {
                                current_id = np;
                                depth += 1;
                            } else {
                                break;
                            }
                        }
                        if let Some(&(None, rdate)) = parent_map.get(&root_id) {
                            if rdate == todo.created_date {
                                todo.parent = Some(root_id);
                            } else {
                                todo.parent = None;
                            }
                        } else {
                            todo.parent = None;
                        }
                    }
                }
                None => {
                    // Parent missing -> promote to top-level
                    todo.parent = None;
                }
            }
        }
    }

    // 3. Clear missing focus ID
    if let Some(fid) = *focus {
        if !todos.iter().any(|t| t.id == fid) {
            *focus = None;
        }
    }

    // 4. Reopen done parent with undone child
    let undone_parent_ids: std::collections::HashSet<u64> = todos
        .iter()
        .filter(|t| !t.done)
        .filter_map(|t| t.parent)
        .collect();

    for todo in todos.iter_mut() {
        if todo.done && undone_parent_ids.contains(&todo.id) {
            todo.done = false;
            todo.done_at = None;
        }
    }

    Ok(())
}

pub fn load_todos(path: &Path) -> Result<LoadResult, LoadError> {
    if !path.exists() {
        return Ok(LoadResult {
            todos: Vec::new(),
            next_id: 1,
            recurring: Vec::new(),
            focus: None,
            original_version: CURRENT_SCHEMA_VERSION,
            was_legacy: false,
            fingerprint: None,
        });
    }

    let data = fs::read_to_string(path).map_err(LoadError::Io)?;
    let fp = FileFingerprint::of(path);

    if data.trim().is_empty() {
        return Ok(LoadResult {
            todos: Vec::new(),
            next_id: 1,
            recurring: Vec::new(),
            focus: None,
            original_version: CURRENT_SCHEMA_VERSION,
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

    let (migrated_val, initial_version) =
        migrate_to_latest(value).map_err(|msg| LoadError::ParseError {
            path: path.to_path_buf(),
            line: 1,
            column: 1,
            message: msg,
            backup_path: find_newest_backup(path),
        })?;

    let mut storage_data: StorageData =
        serde_json::from_value(migrated_val).map_err(|e| LoadError::ParseError {
            path: path.to_path_buf(),
            line: e.line(),
            column: e.column(),
            message: e.to_string(),
            backup_path: find_newest_backup(path),
        })?;

    // In-memory repairs
    repair_in_memory(&mut storage_data.todos, &mut storage_data.focus).map_err(|msg| {
        LoadError::ParseError {
            path: path.to_path_buf(),
            line: 1,
            column: 1,
            message: msg,
            backup_path: find_newest_backup(path),
        }
    })?;

    let max_id = storage_data.todos.iter().map(|t| t.id).max().unwrap_or(0);
    if storage_data.next_id <= max_id {
        storage_data.next_id = max_id + 1;
    }

    Ok(LoadResult {
        todos: storage_data.todos,
        next_id: storage_data.next_id,
        recurring: storage_data.recurring,
        focus: storage_data.focus,
        original_version: initial_version,
        was_legacy: initial_version < CURRENT_SCHEMA_VERSION,
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

pub fn save_storage_data(
    path: &Path,
    data: &StorageData,
    original_version: u32,
    expected_fingerprint: Option<FileFingerprint>,
    is_first_save: bool,
    last_backup_time: Option<SystemTime>,
) -> Result<SaveResult, Box<dyn std::error::Error>> {
    let _lock = lock::acquire_lock(path, Duration::from_secs(3))?;
    save_storage_data_unlocked(
        path,
        data,
        original_version,
        expected_fingerprint,
        is_first_save,
        last_backup_time,
    )
}

pub fn save_todos(
    path: &Path,
    todos: &[Todo],
    is_legacy: bool,
    expected_fingerprint: Option<FileFingerprint>,
    is_first_save: bool,
    last_backup_time: Option<SystemTime>,
) -> Result<SaveResult, Box<dyn std::error::Error>> {
    let next_id = todos.iter().map(|t| t.id).max().unwrap_or(0) + 1;
    let data = StorageData {
        version: CURRENT_SCHEMA_VERSION,
        next_id,
        todos: todos.to_vec(),
        recurring: Vec::new(),
        focus: None,
    };
    save_storage_data(
        path,
        &data,
        if is_legacy { 1 } else { CURRENT_SCHEMA_VERSION },
        expected_fingerprint,
        is_first_save,
        last_backup_time,
    )
}

pub fn load_modify_save<F, R>(
    path: &Path,
    f: F,
) -> Result<(R, SaveResult), Box<dyn std::error::Error>>
where
    F: FnOnce(&mut StorageData) -> R,
{
    try_load_modify_save(path, |data| Ok::<R, std::convert::Infallible>(f(data)))
}

pub fn try_load_modify_save<F, R, E>(
    path: &Path,
    f: F,
) -> Result<(R, SaveResult), Box<dyn std::error::Error>>
where
    F: FnOnce(&mut StorageData) -> Result<R, E>,
    E: Into<Box<dyn std::error::Error>>,
{
    let _lock = lock::acquire_lock(path, Duration::from_secs(3))?;
    let load_res = load_todos(path)?;
    let mut data = StorageData {
        version: CURRENT_SCHEMA_VERSION,
        next_id: load_res.next_id,
        todos: load_res.todos,
        recurring: load_res.recurring,
        focus: load_res.focus,
    };
    let r = f(&mut data).map_err(Into::into)?;
    if let Some(fid) = data.focus {
        if !data.todos.iter().any(|t| t.id == fid && !t.done) {
            data.focus = None;
        }
    }
    let save_res = save_storage_data_unlocked(
        path,
        &data,
        load_res.original_version,
        load_res.fingerprint,
        false,
        None,
    )?;
    Ok((r, save_res))
}

pub fn save_storage_data_unlocked(
    path: &Path,
    data: &StorageData,
    original_version: u32,
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

    if path.exists() {
        if let Some(expected) = expected_fingerprint {
            if let Some(actual) = FileFingerprint::of(path) {
                if actual != expected {
                    external_change = true;
                    if let Err(e) = backup::backup_now(path, BackupReason::ExternalChange) {
                        backup_warning = Some(format!("Backup failed: {e}"));
                    }
                }
            }
        }

        if !external_change {
            if let Some(reason) =
                backup::should_backup(is_first_save, false, last_backup_time, SystemTime::now())
            {
                if let Err(e) = backup::backup_now(path, reason) {
                    backup_warning = Some(format!("Backup failed: {e}"));
                }
            }
        }

        // Safety backup for v1 -> v2 or v0 -> v2
        if original_version == 1 {
            let bak_path = path.with_extension("v1.bak");
            if !bak_path.exists() {
                let _ = fs::copy(path, &bak_path);
            }
        } else if original_version == 0 {
            let bak_path = path.with_extension("v0.bak");
            if !bak_path.exists() {
                let _ = fs::copy(path, &bak_path);
            }
        }
    }

    let json_str = serde_json::to_string_pretty(data)?;

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
                ..Default::default()
            },
            Todo {
                id: 2,
                text: "Test round trip 2".to_string(),
                created_date: NaiveDate::from_ymd_opt(2026, 10, 26).unwrap(),
                done: true,
                done_at: None,
                ..Default::default()
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
            ..Default::default()
        }];

        save_todos(&path, &todos, false, None, true, None).unwrap();
        assert!(path.exists());

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

        let slate_tmp1 = dir.path().join(".todos.json.tmp.1234.5678");
        let slate_tmp2 = dir.path().join(".todos.json.tmp.9999.0000");
        fs::write(&slate_tmp1, "stale").unwrap();
        fs::write(&slate_tmp2, "stale").unwrap();

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
            ..Default::default()
        }];
        let res = save_todos(&path, &initial_todos, false, None, true, None).unwrap();
        let fp1 = res.new_fingerprint;

        std::thread::sleep(Duration::from_millis(50));
        fs::write(&path, r#"{"version": 2, "next_id": 100, "todos": [{"id": 99, "text": "External edit", "created_date": "2026-10-27", "done": false}]}"#).unwrap();

        let my_new_todos = vec![Todo {
            id: 1,
            text: "My update".to_string(),
            created_date: NaiveDate::from_ymd_opt(2026, 10, 27).unwrap(),
            done: true,
            done_at: None,
            ..Default::default()
        }];

        let save_res = save_todos(&path, &my_new_todos, false, fp1, false, None).unwrap();
        assert!(save_res.external_change);

        let backups_dir = dir.path().join("backups");
        let backup_file = backup::find_newest_backup(&backups_dir).unwrap();
        let backup_content = fs::read_to_string(backup_file).unwrap();
        assert!(backup_content.contains("External edit"));
    }

    #[test]
    fn test_lock_timeout_returns_error() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("todos.json");

        let _held_lock = lock::acquire_lock(&path, Duration::from_millis(500)).unwrap();

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

        save_todos(&path, &[], false, None, true, None).unwrap();

        let mut handles = Vec::new();
        for thread_idx in 0..2 {
            let path_clone = Arc::clone(&path);
            handles.push(std::thread::spawn(move || {
                for i in 0..50 {
                    loop {
                        let res = load_modify_save(&path_clone, |data| {
                            let next_id = data.next_id;
                            data.next_id += 1;
                            data.todos.push(Todo {
                                id: next_id,
                                text: format!("Thread {thread_idx} task {i}"),
                                created_date: NaiveDate::from_ymd_opt(2026, 10, 27).unwrap(),
                                done: false,
                                done_at: None,
                                ..Default::default()
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
        let ids: std::collections::HashSet<u64> = final_loaded.todos.iter().map(|t| t.id).collect();
        assert_eq!(ids.len(), 100);
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

        fs::set_permissions(&backups_dir, fs::Permissions::from_mode(0o444)).unwrap();

        let todos = vec![Todo {
            id: 1,
            text: "Save despite backup failure".to_string(),
            created_date: NaiveDate::from_ymd_opt(2026, 10, 27).unwrap(),
            done: false,
            done_at: None,
            ..Default::default()
        }];

        let save_res = save_todos(&path, &todos, false, None, true, None).unwrap();
        assert!(save_res.backup_warning.is_some());
        assert!(save_res.backup_warning.unwrap().contains("Backup failed:"));

        let loaded = load_todos(&path).unwrap();
        assert_eq!(loaded.todos.len(), 1);

        let _ = fs::set_permissions(&backups_dir, fs::Permissions::from_mode(0o755));
    }

    #[test]
    fn test_migration_v1_fixture_creates_v1_bak() {
        let dir = tempdir().unwrap();
        let target_file = dir.path().join("todos.json");
        let v1_fixture = Path::new("tests/fixtures/real_v1_todos.json");
        let fixture_content = fs::read_to_string(v1_fixture).unwrap();
        fs::write(&target_file, &fixture_content).unwrap();

        let loaded = load_todos(&target_file).unwrap();
        assert_eq!(loaded.todos.len(), 2);
        assert_eq!(loaded.todos[0].id, 1);
        assert_eq!(loaded.todos[1].id, 2);
        assert_eq!(loaded.next_id, 3);
        assert!(loaded.was_legacy);

        // Save
        let data = StorageData {
            version: CURRENT_SCHEMA_VERSION,
            next_id: loaded.next_id,
            todos: loaded.todos,
            recurring: loaded.recurring,
            focus: loaded.focus,
        };
        save_storage_data(
            &target_file,
            &data,
            loaded.original_version,
            loaded.fingerprint,
            true,
            None,
        )
        .unwrap();

        let bak_file = dir.path().join("todos.v1.bak");
        assert!(bak_file.exists());
        let bak_content = fs::read_to_string(&bak_file).unwrap();
        assert_eq!(bak_content, fixture_content);
    }

    #[test]
    fn test_duplicate_ids_hard_error() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("todos.json");
        fs::write(
            &path,
            r#"{"version": 2, "next_id": 3, "todos": [{"id": 1, "text": "Task A", "created_date": "2026-10-27", "done": false}, {"id": 1, "text": "Task B", "created_date": "2026-10-27", "done": false}]}"#,
        )
        .unwrap();

        let res = load_todos(&path);
        assert!(res.is_err());
        let err_msg = res.unwrap_err().to_string();
        assert!(err_msg.contains("duplicate task id: 1"));
    }

    #[test]
    fn test_in_memory_repairs() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("todos.json");
        // 1: Done parent with undone child (2) -> parent should be reopened
        // 2: Child of 1 (same date)
        // 3: Child of 999 (missing parent) -> promoted to top-level
        // 4: Child of 1 on DIFFERENT date -> promoted to top-level
        // 5: Child of 2 (depth 2) -> flattened to child of 1
        // focus: 888 (missing focus) -> cleared
        fs::write(
            &path,
            r#"{
                "version": 2,
                "next_id": 10,
                "focus": 888,
                "todos": [
                    {"id": 1, "text": "Parent", "created_date": "2026-10-27", "done": true, "parent": null},
                    {"id": 2, "text": "Child", "created_date": "2026-10-27", "done": false, "parent": 1},
                    {"id": 3, "text": "Orphan", "created_date": "2026-10-27", "done": false, "parent": 999},
                    {"id": 4, "text": "WrongDate", "created_date": "2026-10-28", "done": false, "parent": 1},
                    {"id": 5, "text": "Grandchild", "created_date": "2026-10-27", "done": false, "parent": 2}
                ]
            }"#,
        )
        .unwrap();

        let loaded = load_todos(&path).unwrap();
        assert_eq!(loaded.focus, None);
        assert!(!loaded.todos[0].done); // Parent reopened
        assert_eq!(loaded.todos[1].parent, Some(1));
        assert_eq!(loaded.todos[2].parent, None); // Promoted
        assert_eq!(loaded.todos[3].parent, None); // Promoted
        assert_eq!(loaded.todos[4].parent, Some(1)); // Flattened
    }
}
