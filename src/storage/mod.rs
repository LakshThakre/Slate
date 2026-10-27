pub mod migrate;

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::model::Todo;
use migrate::{detect_version, migrate_to_latest, CURRENT_SCHEMA_VERSION};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StorageData {
    pub version: u32,
    pub todos: Vec<Todo>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadResult {
    pub todos: Vec<Todo>,
    pub was_legacy: bool,
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
    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;

    // Check backups/ directory
    let backups_dir = parent.join("backups");
    if let Ok(entries) = fs::read_dir(backups_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if let Some(file_name) = p.file_name().and_then(|n| n.to_str()) {
                if file_name.starts_with("todos-") && file_name.ends_with(".json") {
                    if let Ok(meta) = entry.metadata() {
                        let mtime = meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                        if newest.as_ref().is_none_or(|(t, _)| mtime > *t) {
                            newest = Some((mtime, p));
                        }
                    }
                }
            }
        }
    }

    // Check todos.v0.bak
    let v0_bak = path.with_extension("v0.bak");
    if v0_bak.exists() {
        if let Ok(meta) = v0_bak.metadata() {
            let mtime = meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            if newest.as_ref().is_none_or(|(t, _)| mtime > *t) {
                newest = Some((mtime, v0_bak));
            }
        }
    }

    newest.map(|(_, p)| p)
}

pub fn load_todos(path: &Path) -> Result<LoadResult, LoadError> {
    if !path.exists() {
        return Ok(LoadResult {
            todos: Vec::new(),
            was_legacy: false,
        });
    }

    let data = fs::read_to_string(path).map_err(LoadError::Io)?;
    if data.trim().is_empty() {
        return Ok(LoadResult {
            todos: Vec::new(),
            was_legacy: false,
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
    })
}

pub fn save_todos(
    path: &Path,
    todos: &[Todo],
    is_legacy: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let parent = path
        .parent()
        .ok_or("Cannot determine parent data directory")?;
    fs::create_dir_all(parent)?;

    // If migrating from legacy, back up original once to todos.v0.bak
    if is_legacy && path.exists() {
        let bak_path = path.with_extension("v0.bak");
        if !bak_path.exists() {
            fs::copy(path, &bak_path)?;
        }
    }

    let storage_data = StorageData {
        version: CURRENT_SCHEMA_VERSION,
        todos: todos.to_vec(),
    };

    // Write safely to a temp file in the same directory, then rename atomically
    let tmp_path = parent.join(format!(".todos.json.tmp.{}", std::process::id()));
    let json_str = serde_json::to_string_pretty(&storage_data)?;

    {
        let mut file = fs::File::create(&tmp_path)?;
        file.write_all(json_str.as_bytes())?;
        file.sync_all()?;
    }

    fs::rename(&tmp_path, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
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

        save_todos(&path, &todos, false).unwrap();
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

        save_todos(&path, &todos, false).unwrap();
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
    fn test_legacy_fixture_migrates_and_saves_v1_and_creates_bak() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("todos.json");

        // Copy legacy v0 fixture
        let fixture_bytes = fs::read("tests/fixtures/legacy_v0_todos.json").unwrap();
        fs::write(&path, &fixture_bytes).unwrap();

        // 1. Load legacy fixture
        let loaded = load_todos(&path).unwrap();
        assert!(loaded.was_legacy);
        assert_eq!(loaded.todos.len(), 3);

        // 2. Save migrated v1 file
        let bak_path = path.with_extension("v0.bak");
        assert!(!bak_path.exists());

        save_todos(&path, &loaded.todos, loaded.was_legacy).unwrap();

        // Check todos.v0.bak created once with identical bytes
        assert!(bak_path.exists());
        let bak_bytes = fs::read(&bak_path).unwrap();
        assert_eq!(bak_bytes, fixture_bytes);

        // Check todos.json is now version 1 format
        let new_data: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(new_data["version"], 1);
        assert_eq!(new_data["todos"].as_array().unwrap().len(), 3);

        // 3. Second save doesn't touch .v0.bak
        let bak_meta_before = fs::metadata(&bak_path).unwrap().modified().unwrap();
        save_todos(&path, &loaded.todos, false).unwrap();
        let bak_bytes_after = fs::read(&bak_path).unwrap();
        assert_eq!(bak_bytes_after, fixture_bytes);
        let bak_meta_after = fs::metadata(&bak_path).unwrap().modified().unwrap();
        assert_eq!(bak_meta_before, bak_meta_after);
    }

    #[test]
    fn test_future_version_rejected_file_untouched() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("todos.json");

        let future_json = r#"{"version": 99, "todos": [{"id": 1, "text": "Future", "created_date": "2026-10-27", "done": false}]}"#;
        fs::write(&path, future_json).unwrap();

        let res = load_todos(&path);
        match res {
            Err(LoadError::VersionTooNew {
                file_version,
                supported_version,
                ..
            }) => {
                assert_eq!(file_version, 99);
                assert_eq!(supported_version, 1);
            }
            other => panic!("Expected VersionTooNew, got: {other:?}"),
        }

        // File is byte-identical afterward
        let after = fs::read_to_string(&path).unwrap();
        assert_eq!(after, future_json);
    }

    #[test]
    fn test_corrupt_json_rejected_file_untouched() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("todos.json");

        let corrupt_json = r#"{"version": 1, "todos": [invalid json..."#;
        fs::write(&path, corrupt_json).unwrap();

        let res = load_todos(&path);
        match res {
            Err(LoadError::ParseError { line, column, .. }) => {
                assert_eq!(line, 1);
                assert!(column > 0);
            }
            other => panic!("Expected ParseError, got: {other:?}"),
        }

        // File is byte-identical afterward
        let after = fs::read_to_string(&path).unwrap();
        assert_eq!(after, corrupt_json);
    }

    #[test]
    fn test_unknown_extra_fields_tolerated() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("todos.json");

        let extra_fields_json = r#"{
            "version": 1,
            "extra_root_prop": "ignored",
            "todos": [
                {
                    "id": 1,
                    "text": "Task with extras",
                    "created_date": "2026-10-27",
                    "done": false,
                    "priority": "urgent",
                    "labels": ["dev", "v1"]
                }
            ]
        }"#;
        fs::write(&path, extra_fields_json).unwrap();

        let loaded = load_todos(&path).unwrap();
        assert_eq!(loaded.todos.len(), 1);
        assert_eq!(loaded.todos[0].text, "Task with extras");
    }
}
