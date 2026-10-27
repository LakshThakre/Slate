use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::model::Todo;

pub fn get_storage_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let data_dir = dirs::data_dir().ok_or("Could not determine user data directory")?;
    Ok(data_dir.join("slate").join("todos.json"))
}

pub fn load_todos(path: &Path) -> Result<Vec<Todo>, Box<dyn std::error::Error>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let data = fs::read_to_string(path)?;
    if data.trim().is_empty() {
        return Ok(Vec::new());
    }
    let list: Vec<Todo> = serde_json::from_str(&data)?;
    Ok(list)
}

pub fn save_todos(path: &Path, todos: &[Todo]) -> Result<(), Box<dyn std::error::Error>> {
    let parent = path
        .parent()
        .ok_or("Cannot determine parent data directory")?;
    fs::create_dir_all(parent)?;

    // Write safely to a temp file in the same directory, then rename atomically
    let tmp_path = parent.join(format!(".todos.json.tmp.{}", std::process::id()));
    let json_str = serde_json::to_string_pretty(todos)?;

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
        assert!(result.is_empty());
    }

    #[test]
    fn test_load_empty_file_returns_empty_vec() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("empty_todos.json");
        fs::write(&path, "   \n").unwrap();
        let result = load_todos(&path).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_load_invalid_json_returns_error() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("corrupted.json");
        fs::write(&path, "{ invalid: json ]").unwrap();
        let result = load_todos(&path);
        assert!(result.is_err());
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

        save_todos(&path, &todos).unwrap();
        let loaded = load_todos(&path).unwrap();
        assert_eq!(loaded, todos);
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

        save_todos(&path, &todos).unwrap();
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
    fn test_load_legacy_fixture() {
        let fixture_path = PathBuf::from("tests/fixtures/legacy_v0_todos.json");
        let todos = load_todos(&fixture_path).expect("Fixture should load successfully");
        assert_eq!(todos.len(), 3);

        assert_eq!(todos[0].id, 1);
        assert_eq!(todos[0].text, "Review project architecture");
        assert_eq!(
            todos[0].created_date,
            NaiveDate::from_ymd_opt(2026, 10, 25).unwrap()
        );
        assert!(!todos[0].done);
        assert_eq!(todos[0].done_at, None);

        assert_eq!(todos[1].id, 2);
        assert_eq!(todos[1].text, "Buy fresh bread");
        assert!(todos[1].done);
        assert!(todos[1].done_at.is_some());

        assert_eq!(todos[2].id, 3);
        assert_eq!(todos[2].text, "Old legacy date format task");
        assert_eq!(
            todos[2].created_date,
            NaiveDate::from_ymd_opt(2026, 10, 20).unwrap()
        );
        assert!(todos[2].done);
    }
}
