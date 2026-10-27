use serde_json::Value;

pub const CURRENT_SCHEMA_VERSION: u32 = 1;

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

pub fn migrate_to_latest(mut value: Value) -> Result<(Value, bool), String> {
    let mut version = detect_version(&value)?;
    let mut was_legacy = false;

    if version == 0 {
        was_legacy = true;
        migrate_v0_to_v1(&mut value)?;
        version = 1;
    }

    if version > CURRENT_SCHEMA_VERSION {
        return Err(format!(
            "Schema version {version} is newer than supported version {CURRENT_SCHEMA_VERSION}"
        ));
    }

    Ok((value, was_legacy))
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
    }

    #[test]
    fn test_migrate_v0_array_to_v1() {
        let val = serde_json::json!([
            {"id": 1, "text": "Task 1", "created_date": "2026-10-27", "done": false}
        ]);
        let (migrated, was_legacy) = migrate_to_latest(val.clone()).unwrap();
        assert!(was_legacy);
        assert_eq!(migrated["version"], 1);
        assert_eq!(migrated["todos"].as_array().unwrap().len(), 1);
    }
}
