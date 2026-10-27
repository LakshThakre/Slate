use std::fs;
use std::io::Write;
use std::path::PathBuf;

use crate::model::Todo;

pub fn get_storage_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let data_dir = dirs::data_dir().ok_or("Could not determine user data directory")?;
    Ok(data_dir.join("slate").join("todos.json"))
}

pub fn load_todos() -> Result<Vec<Todo>, Box<dyn std::error::Error>> {
    let file_path = get_storage_path()?;
    if !file_path.exists() {
        return Ok(Vec::new());
    }
    let data = fs::read_to_string(&file_path)?;
    if data.trim().is_empty() {
        return Ok(Vec::new());
    }
    let list: Vec<Todo> = serde_json::from_str(&data)?;
    Ok(list)
}

pub fn save_todos(todos: &[Todo]) -> Result<(), Box<dyn std::error::Error>> {
    let file_path = get_storage_path()?;
    let parent = file_path
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

    fs::rename(&tmp_path, &file_path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_load_legacy_todos_json_without_new_fields() {
        // Legacy JSON format: id, text, date (or created_at), done, without done_at
        let legacy_json = r#"[
            {
                "id": 1,
                "text": "Old legacy task 1",
                "date": "2026-10-20",
                "done": false
            },
            {
                "id": 2,
                "text": "Old legacy task 2",
                "created_at": "2026-10-21",
                "done": true
            }
        ]"#;

        let todos: Vec<Todo> =
            serde_json::from_str(legacy_json).expect("Legacy todos should deserialize");
        assert_eq!(todos.len(), 2);

        assert_eq!(todos[0].id, 1);
        assert_eq!(todos[0].text, "Old legacy task 1");
        assert_eq!(
            todos[0].created_date,
            NaiveDate::from_ymd_opt(2026, 10, 20).unwrap()
        );
        assert!(!todos[0].done);
        assert_eq!(todos[0].done_at, None);

        assert_eq!(todos[1].id, 2);
        assert_eq!(todos[1].text, "Old legacy task 2");
        assert_eq!(
            todos[1].created_date,
            NaiveDate::from_ymd_opt(2026, 10, 21).unwrap()
        );
        assert!(todos[1].done);
        assert_eq!(todos[1].done_at, None);
    }
}
