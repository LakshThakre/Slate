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
