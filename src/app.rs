use std::collections::BTreeSet;

use chrono::{Local, NaiveDate};
use ratatui::crossterm::event::{KeyCode, KeyEvent};

use crate::model::{HistoryPane, InputMode, Todo, View};
use crate::storage::{load_todos, save_todos};

pub struct App {
    pub todos: Vec<Todo>,
    pub current_view: View,
    pub history_pane: HistoryPane,
    pub input_mode: InputMode,
    pub input_buffer: String,
    pub cursor_position: usize,
    pub today_selected_index: usize,
    pub history_date_index: usize,
    pub history_todo_index: usize,
    pub status_message: Option<String>,
}

impl App {
    pub fn new() -> Self {
        let (todos, status_message) = match load_todos() {
            Ok(list) => (list, None),
            Err(e) => (Vec::new(), Some(format!("Load warning: {e}"))),
        };

        let mut app = Self {
            todos,
            current_view: View::Today,
            history_pane: HistoryPane::Dates,
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            cursor_position: 0,
            today_selected_index: 0,
            history_date_index: 0,
            history_todo_index: 0,
            status_message,
        };

        app.clamp_selections();
        app
    }

    pub fn save(&mut self) {
        if let Err(e) = save_todos(&self.todos) {
            self.status_message = Some(format!("Save error: {e}"));
        } else if let Some(ref msg) = self.status_message {
            if msg.starts_with("Save error:") {
                self.status_message = None;
            }
        }
    }

    pub fn dates_list(&self) -> Vec<NaiveDate> {
        let today = Local::now().date_naive();
        let mut dates: BTreeSet<NaiveDate> = self.todos.iter().map(|t| t.created_date).collect();
        dates.insert(today);
        let mut list: Vec<NaiveDate> = dates.into_iter().collect();
        list.reverse(); // Newest first
        list
    }

    pub fn clamp_selections(&mut self) {
        let today = Local::now().date_naive();
        let today_count = self
            .todos
            .iter()
            .filter(|t| t.created_date == today)
            .count();
        if today_count == 0 {
            self.today_selected_index = 0;
        } else if self.today_selected_index >= today_count {
            self.today_selected_index = today_count - 1;
        }

        let dates = self.dates_list();
        if dates.is_empty() {
            self.history_date_index = 0;
        } else if self.history_date_index >= dates.len() {
            self.history_date_index = dates.len() - 1;
        }

        if let Some(&selected_date) = dates.get(self.history_date_index) {
            let date_todos_count = self
                .todos
                .iter()
                .filter(|t| t.created_date == selected_date)
                .count();
            if date_todos_count == 0 {
                self.history_todo_index = 0;
            } else if self.history_todo_index >= date_todos_count {
                self.history_todo_index = date_todos_count - 1;
            }
        } else {
            self.history_todo_index = 0;
        }
    }

    pub fn add_todo(&mut self, text: &str) {
        let clean_text = text.trim();
        if clean_text.is_empty() {
            return;
        }

        let today = Local::now().date_naive();
        let next_id = self.todos.iter().map(|t| t.id).max().unwrap_or(0) + 1;
        let new_todo = Todo {
            id: next_id,
            text: clean_text.to_string(),
            created_date: today,
            done: false,
            done_at: None,
        };

        self.todos.push(new_todo);
        self.save();

        // Selection lands on the newly added todo
        let today_count = self
            .todos
            .iter()
            .filter(|t| t.created_date == today)
            .count();
        self.today_selected_index = today_count.saturating_sub(1);
        self.clamp_selections();
    }

    pub fn toggle_today_todo(&mut self) {
        let today = Local::now().date_naive();
        let today_ids: Vec<u64> = self
            .todos
            .iter()
            .filter(|t| t.created_date == today)
            .map(|t| t.id)
            .collect();

        if let Some(&target_id) = today_ids.get(self.today_selected_index) {
            if let Some(todo) = self.todos.iter_mut().find(|t| t.id == target_id) {
                todo.done = !todo.done;
                todo.done_at = if todo.done { Some(Local::now()) } else { None };
            }
            self.save();
        }
    }

    pub fn delete_today_todo(&mut self) {
        let today = Local::now().date_naive();
        let today_ids: Vec<u64> = self
            .todos
            .iter()
            .filter(|t| t.created_date == today)
            .map(|t| t.id)
            .collect();

        if let Some(&target_id) = today_ids.get(self.today_selected_index) {
            self.todos.retain(|t| t.id != target_id);
            self.save();
            self.clamp_selections();
        }
    }

    pub fn toggle_history_todo(&mut self) {
        let dates = self.dates_list();
        if let Some(&selected_date) = dates.get(self.history_date_index) {
            let date_ids: Vec<u64> = self
                .todos
                .iter()
                .filter(|t| t.created_date == selected_date)
                .map(|t| t.id)
                .collect();

            if let Some(&target_id) = date_ids.get(self.history_todo_index) {
                if let Some(todo) = self.todos.iter_mut().find(|t| t.id == target_id) {
                    todo.done = !todo.done;
                    todo.done_at = if todo.done { Some(Local::now()) } else { None };
                }
                self.save();
            }
        }
    }

    pub fn delete_history_todo(&mut self) {
        let dates = self.dates_list();
        if let Some(&selected_date) = dates.get(self.history_date_index) {
            let date_ids: Vec<u64> = self
                .todos
                .iter()
                .filter(|t| t.created_date == selected_date)
                .map(|t| t.id)
                .collect();

            if let Some(&target_id) = date_ids.get(self.history_todo_index) {
                self.todos.retain(|t| t.id != target_id);
                self.save();
                self.clamp_selections();
            }
        }
    }

    /// Handles a key event. Returns true if the app should exit.
    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        match self.input_mode {
            InputMode::Adding => self.handle_adding_key(key),
            InputMode::Normal => self.handle_normal_key(key),
        }
    }

    fn handle_adding_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.input_buffer.clear();
                self.cursor_position = 0;
            }
            KeyCode::Enter => {
                let text = self.input_buffer.clone();
                self.input_mode = InputMode::Normal;
                self.input_buffer.clear();
                self.cursor_position = 0;
                self.add_todo(&text);
            }
            KeyCode::Left => {
                self.cursor_position = self.cursor_position.saturating_sub(1);
            }
            KeyCode::Right => {
                let char_len = self.input_buffer.chars().count();
                if self.cursor_position < char_len {
                    self.cursor_position += 1;
                }
            }
            KeyCode::Home => {
                self.cursor_position = 0;
            }
            KeyCode::End => {
                self.cursor_position = self.input_buffer.chars().count();
            }
            KeyCode::Backspace => {
                if self.cursor_position > 0 {
                    let mut chars: Vec<char> = self.input_buffer.chars().collect();
                    chars.remove(self.cursor_position - 1);
                    self.input_buffer = chars.into_iter().collect();
                    self.cursor_position -= 1;
                }
            }
            KeyCode::Delete => {
                let mut chars: Vec<char> = self.input_buffer.chars().collect();
                if self.cursor_position < chars.len() {
                    chars.remove(self.cursor_position);
                    self.input_buffer = chars.into_iter().collect();
                }
            }
            KeyCode::Char(c) => {
                let mut chars: Vec<char> = self.input_buffer.chars().collect();
                chars.insert(self.cursor_position, c);
                self.input_buffer = chars.into_iter().collect();
                self.cursor_position += 1;
            }
            _ => {}
        }
        false
    }

    fn handle_normal_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char('q') => return true,
            KeyCode::Tab | KeyCode::BackTab => {
                self.current_view = match self.current_view {
                    View::Today => View::History,
                    View::History => View::Today,
                };
                self.clamp_selections();
            }
            KeyCode::Char('a') => {
                self.current_view = View::Today;
                self.input_mode = InputMode::Adding;
                self.input_buffer.clear();
                self.cursor_position = 0;
            }
            _ => match self.current_view {
                View::Today => self.handle_today_key(key),
                View::History => self.handle_history_key(key),
            },
        }
        false
    }

    fn handle_today_key(&mut self, key: KeyEvent) {
        let today = Local::now().date_naive();
        let count = self
            .todos
            .iter()
            .filter(|t| t.created_date == today)
            .count();

        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if count > 0 && self.today_selected_index + 1 < count {
                    self.today_selected_index += 1;
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.today_selected_index = self.today_selected_index.saturating_sub(1);
            }
            KeyCode::Char(' ') | KeyCode::Enter => {
                self.toggle_today_todo();
            }
            KeyCode::Char('d') | KeyCode::Delete => {
                self.delete_today_todo();
            }
            _ => {}
        }
    }

    fn handle_history_key(&mut self, key: KeyEvent) {
        match self.history_pane {
            HistoryPane::Dates => {
                let dates = self.dates_list();
                match key.code {
                    KeyCode::Char('j') | KeyCode::Down => {
                        if !dates.is_empty() && self.history_date_index + 1 < dates.len() {
                            self.history_date_index += 1;
                            self.history_todo_index = 0;
                            self.clamp_selections();
                        }
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        if self.history_date_index > 0 {
                            self.history_date_index -= 1;
                            self.history_todo_index = 0;
                            self.clamp_selections();
                        }
                    }
                    KeyCode::Char('l') | KeyCode::Right | KeyCode::Enter => {
                        self.history_pane = HistoryPane::Todos;
                    }
                    _ => {}
                }
            }
            HistoryPane::Todos => {
                let dates = self.dates_list();
                let selected_date = dates.get(self.history_date_index).copied();
                let count = if let Some(d) = selected_date {
                    self.todos.iter().filter(|t| t.created_date == d).count()
                } else {
                    0
                };

                match key.code {
                    KeyCode::Char('h') | KeyCode::Left => {
                        self.history_pane = HistoryPane::Dates;
                    }
                    KeyCode::Char('j') | KeyCode::Down => {
                        if count > 0 && self.history_todo_index + 1 < count {
                            self.history_todo_index += 1;
                        }
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        self.history_todo_index = self.history_todo_index.saturating_sub(1);
                    }
                    KeyCode::Char(' ') | KeyCode::Enter => {
                        self.toggle_history_todo();
                    }
                    KeyCode::Char('d') | KeyCode::Delete => {
                        self.delete_history_todo();
                    }
                    _ => {}
                }
            }
        }
    }
}
