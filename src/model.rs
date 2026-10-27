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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Today,
    History,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryPane {
    Dates,
    Todos,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Adding,
    Editing(u64),
}
