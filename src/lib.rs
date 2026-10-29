pub mod app;
pub mod cli;
pub mod commands;
pub mod config;
pub mod exchange;
pub mod keymap;
pub mod model;
pub mod output;
pub mod storage;
pub mod store;
pub mod theme;
pub mod ui;

use std::path::{Path, PathBuf};

pub fn resolve_data_path_with_env(
    flag: Option<&Path>,
    env: &impl config::EnvLookup,
) -> Result<PathBuf, String> {
    if let Some(p) = flag {
        return Ok(p.to_path_buf());
    }
    if let Some(path_str) = env.get("SLATE_DATA") {
        let trimmed = path_str.trim();
        if !trimmed.is_empty() {
            return Ok(PathBuf::from(trimmed));
        }
    }
    storage::get_storage_path().map_err(|e| e.to_string())
}

pub fn resolve_data_path(flag: Option<&Path>) -> Result<PathBuf, String> {
    resolve_data_path_with_env(flag, &config::SystemEnv)
}
