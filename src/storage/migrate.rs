use serde_json::Value;

pub const CURRENT_SCHEMA_VERSION: u32 = 2;

pub fn detect_version(value: &Value) -> Result<u32, String> {
    if value.is_array() {
        return Ok(0);
    }
    if let Some(obj) = value.as_object() {
        if let Some(ver_val) = obj.get("version") {
            if let Some(v) = ver_val.as_u64() {
                return Ok(v as u32);
            } else {
                return Err("Schema version field is not an integer".to_string());
            }
        }
        return Ok(0);
    }
    Err("Invalid JSON root: expected array or object".to_string())
}

pub fn migrate_v0_to_v1(value: &mut Value) -> Result<(), String> {
    if value.is_array() {
        let arr = std::mem::replace(value, Value::Null);
        *value = serde_json::json!({
            "version": 1,
            "todos": arr,
        });
        return Ok(());
    }
    if let Some(obj) = value.as_object_mut() {
        obj.insert("version".to_string(), serde_json::json!(1));
        if !obj.contains_key("todos") {
            obj.insert("todos".to_string(), serde_json::json!([]));
        }
        return Ok(());
    }
    Err("Failed to migrate v0 to v1: invalid root".to_string())
}

pub fn migrate_v1_to_v2(value: &mut Value) -> Result<(), String> {
    let Some(obj) = value.as_object_mut() else {
        return Err("Failed to migrate v1 to v2: invalid root object".to_string());
    };

    obj.insert("version".to_string(), serde_json::json!(2));

    let todos_val = obj
        .entry("todos".to_string())
        .or_insert_with(|| serde_json::json!([]));

    let mut next_id: u64 = 1;
    if let Some(arr) = todos_val.as_array_mut() {
        for (idx, item) in arr.iter_mut().enumerate() {
            let assigned_id = (idx + 1) as u64;
            if let Some(todo_obj) = item.as_object_mut() {
                todo_obj.insert("id".to_string(), serde_json::json!(assigned_id));
                todo_obj.entry("priority").or_insert(serde_json::json!(0));
                todo_obj.entry("tags").or_insert(serde_json::json!([]));
                todo_obj.entry("note").or_insert(serde_json::Value::Null);
                todo_obj.entry("parent").or_insert(serde_json::Value::Null);
                todo_obj
                    .entry("recurring_id")
                    .or_insert(serde_json::Value::Null);
            }
        }
        next_id = (arr.len() + 1) as u64;
    }

    obj.insert("next_id".to_string(), serde_json::json!(next_id));
    obj.entry("recurring".to_string())
        .or_insert_with(|| serde_json::json!([]));
    obj.entry("focus".to_string())
        .or_insert(serde_json::Value::Null);

    Ok(())
}

pub fn migrate_to_latest(mut value: Value) -> Result<(Value, u32), String> {
    let initial_version = detect_version(&value)?;
    let mut version = initial_version;

    if version == 0 {
        migrate_v0_to_v1(&mut value)?;
        version = 1;
    }

    if version == 1 {
        migrate_v1_to_v2(&mut value)?;
        version = 2;
    }

    if version > CURRENT_SCHEMA_VERSION {
        return Err(format!(
            "Schema version {version} is newer than supported version {CURRENT_SCHEMA_VERSION}"
        ));
    }

    Ok((value, initial_version))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_version_array_is_v0() {
        let val = serde_json::json!([]);
        assert_eq!(detect_version(&val).unwrap(), 0);
    }

    #[test]
    fn test_detect_version_object_without_version_is_v0() {
        let val = serde_json::json!({"todos": []});
        assert_eq!(detect_version(&val).unwrap(), 0);
    }

    #[test]
    fn test_detect_version_object_with_version() {
        let val = serde_json::json!({"version": 1, "todos": []});
        assert_eq!(detect_version(&val).unwrap(), 1);
        let val2 = serde_json::json!({"version": 2, "next_id": 1, "todos": [], "recurring": [], "focus": null});
        assert_eq!(detect_version(&val2).unwrap(), 2);
    }

    #[test]
    fn test_migrate_v0_array_to_v1() {
        let mut val = serde_json::json!([
            {"id": 1, "text": "Task 1", "created_date": "2026-10-27", "done": false}
        ]);
        migrate_v0_to_v1(&mut val).unwrap();
        assert_eq!(val["version"], 1);
        assert_eq!(val["todos"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_migrate_v1_to_v2() {
        let mut val = serde_json::json!({
            "version": 1,
            "todos": [
                {"id": 99, "text": "Task 1", "created_date": "2026-10-27", "done": false},
                {"id": 105, "text": "Task 2", "created_date": "2026-10-28", "done": true}
            ]
        });
        migrate_v1_to_v2(&mut val).unwrap();
        assert_eq!(val["version"], 2);
        assert_eq!(val["next_id"], 3);
        assert_eq!(val["recurring"], serde_json::json!([]));
        assert!(val["focus"].is_null());
        let todos = val["todos"].as_array().unwrap();
        assert_eq!(todos[0]["id"], 1);
        assert_eq!(todos[1]["id"], 2);
        assert_eq!(todos[0]["priority"], 0);
        assert_eq!(todos[0]["tags"], serde_json::json!([]));
    }
}
