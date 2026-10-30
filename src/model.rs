use chrono::{DateTime, Local, NaiveDate};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Todo {
    pub id: u64,
    pub text: String,
    #[serde(alias = "created_at", alias = "date")]
    pub created_date: NaiveDate,
    pub done: bool,
    #[serde(default)]
    pub done_at: Option<DateTime<Local>>,
    #[serde(default)]
    pub priority: u8,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub parent: Option<u64>,
    #[serde(default)]
    pub recurring_id: Option<u64>,
}

impl Default for Todo {
    fn default() -> Self {
        Self {
            id: 0,
            text: String::new(),
            created_date: NaiveDate::from_ymd_opt(1970, 1, 1).unwrap(),
            done: false,
            done_at: None,
            priority: 0,
            tags: Vec::new(),
            note: None,
            parent: None,
            recurring_id: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RecurrenceRule {
    Daily,
    Weekdays,
    Weekly,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Recurring {
    pub id: u64,
    pub text: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub priority: u8,
    pub rule: RecurrenceRule,
    pub anchor: NaiveDate,
    #[serde(default)]
    pub last_materialized: Option<NaiveDate>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Row {
    pub id: u64,
    pub depth: usize,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum SortMode {
    #[default]
    Manual,
    Priority,
}

impl std::fmt::Display for SortMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SortMode::Manual => write!(f, "manual"),
            SortMode::Priority => write!(f, "priority"),
        }
    }
}

impl SortMode {
    pub fn next(self) -> Self {
        match self {
            SortMode::Manual => SortMode::Priority,
            SortMode::Priority => SortMode::Manual,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Filter {
    pub tags: Vec<String>,
    pub min_priority: u8,
}

impl Filter {
    pub fn is_empty(&self) -> bool {
        self.tags.is_empty() && self.min_priority == 0
    }

    pub fn matches(&self, todo: &Todo) -> bool {
        if self.min_priority > 0 && todo.priority < self.min_priority {
            return false;
        }
        for tag in &self.tags {
            if !todo.tags.iter().any(|t| t.eq_ignore_ascii_case(tag)) {
                return false;
            }
        }
        true
    }

    pub fn format_header(&self) -> String {
        let mut parts = Vec::new();
        for tag in &self.tags {
            parts.push(format!("#{tag}"));
        }
        if self.min_priority == 2 {
            parts.push("!!".to_string());
        } else if self.min_priority == 1 {
            parts.push("!".to_string());
        }
        format!("filter: {}", parts.join(" "))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum View {
    #[default]
    Today,
    Upcoming,
    History,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HistoryPane {
    #[default]
    Dates,
    Todos,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Adding,
    Editing(u64),
    Rescheduling(u64),
    Filtering,
    EditingNote(u64),
}
