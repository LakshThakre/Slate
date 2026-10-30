use std::collections::HashMap;
use std::path::PathBuf;

use ratatui::style::Color;
use serde::{Deserialize, Serialize};

pub trait EnvLookup {
    fn get(&self, key: &str) -> Option<String>;
}

pub struct SystemEnv;

impl EnvLookup for SystemEnv {
    fn get(&self, key: &str) -> Option<String> {
        std::env::var(key).ok()
    }
}

pub struct MapEnv<'a>(pub HashMap<&'a str, String>);

impl EnvLookup for MapEnv<'_> {
    fn get(&self, key: &str) -> Option<String> {
        self.0.get(key).cloned()
    }
}

pub fn get_config_path_with_env(env: &impl EnvLookup) -> Option<PathBuf> {
    if let Some(path_str) = env.get("SLATE_CONFIG") {
        if !path_str.trim().is_empty() {
            return Some(PathBuf::from(path_str.trim()));
        }
    }

    #[cfg(target_os = "macos")]
    {
        dirs::data_dir().map(|p| p.join("slate").join("config.toml"))
    }
    #[cfg(not(target_os = "macos"))]
    {
        dirs::config_dir().map(|p| p.join("slate").join("config.toml"))
    }
}

pub fn get_config_path() -> Option<PathBuf> {
    get_config_path_with_env(&SystemEnv)
}

fn default_theme() -> String {
    "teal".to_string()
}

fn default_date_format() -> String {
    "%A, %d %B".to_string()
}

fn default_short_date_format() -> String {
    "%a %d %b".to_string()
}

fn default_progress_width() -> u16 {
    10
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum TimeFormat {
    #[default]
    #[serde(rename = "24h")]
    TwentyFourHour,
    #[serde(rename = "12h")]
    TwelveHour,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum WeekStart {
    #[default]
    Monday,
    Sunday,
    Saturday,
}

impl WeekStart {
    pub fn weekday(&self) -> chrono::Weekday {
        match self {
            WeekStart::Monday => chrono::Weekday::Mon,
            WeekStart::Sunday => chrono::Weekday::Sun,
            WeekStart::Saturday => chrono::Weekday::Sat,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum SymbolsMode {
    #[default]
    Unicode,
    Ascii,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UiConfig {
    #[serde(default)]
    pub symbols: SymbolsMode,

    #[serde(default = "default_true")]
    pub show_history: bool,

    #[serde(default = "default_true")]
    pub show_upcoming: bool,

    #[serde(default)]
    pub mouse: bool,
}

fn default_true() -> bool {
    true
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            symbols: SymbolsMode::Unicode,
            show_history: true,
            show_upcoming: true,
            mouse: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ProgressStyle {
    #[default]
    Block,
    Line,
    Dots,
    Ascii,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProgressConfig {
    #[serde(default)]
    pub style: ProgressStyle,

    #[serde(default = "default_progress_width")]
    pub width: u16,

    #[serde(default = "default_true")]
    pub show_count: bool,
}

impl Default for ProgressConfig {
    fn default() -> Self {
        Self {
            style: ProgressStyle::Block,
            width: 10,
            show_count: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum KeyMode {
    #[default]
    Default,
    Arrows,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct KeysConfig {
    #[serde(default)]
    pub mode: KeyMode,

    #[serde(flatten)]
    pub overrides: HashMap<String, toml::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TasksConfig {
    #[serde(default)]
    pub sort: crate::model::SortMode,

    #[serde(default = "default_true")]
    pub show_tags: bool,
}

impl Default for TasksConfig {
    fn default() -> Self {
        Self {
            sort: crate::model::SortMode::Manual,
            show_tags: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    #[serde(default = "default_theme")]
    pub theme: String,

    #[serde(default)]
    pub accent: Option<String>,

    #[serde(default = "default_date_format")]
    pub date_format: String,

    #[serde(default = "default_short_date_format")]
    pub short_date_format: String,

    #[serde(default)]
    pub time_format: TimeFormat,

    #[serde(default)]
    pub week_start: WeekStart,

    #[serde(default)]
    pub ui: UiConfig,

    #[serde(default)]
    pub progress: ProgressConfig,

    #[serde(default)]
    pub keys: KeysConfig,

    #[serde(default)]
    pub tasks: TasksConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            accent: None,
            date_format: default_date_format(),
            short_date_format: default_short_date_format(),
            time_format: TimeFormat::TwentyFourHour,
            week_start: WeekStart::Monday,
            ui: UiConfig::default(),
            progress: ProgressConfig::default(),
            keys: KeysConfig::default(),
            tasks: TasksConfig::default(),
        }
    }
}

pub fn validate_strftime(fmt: &str) -> bool {
    if fmt.is_empty() {
        return false;
    }
    use chrono::format::Item;
    for item in chrono::format::strftime::StrftimeItems::new(fmt) {
        if matches!(item, Item::Error) {
            return false;
        }
    }
    true
}

pub fn parse_color(s: &str) -> Result<Color, String> {
    let s_clean = s.trim();
    if let Some(hex_part) = s_clean.strip_prefix('#') {
        if hex_part.len() == 6 {
            let r = u8::from_str_radix(&hex_part[0..2], 16)
                .map_err(|_| format!("invalid hex color '{s_clean}'"))?;
            let g = u8::from_str_radix(&hex_part[2..4], 16)
                .map_err(|_| format!("invalid hex color '{s_clean}'"))?;
            let b = u8::from_str_radix(&hex_part[4..6], 16)
                .map_err(|_| format!("invalid hex color '{s_clean}'"))?;
            return Ok(Color::Rgb(r, g, b));
        } else {
            return Err(format!(
                "hex color '{s_clean}' must be 6 digits (e.g. #4ebaaf)"
            ));
        }
    }

    match s_clean.to_ascii_lowercase().as_str() {
        "black" => Ok(Color::Black),
        "red" => Ok(Color::Red),
        "green" => Ok(Color::Green),
        "yellow" => Ok(Color::Yellow),
        "blue" => Ok(Color::Blue),
        "magenta" => Ok(Color::Magenta),
        "cyan" => Ok(Color::Cyan),
        "gray" | "grey" => Ok(Color::Gray),
        "darkgray" | "darkgrey" | "dark_gray" | "dark-gray" => Ok(Color::DarkGray),
        "white" => Ok(Color::White),
        "lightred" | "light_red" | "light-red" => Ok(Color::LightRed),
        "lightgreen" | "light_green" | "light-green" => Ok(Color::LightGreen),
        "lightyellow" | "light_yellow" | "light-yellow" => Ok(Color::LightYellow),
        "lightblue" | "light_blue" | "light-blue" => Ok(Color::LightBlue),
        "lightmagenta" | "light_magenta" | "light-magenta" => Ok(Color::LightMagenta),
        "lightcyan" | "light_cyan" | "light-cyan" => Ok(Color::LightCyan),
        "reset" => Ok(Color::Reset),
        _ => Err(format!("unknown color '{s_clean}'")),
    }
}

impl Config {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(toml_str: &str) -> (Self, Vec<String>) {
        let mut warnings = Vec::new();
        let mut config = Config::default();

        let val: toml::Value = match toml::from_str(toml_str) {
            Ok(v) => v,
            Err(e) => {
                warnings.push(format!("Config syntax error: {e} · using defaults"));
                return (config, warnings);
            }
        };

        let Some(table) = val.as_table() else {
            warnings.push("Config root must be a table · using defaults".to_string());
            return (config, warnings);
        };

        // Allowed root keys
        let allowed_keys = [
            "theme",
            "accent",
            "date_format",
            "short_date_format",
            "time_format",
            "week_start",
            "ui",
            "progress",
            "keys",
            "tasks",
        ];

        for k in table.keys() {
            if !allowed_keys.contains(&k.as_str()) {
                warnings.push(format!("Unknown config key: '{k}'"));
            }
        }

        // 1. theme
        if let Some(theme_val) = table.get("theme") {
            if let Some(s) = theme_val.as_str() {
                let s_lower = s.to_ascii_lowercase();
                if ["teal", "orange", "lavender", "mono"].contains(&s_lower.as_str()) {
                    config.theme = s_lower;
                } else {
                    warnings.push(format!(
                        "Invalid theme '{s}': expected teal, orange, lavender, mono · using 'teal'"
                    ));
                }
            } else {
                warnings.push("theme must be a string · using 'teal'".to_string());
            }
        }

        // 2. accent
        if let Some(accent_val) = table.get("accent") {
            if let Some(s) = accent_val.as_str() {
                match parse_color(s) {
                    Ok(_) => config.accent = Some(s.to_string()),
                    Err(e) => {
                        warnings.push(format!("Invalid accent: {e} · using preset accent"));
                    }
                }
            } else {
                warnings.push("accent must be a string · using preset accent".to_string());
            }
        }

        // 3. date_format
        if let Some(df_val) = table.get("date_format") {
            if let Some(s) = df_val.as_str() {
                if validate_strftime(s) {
                    config.date_format = s.to_string();
                } else {
                    warnings.push(format!(
                        "Invalid date_format '{s}' · using default '{}'",
                        config.date_format
                    ));
                }
            } else {
                warnings.push("date_format must be a string".to_string());
            }
        }

        // 4. short_date_format
        if let Some(sdf_val) = table.get("short_date_format") {
            if let Some(s) = sdf_val.as_str() {
                if validate_strftime(s) {
                    config.short_date_format = s.to_string();
                } else {
                    warnings.push(format!(
                        "Invalid short_date_format '{s}' · using default '{}'",
                        config.short_date_format
                    ));
                }
            } else {
                warnings.push("short_date_format must be a string".to_string());
            }
        }

        // 5. time_format
        if let Some(tf_val) = table.get("time_format") {
            if let Some(s) = tf_val.as_str() {
                match s.to_ascii_lowercase().as_str() {
                    "24h" => config.time_format = TimeFormat::TwentyFourHour,
                    "12h" => config.time_format = TimeFormat::TwelveHour,
                    _ => {
                        warnings.push(format!(
                            "Invalid time_format '{s}': expected 24h or 12h · using 24h"
                        ));
                    }
                }
            } else {
                warnings.push("time_format must be a string".to_string());
            }
        }

        // 6. week_start
        if let Some(ws_val) = table.get("week_start") {
            if let Some(s) = ws_val.as_str() {
                match s.to_ascii_lowercase().as_str() {
                    "monday" => config.week_start = WeekStart::Monday,
                    "sunday" => config.week_start = WeekStart::Sunday,
                    "saturday" => config.week_start = WeekStart::Saturday,
                    _ => {
                        warnings.push(format!(
                            "Invalid week_start '{s}': expected monday, sunday, saturday · using monday"
                        ));
                    }
                }
            } else {
                warnings.push("week_start must be a string".to_string());
            }
        }

        // 7. [ui]
        if let Some(ui_val) = table.get("ui") {
            if let Some(ui_table) = ui_val.as_table() {
                let allowed_ui_keys = ["symbols", "show_history", "show_upcoming", "mouse"];
                for k in ui_table.keys() {
                    if !allowed_ui_keys.contains(&k.as_str()) {
                        warnings.push(format!("Unknown ui config key: '{k}'"));
                    }
                }

                if let Some(sym_val) = ui_table.get("symbols") {
                    if let Some(s) = sym_val.as_str() {
                        match s.to_ascii_lowercase().as_str() {
                            "unicode" => config.ui.symbols = SymbolsMode::Unicode,
                            "ascii" => config.ui.symbols = SymbolsMode::Ascii,
                            _ => {
                                warnings.push(format!(
                                    "Invalid ui.symbols '{s}': expected unicode or ascii · using unicode"
                                ));
                            }
                        }
                    } else {
                        warnings.push("ui.symbols must be a string".to_string());
                    }
                }

                if let Some(sh_val) = ui_table.get("show_history") {
                    if let Some(b) = sh_val.as_bool() {
                        config.ui.show_history = b;
                    } else {
                        warnings.push("ui.show_history must be a boolean · using true".to_string());
                    }
                }

                if let Some(su_val) = ui_table.get("show_upcoming") {
                    if let Some(b) = su_val.as_bool() {
                        config.ui.show_upcoming = b;
                    } else {
                        warnings
                            .push("ui.show_upcoming must be a boolean · using true".to_string());
                    }
                }

                if let Some(m_val) = ui_table.get("mouse") {
                    if let Some(b) = m_val.as_bool() {
                        config.ui.mouse = b;
                    } else {
                        warnings.push("ui.mouse must be a boolean · using false".to_string());
                    }
                }
            } else {
                warnings.push("[ui] must be a table".to_string());
            }
        }

        // 8. [progress]
        if let Some(prog_val) = table.get("progress") {
            if let Some(prog_table) = prog_val.as_table() {
                let allowed_prog_keys = ["style", "width", "show_count"];
                for k in prog_table.keys() {
                    if !allowed_prog_keys.contains(&k.as_str()) {
                        warnings.push(format!("Unknown progress config key: '{k}'"));
                    }
                }

                if let Some(st_val) = prog_table.get("style") {
                    if let Some(s) = st_val.as_str() {
                        match s.to_ascii_lowercase().as_str() {
                            "block" => config.progress.style = ProgressStyle::Block,
                            "line" => config.progress.style = ProgressStyle::Line,
                            "dots" => config.progress.style = ProgressStyle::Dots,
                            "ascii" => config.progress.style = ProgressStyle::Ascii,
                            "none" => config.progress.style = ProgressStyle::None,
                            _ => {
                                warnings.push(format!(
                                    "Invalid progress.style '{s}': expected block, line, dots, ascii, none · using block"
                                ));
                            }
                        }
                    } else {
                        warnings.push("progress.style must be a string".to_string());
                    }
                }

                if let Some(w_val) = prog_table.get("width") {
                    if let Some(w) = w_val.as_integer() {
                        if (4..=40).contains(&w) {
                            config.progress.width = w as u16;
                        } else {
                            let clamped = w.clamp(4, 40) as u16;
                            warnings.push(format!(
                                "progress.width {w} out of range 4..=40 · clamped to {clamped}"
                            ));
                            config.progress.width = clamped;
                        }
                    } else {
                        warnings.push("progress.width must be an integer".to_string());
                    }
                }

                if let Some(sc_val) = prog_table.get("show_count") {
                    if let Some(b) = sc_val.as_bool() {
                        config.progress.show_count = b;
                    } else {
                        warnings.push("progress.show_count must be a boolean".to_string());
                    }
                }
            } else {
                warnings.push("[progress] must be a table".to_string());
            }
        }

        // 9. [keys]
        if let Some(keys_val) = table.get("keys") {
            if let Some(keys_table) = keys_val.as_table() {
                if let Some(mode_val) = keys_table.get("mode") {
                    if let Some(s) = mode_val.as_str() {
                        match s.to_ascii_lowercase().as_str() {
                            "default" => config.keys.mode = KeyMode::Default,
                            "arrows" => config.keys.mode = KeyMode::Arrows,
                            _ => {
                                warnings.push(format!(
                                    "Invalid keys.mode '{s}': expected default or arrows · using default"
                                ));
                            }
                        }
                    } else {
                        warnings.push("keys.mode must be a string".to_string());
                    }
                }

                for (action_key, action_val) in keys_table {
                    if action_key == "mode" {
                        continue;
                    }
                    config
                        .keys
                        .overrides
                        .insert(action_key.clone(), action_val.clone());
                }
            } else {
                warnings.push("[keys] must be a table".to_string());
            }
        }

        // 10. [tasks]
        if let Some(tasks_val) = table.get("tasks") {
            if let Some(tasks_table) = tasks_val.as_table() {
                let allowed_tasks_keys = ["sort", "show_tags"];
                for k in tasks_table.keys() {
                    if !allowed_tasks_keys.contains(&k.as_str()) {
                        warnings.push(format!("Unknown tasks config key: '{k}'"));
                    }
                }

                if let Some(sort_val) = tasks_table.get("sort") {
                    if let Some(s) = sort_val.as_str() {
                        match s.to_ascii_lowercase().as_str() {
                            "manual" => config.tasks.sort = crate::model::SortMode::Manual,
                            "priority" => config.tasks.sort = crate::model::SortMode::Priority,
                            _ => {
                                warnings.push(format!(
                                    "Invalid tasks.sort '{s}': expected manual or priority · using manual"
                                ));
                            }
                        }
                    } else {
                        warnings.push("tasks.sort must be a string".to_string());
                    }
                }

                if let Some(st_val) = tasks_table.get("show_tags") {
                    if let Some(b) = st_val.as_bool() {
                        config.tasks.show_tags = b;
                    } else {
                        warnings.push("tasks.show_tags must be a boolean · using true".to_string());
                    }
                }
            } else {
                warnings.push("[tasks] must be a table".to_string());
            }
        }

        (config, warnings)
    }

    pub fn load_from_path_or_env(
        path_override: Option<&std::path::Path>,
        env: &impl EnvLookup,
    ) -> (Self, Vec<String>) {
        let path = match path_override {
            Some(p) => Some(p.to_path_buf()),
            None => get_config_path_with_env(env),
        };
        let Some(path) = path else {
            return (Config::default(), Vec::new());
        };

        if !path.exists() {
            if path_override.is_some() {
                return (
                    Config::default(),
                    vec![format!("Config file not found: {}", path.display())],
                );
            }
            return (Config::default(), Vec::new());
        }

        match std::fs::read_to_string(&path) {
            Ok(content) => Config::from_str(&content),
            Err(e) => (
                Config::default(),
                vec![format!(
                    "Could not read config file {}: {e}",
                    path.display()
                )],
            ),
        }
    }

    pub fn load_with_env(env: &impl EnvLookup) -> (Self, Vec<String>) {
        Self::load_from_path_or_env(None, env)
    }

    pub fn load() -> (Self, Vec<String>) {
        Self::load_with_env(&SystemEnv)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_empty_string_gives_defaults() {
        let (config, warnings) = Config::from_str("");
        assert!(warnings.is_empty());
        assert_eq!(config, Config::default());
        assert_eq!(config.theme, "teal");
        assert_eq!(config.progress.width, 10);
        assert!(config.ui.show_history);
    }

    #[test]
    fn test_config_invalid_toml_syntax_gives_defaults_and_warning() {
        let (config, warnings) = Config::from_str("invalid = [toml");
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("Config syntax error"));
        assert_eq!(config, Config::default());
    }

    #[test]
    fn test_config_unknown_key_warns_and_keeps_rest() {
        let toml_str = r#"
            theme = "lavender"
            mystery_key = "surprise"
        "#;
        let (config, warnings) = Config::from_str(toml_str);
        assert_eq!(config.theme, "lavender");
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("Unknown config key: 'mystery_key'"));
    }

    #[test]
    fn test_config_unknown_theme_preset_warns_and_falls_back() {
        let toml_str = r#"theme = "neon_pink""#;
        let (config, warnings) = Config::from_str(toml_str);
        assert_eq!(config.theme, "teal");
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("Invalid theme 'neon_pink'"));
    }

    #[test]
    fn test_config_invalid_accent_warns_and_falls_back() {
        let toml_str = r#"accent = "not-a-color""#;
        let (config, warnings) = Config::from_str(toml_str);
        assert_eq!(config.accent, None);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("Invalid accent"));
    }

    #[test]
    fn test_config_valid_accent_hex_and_ansi() {
        let toml_str = r##"accent = "#4ebaaf""##;
        let (config, warnings) = Config::from_str(toml_str);
        assert!(warnings.is_empty());
        assert_eq!(config.accent, Some("#4ebaaf".to_string()));

        let toml_ansi = r#"accent = "cyan""#;
        let (config2, warnings2) = Config::from_str(toml_ansi);
        assert!(warnings2.is_empty());
        assert_eq!(config2.accent, Some("cyan".to_string()));
    }

    #[test]
    fn test_config_invalid_strftime_warns_and_falls_back() {
        let toml_str = r#"date_format = "%A, %Q %B""#; // %Q is invalid in strftime
        let (config, warnings) = Config::from_str(toml_str);
        assert_eq!(config.date_format, default_date_format());
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("Invalid date_format"));
    }

    #[test]
    fn test_config_progress_width_clamping() {
        let (config_low, warnings_low) = Config::from_str("progress.width = 1");
        assert_eq!(config_low.progress.width, 4);
        assert_eq!(warnings_low.len(), 1);
        assert!(warnings_low[0].contains("clamped to 4"));

        let (config_high, warnings_high) = Config::from_str("progress.width = 99");
        assert_eq!(config_high.progress.width, 40);
        assert_eq!(warnings_high.len(), 1);
        assert!(warnings_high[0].contains("clamped to 40"));
    }

    #[test]
    fn test_config_path_env_override() {
        let mut map = HashMap::new();
        map.insert("SLATE_CONFIG", "/custom/path/config.toml".to_string());
        let env = MapEnv(map);
        let path = get_config_path_with_env(&env);
        assert_eq!(path, Some(PathBuf::from("/custom/path/config.toml")));
    }

    #[test]
    fn test_week_start_weekday_helper() {
        assert_eq!(WeekStart::Monday.weekday(), chrono::Weekday::Mon);
        assert_eq!(WeekStart::Sunday.weekday(), chrono::Weekday::Sun);
        assert_eq!(WeekStart::Saturday.weekday(), chrono::Weekday::Sat);
    }

    #[test]
    fn test_config_example_file_loads_without_warnings() {
        let example_content = include_str!("../config.example.toml");
        let (config, warnings) = Config::from_str(example_content);
        assert!(
            warnings.is_empty(),
            "Expected config.example.toml to parse cleanly, but got warnings: {warnings:?}"
        );
        assert_eq!(config.theme, "teal");
        assert_eq!(config.date_format, "%A, %d %B");
        assert_eq!(config.short_date_format, "%a %d %b");
        assert_eq!(config.time_format, TimeFormat::TwentyFourHour);
        assert_eq!(config.week_start, WeekStart::Monday);
        assert_eq!(config.ui.symbols, SymbolsMode::Unicode);
        assert!(config.ui.show_history);
        assert!(!config.ui.mouse);
        assert_eq!(config.progress.style, ProgressStyle::Block);
        assert_eq!(config.progress.width, 10);
        assert!(config.progress.show_count);
        assert_eq!(config.keys.mode, KeyMode::Default);
    }
}
