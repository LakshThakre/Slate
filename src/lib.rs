pub mod app;
pub mod cli;
pub mod commands;
pub mod config;
pub mod keymap;
pub mod model;
pub mod output;
pub mod parser;
pub mod storage;
pub mod store;
pub mod theme;
pub mod ui;

use std::path::PathBuf;

pub fn resolve_data_path() -> Result<PathBuf, String> {
    storage::get_storage_path().map_err(|e| e.to_string())
}
