use std::collections::BTreeSet;

use chrono::{Local, NaiveDate};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::model::{HistoryPane, InputMode, Todo, View};
use crate::storage::{load_todos, save_todos};

use std::path::PathBuf;

pub struct App {
    pub data_path: PathBuf,
    pub today: NaiveDate,
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
    pub undo_stack: Vec<Vec<Todo>>,
    pub redo_stack: Vec<Vec<Todo>>,
    pub transient_message: Option<String>,
    pub visible_list_height: usize,
    pub show_help: bool,
    pub auto_save: bool,
    pub is_legacy_format: bool,
    pub fingerprint: Option<crate::storage::FileFingerprint>,
    pub is_first_save: bool,
    pub last_backup_time: Option<std::time::SystemTime>,
    pub terminal_width: u16,
    pub terminal_height: u16,
    pub selected_history_date: Option<NaiveDate>,
    pub config: crate::config::Config,
    pub theme: crate::theme::Theme,
    pub symbols: crate::ui::Symbols,
    pub keymap: crate::keymap::Keymap,
    pub hit_areas: Vec<crate::ui::HitArea>,
}

impl App {
    pub fn load_or_exit() -> Self {
        let (config, warnings) = crate::config::Config::load();
        Self::load_or_exit_with_config(config, warnings)
    }

    pub fn load_or_exit_with_config(config: crate::config::Config, warnings: Vec<String>) -> Self {
        let data_path =
            crate::storage::get_storage_path().unwrap_or_else(|_| PathBuf::from("todos.json"));
        let today = Local::now().date_naive();
        match Self::try_with_config_and_settings(data_path, today, config, warnings) {
            Ok(app) => app,
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        }
    }

    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::load_or_exit()
    }

    #[allow(dead_code)]
    pub fn with_config(data_path: PathBuf, today: NaiveDate) -> Self {
        Self::try_with_config(data_path, today).unwrap_or_else(|e| {
            eprintln!("{e}");
            std::process::exit(1);
        })
    }

    pub fn try_with_config(
        data_path: PathBuf,
        today: NaiveDate,
    ) -> Result<Self, crate::storage::LoadError> {
        Self::try_with_config_and_settings(
            data_path,
            today,
            crate::config::Config::default(),
            Vec::new(),
        )
    }

    pub fn try_with_config_and_settings(
        data_path: PathBuf,
        today: NaiveDate,
        config: crate::config::Config,
        warnings: Vec<String>,
    ) -> Result<Self, crate::storage::LoadError> {
        crate::storage::clean_stale_temp_files(&data_path);
        let load_res = load_todos(&data_path)?;
        let theme = crate::theme::Theme::from_config(&config);
        let symbols = crate::ui::Symbols::from_config(&config);
        let (keymap, keymap_warnings) = crate::keymap::Keymap::from_config(&config);

        let mut all_warnings = warnings;
        all_warnings.extend(keymap_warnings);

        let mut app = Self {
            data_path,
            today,
            todos: load_res.todos,
            current_view: View::Today,
            history_pane: HistoryPane::Dates,
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            cursor_position: 0,
            today_selected_index: 0,
            history_date_index: 0,
            history_todo_index: 0,
            status_message: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            transient_message: None,
            visible_list_height: 10,
            show_help: false,
            auto_save: true,
            is_legacy_format: load_res.was_legacy,
            fingerprint: load_res.fingerprint,
            is_first_save: true,
            last_backup_time: None,
            terminal_width: 80,
            terminal_height: 24,
            selected_history_date: None,
            config,
            theme,
            symbols,
            keymap,
            hit_areas: Vec::new(),
        };

        if !all_warnings.is_empty() {
            app.set_transient_message(all_warnings.join(" · "));
        }

        app.clamp_selections();
        Ok(app)
    }

    #[cfg(test)]
    pub fn new_for_test(todos: Vec<Todo>) -> Self {
        Self::new_for_test_with_date(todos, Local::now().date_naive())
    }

    #[cfg(test)]
    pub fn new_for_test_with_date(todos: Vec<Todo>, today: NaiveDate) -> Self {
        let mut app = Self {
            data_path: PathBuf::from("dummy_test.json"),
            today,
            todos,
            current_view: View::Today,
            history_pane: HistoryPane::Dates,
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            cursor_position: 0,
            today_selected_index: 0,
            history_date_index: 0,
            history_todo_index: 0,
            status_message: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            transient_message: None,
            visible_list_height: 10,
            show_help: false,
            auto_save: false,
            is_legacy_format: false,
            fingerprint: None,
            is_first_save: true,
            last_backup_time: None,
            terminal_width: 80,
            terminal_height: 24,
            selected_history_date: None,
            config: crate::config::Config::default(),
            theme: crate::theme::Theme::default(),
            symbols: crate::ui::Symbols::unicode(),
            keymap: crate::keymap::Keymap::default(),
            hit_areas: Vec::new(),
        };
        app.clamp_selections();
        app
    }

    pub fn save(&mut self) {
        if !self.auto_save {
            return;
        }
        let res = save_todos(
            &self.data_path,
            &self.todos,
            self.is_legacy_format,
            self.fingerprint,
            self.is_first_save,
            self.last_backup_time,
        );
        match res {
            Ok(save_res) => {
                self.is_legacy_format = false;
                self.is_first_save = false;
                self.fingerprint = save_res.new_fingerprint;
                self.last_backup_time = Some(std::time::SystemTime::now());

                if let Some(warn) = save_res.backup_warning {
                    self.status_message = Some(warn);
                } else if let Some(ref msg) = self.status_message {
                    if msg.starts_with("Save error:") || msg.starts_with("Backup failed:") {
                        self.status_message = None;
                    }
                }

                if save_res.external_change {
                    self.set_transient_message("Data changed on disk · previous version backed up");
                }
            }
            Err(e) => {
                self.status_message = Some(format!("Save error: {e}"));
            }
        }
    }

    pub fn dates_list(&self) -> Vec<NaiveDate> {
        let today = self.today;
        let mut dates: BTreeSet<NaiveDate> = self.todos.iter().map(|t| t.created_date).collect();
        dates.insert(today);
        let mut list: Vec<NaiveDate> = dates.into_iter().collect();
        list.reverse(); // Newest first
        list
    }

    pub fn clamp_selections(&mut self) {
        let today = self.today;
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
            self.selected_history_date = None;
        } else {
            if self.history_date_index >= dates.len() {
                self.history_date_index = dates.len() - 1;
            }
            self.selected_history_date = dates.get(self.history_date_index).copied();
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

    pub fn on_day_changed(&mut self, _old_date: NaiveDate, _new_date: NaiveDate) {
        // Hook for future phases (carry-over, etc.)
    }

    pub fn tick(&mut self, current_today: NaiveDate) -> bool {
        if current_today != self.today {
            let old_date = self.today;
            let prev_selected_date = self.dates_list().get(self.history_date_index).copied();

            self.today = current_today;
            self.on_day_changed(old_date, current_today);

            if let Some(target_date) = prev_selected_date {
                let new_dates = self.dates_list();
                if let Some(pos) = new_dates.iter().position(|&d| d == target_date) {
                    self.history_date_index = pos;
                    self.selected_history_date = Some(target_date);
                }
            }

            self.clamp_selections();
            self.set_transient_message(format!("New day · {}", current_today.format("%A, %-d %B")));
            true
        } else {
            false
        }
    }

    pub fn on_resize(&mut self, width: u16, height: u16) {
        self.terminal_width = width;
        self.terminal_height = height;
        self.clamp_selections();
    }

    pub fn is_too_small(&self) -> bool {
        crate::ui::is_too_small(self.terminal_width, self.terminal_height)
    }

    pub fn set_transient_message(&mut self, msg: impl Into<String>) {
        self.transient_message = Some(msg.into());
    }

    pub fn mutate<F>(&mut self, f: F)
    where
        F: FnOnce(&mut Vec<Todo>),
    {
        self.undo_stack.push(self.todos.clone());
        if self.undo_stack.len() > 50 {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
        f(&mut self.todos);
        self.save();
        self.clamp_selections();
    }

    pub fn undo(&mut self) {
        if let Some(prev) = self.undo_stack.pop() {
            self.redo_stack.push(self.todos.clone());
            if self.redo_stack.len() > 50 {
                self.redo_stack.remove(0);
            }
            self.todos = prev;
            self.save();
            self.clamp_selections();
            self.set_transient_message("Undone");
        } else {
            self.set_transient_message("Nothing to undo");
        }
    }

    pub fn redo(&mut self) {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(self.todos.clone());
            if self.undo_stack.len() > 50 {
                self.undo_stack.remove(0);
            }
            self.todos = next;
            self.save();
            self.clamp_selections();
            self.set_transient_message("Redone");
        } else {
            self.set_transient_message("Nothing to redo");
        }
    }

    pub fn start_editing_today(&mut self) {
        let today = self.today;
        let today_todos: Vec<&Todo> = self
            .todos
            .iter()
            .filter(|t| t.created_date == today)
            .collect();
        if let Some(todo) = today_todos.get(self.today_selected_index) {
            self.input_mode = InputMode::Editing(todo.id);
            self.input_buffer = todo.text.clone();
            self.cursor_position = self.input_buffer.chars().count();
        }
    }

    pub fn start_editing_history(&mut self) {
        let dates = self.dates_list();
        if let Some(&selected_date) = dates.get(self.history_date_index) {
            let date_todos: Vec<&Todo> = self
                .todos
                .iter()
                .filter(|t| t.created_date == selected_date)
                .collect();
            if let Some(todo) = date_todos.get(self.history_todo_index) {
                self.input_mode = InputMode::Editing(todo.id);
                self.input_buffer = todo.text.clone();
                self.cursor_position = self.input_buffer.chars().count();
            }
        }
    }

    pub fn uncarried_previous_tasks(&self) -> Option<(NaiveDate, Vec<u64>)> {
        let today = self.today;
        let prev_date = self
            .todos
            .iter()
            .map(|t| t.created_date)
            .filter(|&d| d < today)
            .max()?;
        let unfinished_ids: Vec<u64> = self
            .todos
            .iter()
            .filter(|t| t.created_date == prev_date && !t.done)
            .map(|t| t.id)
            .collect();
        if unfinished_ids.is_empty() {
            None
        } else {
            Some((prev_date, unfinished_ids))
        }
    }

    pub fn carry_over(&mut self) {
        if let Some((_, ids)) = self.uncarried_previous_tasks() {
            let today = self.today;
            let count = ids.len();
            self.mutate(|todos| {
                let mut carried = Vec::new();
                let mut i = 0;
                while i < todos.len() {
                    if ids.contains(&todos[i].id) {
                        let mut t = todos.remove(i);
                        t.created_date = today;
                        carried.push(t);
                    } else {
                        i += 1;
                    }
                }
                todos.extend(carried);
            });
            self.set_transient_message(format!(
                "Carried over {count} unfinished task{} · u to undo",
                if count == 1 { "" } else { "s" }
            ));
        }
    }

    pub fn reorder_today(&mut self, down: bool) {
        let today = self.today;
        let today_indices: Vec<usize> = self
            .todos
            .iter()
            .enumerate()
            .filter(|(_, t)| t.created_date == today)
            .map(|(i, _)| i)
            .collect();

        if today_indices.len() <= 1 {
            return;
        }

        if down {
            if self.today_selected_index + 1 < today_indices.len() {
                let pos_curr = today_indices[self.today_selected_index];
                let pos_next = today_indices[self.today_selected_index + 1];
                self.mutate(|todos| {
                    todos.swap(pos_curr, pos_next);
                });
                self.today_selected_index += 1;
            }
        } else if self.today_selected_index > 0 {
            let pos_curr = today_indices[self.today_selected_index];
            let pos_prev = today_indices[self.today_selected_index - 1];
            self.mutate(|todos| {
                todos.swap(pos_curr, pos_prev);
            });
            self.today_selected_index -= 1;
        }
    }

    pub fn reorder_history(&mut self, down: bool) {
        let dates = self.dates_list();
        if let Some(&selected_date) = dates.get(self.history_date_index) {
            let date_indices: Vec<usize> = self
                .todos
                .iter()
                .enumerate()
                .filter(|(_, t)| t.created_date == selected_date)
                .map(|(i, _)| i)
                .collect();

            if date_indices.len() <= 1 {
                return;
            }

            if down {
                if self.history_todo_index + 1 < date_indices.len() {
                    let pos_curr = date_indices[self.history_todo_index];
                    let pos_next = date_indices[self.history_todo_index + 1];
                    self.mutate(|todos| {
                        todos.swap(pos_curr, pos_next);
                    });
                    self.history_todo_index += 1;
                }
            } else if self.history_todo_index > 0 {
                let pos_curr = date_indices[self.history_todo_index];
                let pos_prev = date_indices[self.history_todo_index - 1];
                self.mutate(|todos| {
                    todos.swap(pos_curr, pos_prev);
                });
                self.history_todo_index -= 1;
            }
        }
    }

    pub fn clear_completed_today(&mut self) {
        let today = self.today;
        let completed_count = self
            .todos
            .iter()
            .filter(|t| t.created_date == today && t.done)
            .count();

        if completed_count == 0 {
            self.set_transient_message("No completed tasks");
            return;
        }

        self.mutate(|todos| {
            todos.retain(|t| !(t.created_date == today && t.done));
        });
        self.set_transient_message(format!("Cleared {completed_count} completed · u to undo"));
    }

    pub fn clear_completed_history(&mut self) {
        let dates = self.dates_list();
        if let Some(&selected_date) = dates.get(self.history_date_index) {
            let completed_count = self
                .todos
                .iter()
                .filter(|t| t.created_date == selected_date && t.done)
                .count();

            if completed_count == 0 {
                self.set_transient_message("No completed tasks");
                return;
            }

            self.mutate(|todos| {
                todos.retain(|t| !(t.created_date == selected_date && t.done));
            });
            self.set_transient_message(format!("Cleared {completed_count} completed · u to undo"));
        }
    }

    pub fn jump_top(&mut self) {
        match self.current_view {
            View::Today => {
                self.today_selected_index = 0;
            }
            View::History => match self.history_pane {
                HistoryPane::Dates => {
                    self.history_date_index = 0;
                    self.history_todo_index = 0;
                    self.clamp_selections();
                }
                HistoryPane::Todos => {
                    self.history_todo_index = 0;
                }
            },
        }
    }

    pub fn jump_bottom(&mut self) {
        match self.current_view {
            View::Today => {
                let today = self.today;
                let count = self
                    .todos
                    .iter()
                    .filter(|t| t.created_date == today)
                    .count();
                self.today_selected_index = count.saturating_sub(1);
            }
            View::History => match self.history_pane {
                HistoryPane::Dates => {
                    let dates = self.dates_list();
                    self.history_date_index = dates.len().saturating_sub(1);
                    self.history_todo_index = 0;
                    self.clamp_selections();
                }
                HistoryPane::Todos => {
                    let dates = self.dates_list();
                    let count = dates
                        .get(self.history_date_index)
                        .map(|&d| self.todos.iter().filter(|t| t.created_date == d).count())
                        .unwrap_or(0);
                    self.history_todo_index = count.saturating_sub(1);
                }
            },
        }
    }

    pub fn jump_half_page_down(&mut self) {
        let step = (self.visible_list_height / 2).max(1);
        match self.current_view {
            View::Today => {
                let today = self.today;
                let count = self
                    .todos
                    .iter()
                    .filter(|t| t.created_date == today)
                    .count();
                if count > 0 {
                    self.today_selected_index = (self.today_selected_index + step).min(count - 1);
                } else {
                    self.today_selected_index = 0;
                }
            }
            View::History => match self.history_pane {
                HistoryPane::Dates => {
                    let dates = self.dates_list();
                    let count = dates.len();
                    if count > 0 {
                        self.history_date_index = (self.history_date_index + step).min(count - 1);
                    } else {
                        self.history_date_index = 0;
                    }
                    self.history_todo_index = 0;
                    self.clamp_selections();
                }
                HistoryPane::Todos => {
                    let dates = self.dates_list();
                    let count = dates
                        .get(self.history_date_index)
                        .map(|&d| self.todos.iter().filter(|t| t.created_date == d).count())
                        .unwrap_or(0);
                    if count > 0 {
                        self.history_todo_index = (self.history_todo_index + step).min(count - 1);
                    } else {
                        self.history_todo_index = 0;
                    }
                }
            },
        }
    }

    pub fn jump_half_page_up(&mut self) {
        let step = (self.visible_list_height / 2).max(1);
        match self.current_view {
            View::Today => {
                self.today_selected_index = self.today_selected_index.saturating_sub(step);
            }
            View::History => match self.history_pane {
                HistoryPane::Dates => {
                    self.history_date_index = self.history_date_index.saturating_sub(step);
                    self.history_todo_index = 0;
                    self.clamp_selections();
                }
                HistoryPane::Todos => {
                    self.history_todo_index = self.history_todo_index.saturating_sub(step);
                }
            },
        }
    }

    pub fn add_todo(&mut self, text: &str) {
        let clean_text = text.trim();
        if clean_text.is_empty() {
            return;
        }

        let today = self.today;
        let next_id = self.todos.iter().map(|t| t.id).max().unwrap_or(0) + 1;
        let new_todo = Todo {
            id: next_id,
            text: clean_text.to_string(),
            created_date: today,
            done: false,
            done_at: None,
        };

        self.mutate(|todos| {
            todos.push(new_todo);
        });

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
        let today = self.today;
        let today_ids: Vec<u64> = self
            .todos
            .iter()
            .filter(|t| t.created_date == today)
            .map(|t| t.id)
            .collect();

        if let Some(&target_id) = today_ids.get(self.today_selected_index) {
            self.mutate(|todos| {
                if let Some(todo) = todos.iter_mut().find(|t| t.id == target_id) {
                    todo.done = !todo.done;
                    todo.done_at = if todo.done { Some(Local::now()) } else { None };
                }
            });
        }
    }

    pub fn delete_today_todo(&mut self) {
        let today = self.today;
        let today_ids: Vec<u64> = self
            .todos
            .iter()
            .filter(|t| t.created_date == today)
            .map(|t| t.id)
            .collect();

        if let Some(&target_id) = today_ids.get(self.today_selected_index) {
            let task_text = self
                .todos
                .iter()
                .find(|t| t.id == target_id)
                .map(|t| t.text.clone());
            self.mutate(|todos| {
                todos.retain(|t| t.id != target_id);
            });
            if let Some(text) = task_text {
                self.set_transient_message(format!(
                    "Deleted \"{}\" · u to undo",
                    truncate_str(&text, 24)
                ));
            }
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
                self.mutate(|todos| {
                    if let Some(todo) = todos.iter_mut().find(|t| t.id == target_id) {
                        todo.done = !todo.done;
                        todo.done_at = if todo.done { Some(Local::now()) } else { None };
                    }
                });
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
                let task_text = self
                    .todos
                    .iter()
                    .find(|t| t.id == target_id)
                    .map(|t| t.text.clone());
                self.mutate(|todos| {
                    todos.retain(|t| t.id != target_id);
                });
                if let Some(text) = task_text {
                    self.set_transient_message(format!(
                        "Deleted \"{}\" · u to undo",
                        truncate_str(&text, 24)
                    ));
                }
            }
        }
    }

    pub fn move_history_todo_to_today(&mut self) {
        if self.current_view != View::History || self.history_pane != HistoryPane::Todos {
            return;
        }

        let dates = self.dates_list();
        let Some(&selected_date) = dates.get(self.history_date_index) else {
            return;
        };

        let today = self.today;
        if selected_date == today {
            self.set_transient_message("Already in Today");
            return;
        }

        let date_todos: Vec<u64> = self
            .todos
            .iter()
            .filter(|t| t.created_date == selected_date)
            .map(|t| t.id)
            .collect();

        if let Some(&todo_id) = date_todos.get(self.history_todo_index) {
            self.mutate(|todos| {
                if let Some(pos) = todos.iter().position(|t| t.id == todo_id) {
                    let mut todo = todos.remove(pos);
                    todo.created_date = today;
                    todo.done = false;
                    todo.done_at = None;
                    todos.push(todo);
                }
            });
            self.set_transient_message("Moved to Today");
        }
    }

    /// Handles a key event. Returns true if the app should exit.
    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        if self.is_too_small() {
            if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                return true;
            }
            if key.code == KeyCode::Char('q') && self.input_mode == InputMode::Normal {
                return true;
            }
            return false;
        }

        self.transient_message = None;
        match self.input_mode {
            InputMode::Adding => self.handle_adding_key(key),
            InputMode::Editing(id) => self.handle_editing_key(id, key),
            InputMode::Normal => self.handle_normal_key(key),
        }
    }

    pub fn handle_mouse(&mut self, event: ratatui::crossterm::event::MouseEvent) -> bool {
        use ratatui::crossterm::event::{MouseButton, MouseEventKind};

        if !self.config.ui.mouse
            || self.is_too_small()
            || self.show_help
            || self.input_mode != InputMode::Normal
        {
            return false;
        }

        match event.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                let col = event.column;
                let row = event.row;

                // Check checkbox targets first (they are smaller, specific sub-areas of task rows)
                for area in &self.hit_areas {
                    if area.contains(col, row) {
                        match area.target {
                            crate::ui::HitTarget::TodayCheckbox { index } => {
                                self.today_selected_index = index;
                                self.toggle_today_todo();
                                return true;
                            }
                            crate::ui::HitTarget::HistoryCheckbox { index } => {
                                self.history_pane = HistoryPane::Todos;
                                self.history_todo_index = index;
                                self.toggle_history_todo();
                                return true;
                            }
                            _ => {}
                        }
                    }
                }

                // Check other hit targets
                for area in &self.hit_areas {
                    if area.contains(col, row) {
                        match area.target {
                            crate::ui::HitTarget::TabToday => {
                                self.current_view = View::Today;
                                self.clamp_selections();
                                return true;
                            }
                            crate::ui::HitTarget::TabHistory => {
                                if self.config.ui.show_history {
                                    self.current_view = View::History;
                                    self.clamp_selections();
                                    return true;
                                }
                            }
                            crate::ui::HitTarget::TodayTask { index } => {
                                self.today_selected_index = index;
                                return true;
                            }
                            crate::ui::HitTarget::HistoryDate { index } => {
                                self.history_pane = HistoryPane::Dates;
                                self.history_date_index = index;
                                self.history_todo_index = 0;
                                self.clamp_selections();
                                return true;
                            }
                            crate::ui::HitTarget::HistoryTask { index } => {
                                self.history_pane = HistoryPane::Todos;
                                self.history_todo_index = index;
                                return true;
                            }
                            _ => {}
                        }
                    }
                }

                false
            }
            MouseEventKind::ScrollDown => {
                let col = event.column;
                let row = event.row;
                match self.current_view {
                    View::Today => {
                        let count = self
                            .todos
                            .iter()
                            .filter(|t| t.created_date == self.today)
                            .count();
                        if count > 0 && self.today_selected_index + 1 < count {
                            self.today_selected_index += 1;
                            return true;
                        }
                    }
                    View::History => {
                        let in_date_area = self.hit_areas.iter().any(|a| {
                            a.target == crate::ui::HitTarget::DateArea && a.contains(col, row)
                        });
                        let in_todo_area = self.hit_areas.iter().any(|a| {
                            a.target == crate::ui::HitTarget::TodoArea && a.contains(col, row)
                        });

                        let scroll_dates = if in_date_area {
                            true
                        } else if in_todo_area {
                            false
                        } else {
                            self.history_pane == HistoryPane::Dates
                        };

                        if scroll_dates {
                            let dates = self.dates_list();
                            if !dates.is_empty() && self.history_date_index + 1 < dates.len() {
                                self.history_date_index += 1;
                                self.history_todo_index = 0;
                                self.clamp_selections();
                                return true;
                            }
                        } else {
                            let dates = self.dates_list();
                            let selected_date = dates.get(self.history_date_index).copied();
                            let count = if let Some(d) = selected_date {
                                self.todos.iter().filter(|t| t.created_date == d).count()
                            } else {
                                0
                            };
                            if count > 0 && self.history_todo_index + 1 < count {
                                self.history_todo_index += 1;
                                return true;
                            }
                        }
                    }
                }
                false
            }
            MouseEventKind::ScrollUp => {
                let col = event.column;
                let row = event.row;
                match self.current_view {
                    View::Today => {
                        if self.today_selected_index > 0 {
                            self.today_selected_index -= 1;
                            return true;
                        }
                    }
                    View::History => {
                        let in_date_area = self.hit_areas.iter().any(|a| {
                            a.target == crate::ui::HitTarget::DateArea && a.contains(col, row)
                        });
                        let in_todo_area = self.hit_areas.iter().any(|a| {
                            a.target == crate::ui::HitTarget::TodoArea && a.contains(col, row)
                        });

                        let scroll_dates = if in_date_area {
                            true
                        } else if in_todo_area {
                            false
                        } else {
                            self.history_pane == HistoryPane::Dates
                        };

                        if scroll_dates {
                            if self.history_date_index > 0 {
                                self.history_date_index -= 1;
                                self.history_todo_index = 0;
                                self.clamp_selections();
                                return true;
                            }
                        } else if self.history_todo_index > 0 {
                            self.history_todo_index -= 1;
                            return true;
                        }
                    }
                }
                false
            }
            _ => false,
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

    fn handle_editing_key(&mut self, target_id: u64, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.input_buffer.clear();
                self.cursor_position = 0;
            }
            KeyCode::Enter => {
                let clean_text = self.input_buffer.trim().to_string();
                self.input_mode = InputMode::Normal;
                self.input_buffer.clear();
                self.cursor_position = 0;
                if !clean_text.is_empty() {
                    let old_text = self
                        .todos
                        .iter()
                        .find(|t| t.id == target_id)
                        .map(|t| t.text.clone());
                    if let Some(old) = old_text {
                        if old != clean_text {
                            self.mutate(|todos| {
                                if let Some(todo) = todos.iter_mut().find(|t| t.id == target_id) {
                                    todo.text = clean_text;
                                }
                            });
                            self.set_transient_message("Edited task · u to undo");
                        }
                    }
                }
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
        if self.show_help {
            match key.code {
                KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => {
                    self.show_help = false;
                }
                _ => {}
            }
            return false;
        }

        let Some(action) = self.keymap.resolve(&key) else {
            return false;
        };

        match action {
            crate::keymap::Action::Quit => return true,
            crate::keymap::Action::Help => {
                self.show_help = true;
            }
            crate::keymap::Action::SwitchView => {
                if self.config.ui.show_history {
                    self.current_view = match self.current_view {
                        View::Today => View::History,
                        View::History => View::Today,
                    };
                    self.clamp_selections();
                }
            }
            crate::keymap::Action::Add => {
                self.current_view = View::Today;
                self.input_mode = InputMode::Adding;
                self.input_buffer.clear();
                self.cursor_position = 0;
            }
            crate::keymap::Action::Undo => {
                self.undo();
            }
            crate::keymap::Action::Redo => {
                self.redo();
            }
            crate::keymap::Action::JumpTop => {
                self.jump_top();
            }
            crate::keymap::Action::JumpBottom => {
                self.jump_bottom();
            }
            crate::keymap::Action::HalfPageDown => {
                self.jump_half_page_down();
            }
            crate::keymap::Action::HalfPageUp => {
                self.jump_half_page_up();
            }
            _ => match self.current_view {
                View::Today => self.handle_today_action(action),
                View::History => self.handle_history_action(action),
            },
        }

        false
    }

    fn handle_today_action(&mut self, action: crate::keymap::Action) {
        let today = self.today;
        let count = self
            .todos
            .iter()
            .filter(|t| t.created_date == today)
            .count();

        match action {
            crate::keymap::Action::ReorderDown => {
                self.reorder_today(true);
            }
            crate::keymap::Action::ReorderUp => {
                self.reorder_today(false);
            }
            crate::keymap::Action::MoveSelectionDown => {
                if count > 0 && self.today_selected_index + 1 < count {
                    self.today_selected_index += 1;
                }
            }
            crate::keymap::Action::MoveSelectionUp => {
                self.today_selected_index = self.today_selected_index.saturating_sub(1);
            }
            crate::keymap::Action::Toggle => {
                self.toggle_today_todo();
            }
            crate::keymap::Action::Edit => {
                self.start_editing_today();
            }
            crate::keymap::Action::CarryOver => {
                self.carry_over();
            }
            crate::keymap::Action::ClearCompleted => {
                self.clear_completed_today();
            }
            crate::keymap::Action::MoveToToday => {
                self.set_transient_message("Already in Today");
            }
            crate::keymap::Action::Delete => {
                self.delete_today_todo();
            }
            _ => {}
        }
    }

    fn handle_history_action(&mut self, action: crate::keymap::Action) {
        match self.history_pane {
            HistoryPane::Dates => {
                let dates = self.dates_list();
                match action {
                    crate::keymap::Action::MoveSelectionDown => {
                        if !dates.is_empty() && self.history_date_index + 1 < dates.len() {
                            self.history_date_index += 1;
                            self.history_todo_index = 0;
                            self.clamp_selections();
                        }
                    }
                    crate::keymap::Action::MoveSelectionUp => {
                        if self.history_date_index > 0 {
                            self.history_date_index -= 1;
                            self.history_todo_index = 0;
                            self.clamp_selections();
                        }
                    }
                    crate::keymap::Action::FocusTasks | crate::keymap::Action::Toggle => {
                        self.history_pane = HistoryPane::Todos;
                    }
                    crate::keymap::Action::ClearCompleted => {
                        self.clear_completed_history();
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

                match action {
                    crate::keymap::Action::FocusDates => {
                        self.history_pane = HistoryPane::Dates;
                    }
                    crate::keymap::Action::ReorderDown => {
                        self.reorder_history(true);
                    }
                    crate::keymap::Action::ReorderUp => {
                        self.reorder_history(false);
                    }
                    crate::keymap::Action::MoveSelectionDown => {
                        if count > 0 && self.history_todo_index + 1 < count {
                            self.history_todo_index += 1;
                        }
                    }
                    crate::keymap::Action::MoveSelectionUp => {
                        self.history_todo_index = self.history_todo_index.saturating_sub(1);
                    }
                    crate::keymap::Action::Toggle => {
                        self.toggle_history_todo();
                    }
                    crate::keymap::Action::Edit => {
                        self.start_editing_history();
                    }
                    crate::keymap::Action::ClearCompleted => {
                        self.clear_completed_history();
                    }
                    crate::keymap::Action::MoveToToday => {
                        self.move_history_todo_to_today();
                    }
                    crate::keymap::Action::Delete => {
                        self.delete_history_todo();
                    }
                    _ => {}
                }
            }
        }
    }
}

fn truncate_str(text: &str, max_len: usize) -> String {
    let char_count = text.chars().count();
    if char_count <= max_len || max_len == 0 {
        return text.to_string();
    }
    if max_len <= 1 {
        return "…".to_string();
    }
    let take_count = max_len - 1;
    let truncated: String = text.chars().take(take_count).collect();
    format!("{truncated}…")
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::crossterm::event::KeyModifiers;

    fn key_press(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn test_edit_task_today_saves_new_text() {
        let today = Local::now().date_naive();
        let todo1 = Todo {
            id: 1,
            text: "Initial text".to_string(),
            created_date: today,
            done: true,
            done_at: Some(Local::now()),
        };
        let todo2 = Todo {
            id: 2,
            text: "Second task".to_string(),
            created_date: today,
            done: false,
            done_at: None,
        };
        let mut app = App::new_for_test(vec![todo1.clone(), todo2.clone()]);
        app.today_selected_index = 0;

        // Press 'e' to start editing
        app.handle_key(key_press(KeyCode::Char('e')));
        assert_eq!(app.input_mode, InputMode::Editing(1));
        assert_eq!(app.input_buffer, "Initial text");
        assert_eq!(app.cursor_position, "Initial text".chars().count());

        // Replace text in buffer
        app.input_buffer = "Updated text".to_string();
        app.cursor_position = app.input_buffer.chars().count();

        // Press Enter to save
        app.handle_key(key_press(KeyCode::Enter));
        assert_eq!(app.input_mode, InputMode::Normal);
        assert_eq!(app.todos[0].text, "Updated text");
        // State, timestamp, and position are preserved
        assert_eq!(app.todos[0].id, 1);
        assert_eq!(app.todos[0].done, todo1.done);
        assert_eq!(app.todos[0].done_at, todo1.done_at);
        assert_eq!(app.todos[0].created_date, todo1.created_date);
        assert_eq!(app.todos[1].text, "Second task");
    }

    #[test]
    fn test_edit_task_empty_text_keeps_old() {
        let today = Local::now().date_naive();
        let todo = Todo {
            id: 10,
            text: "Original text".to_string(),
            created_date: today,
            done: false,
            done_at: None,
        };
        let mut app = App::new_for_test(vec![todo]);
        app.handle_key(key_press(KeyCode::Char('e')));
        assert_eq!(app.input_mode, InputMode::Editing(10));

        // Set to empty text
        app.input_buffer = "   ".to_string();
        app.handle_key(key_press(KeyCode::Enter));

        assert_eq!(app.input_mode, InputMode::Normal);
        assert_eq!(app.todos[0].text, "Original text");
        assert_eq!(app.todos.len(), 1);
    }

    #[test]
    fn test_edit_task_cancel_with_esc() {
        let today = Local::now().date_naive();
        let todo = Todo {
            id: 15,
            text: "Unchanged".to_string(),
            created_date: today,
            done: false,
            done_at: None,
        };
        let mut app = App::new_for_test(vec![todo]);
        app.handle_key(key_press(KeyCode::Char('e')));
        app.input_buffer = "Typing something else".to_string();

        app.handle_key(key_press(KeyCode::Esc));
        assert_eq!(app.input_mode, InputMode::Normal);
        assert_eq!(app.todos[0].text, "Unchanged");
    }

    #[test]
    fn test_edit_task_history() {
        let yesterday = Local::now().date_naive().pred_opt().unwrap();
        let todo = Todo {
            id: 20,
            text: "Past task".to_string(),
            created_date: yesterday,
            done: false,
            done_at: None,
        };
        let mut app = App::new_for_test(vec![todo]);
        app.current_view = View::History;
        app.history_pane = HistoryPane::Todos;
        let dates = app.dates_list();
        let date_idx = dates.iter().position(|&d| d == yesterday).unwrap();
        app.history_date_index = date_idx;
        app.history_todo_index = 0;

        app.handle_key(key_press(KeyCode::Char('e')));
        assert_eq!(app.input_mode, InputMode::Editing(20));
        assert_eq!(app.input_buffer, "Past task");

        app.input_buffer = "Edited past task".to_string();
        app.handle_key(key_press(KeyCode::Enter));
        assert_eq!(app.input_mode, InputMode::Normal);
        assert_eq!(app.todos[0].text, "Edited past task");
        assert_eq!(app.todos[0].created_date, yesterday);
    }

    #[test]
    fn test_undo_redo_add() {
        let mut app = App::new_for_test(Vec::new());
        app.add_todo("New task");
        assert_eq!(app.todos.len(), 1);
        assert_eq!(app.todos[0].text, "New task");

        // Undo
        app.handle_key(key_press(KeyCode::Char('u')));
        assert_eq!(app.todos.len(), 0);
        assert_eq!(app.transient_message.as_deref(), Some("Undone"));

        // Redo via Ctrl+r
        app.handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL));
        assert_eq!(app.todos.len(), 1);
        assert_eq!(app.todos[0].text, "New task");
        assert_eq!(app.transient_message.as_deref(), Some("Redone"));
    }

    #[test]
    fn test_undo_redo_edit() {
        let today = Local::now().date_naive();
        let todo = Todo {
            id: 1,
            text: "Before edit".to_string(),
            created_date: today,
            done: false,
            done_at: None,
        };
        let mut app = App::new_for_test(vec![todo]);
        app.handle_key(key_press(KeyCode::Char('e')));
        app.input_buffer = "After edit".to_string();
        app.handle_key(key_press(KeyCode::Enter));
        assert_eq!(app.todos[0].text, "After edit");

        // Undo edit
        app.handle_key(key_press(KeyCode::Char('u')));
        assert_eq!(app.todos[0].text, "Before edit");

        // Redo edit
        app.handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL));
        assert_eq!(app.todos[0].text, "After edit");
    }

    #[test]
    fn test_new_mutation_clears_redo_stack() {
        let mut app = App::new_for_test(Vec::new());
        app.add_todo("Task 1");
        app.add_todo("Task 2");

        // Undo Task 2
        app.handle_key(key_press(KeyCode::Char('u')));
        assert_eq!(app.todos.len(), 1);
        assert_eq!(app.redo_stack.len(), 1);

        // Perform new mutation: Add Task 3
        app.add_todo("Task 3");
        assert_eq!(app.todos.len(), 2);
        assert!(app.redo_stack.is_empty());

        // Attempting to redo should report Nothing to redo
        app.handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL));
        assert_eq!(app.transient_message.as_deref(), Some("Nothing to redo"));
        assert_eq!(app.todos.len(), 2);
    }

    #[test]
    fn test_undo_redo_empty_stacks() {
        let mut app = App::new_for_test(Vec::new());
        app.handle_key(key_press(KeyCode::Char('u')));
        assert_eq!(app.transient_message.as_deref(), Some("Nothing to undo"));

        app.handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL));
        assert_eq!(app.transient_message.as_deref(), Some("Nothing to redo"));
    }

    #[test]
    fn test_carry_over_unfinished_tasks_moves_to_today() {
        let today = Local::now().date_naive();
        let yesterday = today.pred_opt().unwrap();

        let y1 = Todo {
            id: 1,
            text: "Unfinished 1".to_string(),
            created_date: yesterday,
            done: false,
            done_at: None,
        };
        let y2 = Todo {
            id: 2,
            text: "Finished".to_string(),
            created_date: yesterday,
            done: true,
            done_at: Some(Local::now()),
        };
        let y3 = Todo {
            id: 3,
            text: "Unfinished 2".to_string(),
            created_date: yesterday,
            done: false,
            done_at: None,
        };
        let t1 = Todo {
            id: 4,
            text: "Today task".to_string(),
            created_date: today,
            done: false,
            done_at: None,
        };

        let mut app = App::new_for_test(vec![y1, y2, y3, t1]);

        let uncarried = app.uncarried_previous_tasks();
        assert!(uncarried.is_some());
        let (p_date, ids) = uncarried.unwrap();
        assert_eq!(p_date, yesterday);
        assert_eq!(ids, vec![1, 3]);

        // Press 'c' to carry over
        app.handle_key(key_press(KeyCode::Char('c')));

        // Carried tasks should now be in Today, appended after t1
        let today_todos: Vec<&Todo> = app
            .todos
            .iter()
            .filter(|t| t.created_date == today)
            .collect();
        assert_eq!(today_todos.len(), 3);
        assert_eq!(today_todos[0].text, "Today task");
        assert_eq!(today_todos[1].text, "Unfinished 1");
        assert_eq!(today_todos[2].text, "Unfinished 2");

        // Yesterday should now only have the completed task
        let yesterday_todos: Vec<&Todo> = app
            .todos
            .iter()
            .filter(|t| t.created_date == yesterday)
            .collect();
        assert_eq!(yesterday_todos.len(), 1);
        assert_eq!(yesterday_todos[0].text, "Finished");

        // Should not be offered again
        assert!(app.uncarried_previous_tasks().is_none());

        // Undo carry over
        app.handle_key(key_press(KeyCode::Char('u')));
        let yesterday_restored: Vec<&Todo> = app
            .todos
            .iter()
            .filter(|t| t.created_date == yesterday)
            .collect();
        assert_eq!(yesterday_restored.len(), 3);
    }

    #[test]
    fn test_carry_over_none_when_previous_day_all_done() {
        let today = Local::now().date_naive();
        let yesterday = today.pred_opt().unwrap();
        let y_done = Todo {
            id: 1,
            text: "All done".to_string(),
            created_date: yesterday,
            done: true,
            done_at: Some(Local::now()),
        };
        let app = App::new_for_test(vec![y_done]);
        assert!(app.uncarried_previous_tasks().is_none());
    }

    #[test]
    fn test_reorder_today_down_and_up() {
        let today = Local::now().date_naive();
        let t1 = Todo {
            id: 1,
            text: "A".to_string(),
            created_date: today,
            done: false,
            done_at: None,
        };
        let t2 = Todo {
            id: 2,
            text: "B".to_string(),
            created_date: today,
            done: false,
            done_at: None,
        };
        let t3 = Todo {
            id: 3,
            text: "C".to_string(),
            created_date: today,
            done: false,
            done_at: None,
        };

        let mut app = App::new_for_test(vec![t1, t2, t3]);
        app.today_selected_index = 0; // selected on A

        // Move A down with 'J'
        app.handle_key(key_press(KeyCode::Char('J')));
        let texts: Vec<&str> = app.todos.iter().map(|t| t.text.as_str()).collect();
        assert_eq!(texts, vec!["B", "A", "C"]);
        assert_eq!(app.today_selected_index, 1); // Selection followed A

        // Move A up with 'K'
        app.handle_key(key_press(KeyCode::Char('K')));
        let texts: Vec<&str> = app.todos.iter().map(|t| t.text.as_str()).collect();
        assert_eq!(texts, vec!["A", "B", "C"]);
        assert_eq!(app.today_selected_index, 0);

        // Undo should revert
        app.handle_key(key_press(KeyCode::Char('u')));
        let texts: Vec<&str> = app.todos.iter().map(|t| t.text.as_str()).collect();
        assert_eq!(texts, vec!["B", "A", "C"]);
    }

    #[test]
    fn test_reorder_edges_and_single_item_no_op() {
        let today = Local::now().date_naive();
        let t1 = Todo {
            id: 1,
            text: "A".to_string(),
            created_date: today,
            done: false,
            done_at: None,
        };
        let t2 = Todo {
            id: 2,
            text: "B".to_string(),
            created_date: today,
            done: false,
            done_at: None,
        };
        let mut app = App::new_for_test(vec![t1, t2]);
        app.today_selected_index = 0;

        // Top edge: move up is a no-op
        app.handle_key(key_press(KeyCode::Char('K')));
        assert_eq!(app.today_selected_index, 0);
        assert_eq!(app.todos[0].text, "A");
        assert!(app.undo_stack.is_empty()); // No snapshot pushed for no-op

        // Bottom edge: move B down is a no-op
        app.today_selected_index = 1;
        app.handle_key(key_press(KeyCode::Char('J')));
        assert_eq!(app.today_selected_index, 1);
        assert_eq!(app.todos[1].text, "B");
        assert!(app.undo_stack.is_empty());

        // Single item
        let mut single_app = App::new_for_test(vec![app.todos[0].clone()]);
        single_app.handle_key(key_press(KeyCode::Char('J')));
        assert_eq!(single_app.today_selected_index, 0);
        single_app.handle_key(key_press(KeyCode::Char('K')));
        assert_eq!(single_app.today_selected_index, 0);
        assert!(single_app.undo_stack.is_empty());
    }

    #[test]
    fn test_reorder_history() {
        let past = Local::now().date_naive().pred_opt().unwrap();
        let t1 = Todo {
            id: 10,
            text: "Past 1".to_string(),
            created_date: past,
            done: false,
            done_at: None,
        };
        let t2 = Todo {
            id: 20,
            text: "Past 2".to_string(),
            created_date: past,
            done: false,
            done_at: None,
        };
        let mut app = App::new_for_test(vec![t1, t2]);
        app.current_view = View::History;
        app.history_pane = HistoryPane::Todos;
        let dates = app.dates_list();
        let date_idx = dates.iter().position(|&d| d == past).unwrap();
        app.history_date_index = date_idx;
        app.history_todo_index = 0;

        // Move Past 1 down using Shift+j
        app.handle_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::SHIFT));
        let past_todos: Vec<&str> = app
            .todos
            .iter()
            .filter(|t| t.created_date == past)
            .map(|t| t.text.as_str())
            .collect();
        assert_eq!(past_todos, vec!["Past 2", "Past 1"]);
        assert_eq!(app.history_todo_index, 1);
    }

    #[test]
    fn test_clear_completed_today_some() {
        let today = Local::now().date_naive();
        let t1 = Todo {
            id: 1,
            text: "Done 1".to_string(),
            created_date: today,
            done: true,
            done_at: Some(Local::now()),
        };
        let t2 = Todo {
            id: 2,
            text: "Unfinished".to_string(),
            created_date: today,
            done: false,
            done_at: None,
        };
        let t3 = Todo {
            id: 3,
            text: "Done 2".to_string(),
            created_date: today,
            done: true,
            done_at: Some(Local::now()),
        };
        let mut app = App::new_for_test(vec![t1, t2, t3]);

        // Press 'D' (Shift+d)
        app.handle_key(key_press(KeyCode::Char('D')));
        assert_eq!(app.todos.len(), 1);
        assert_eq!(app.todos[0].text, "Unfinished");
        assert_eq!(
            app.transient_message.as_deref(),
            Some("Cleared 2 completed · u to undo")
        );

        // Undo
        app.handle_key(key_press(KeyCode::Char('u')));
        assert_eq!(app.todos.len(), 3);
    }

    #[test]
    fn test_clear_completed_today_none() {
        let today = Local::now().date_naive();
        let t1 = Todo {
            id: 1,
            text: "Unfinished".to_string(),
            created_date: today,
            done: false,
            done_at: None,
        };
        let mut app = App::new_for_test(vec![t1]);

        app.handle_key(key_press(KeyCode::Char('D')));
        assert_eq!(app.todos.len(), 1);
        assert_eq!(app.transient_message.as_deref(), Some("No completed tasks"));
        assert!(app.undo_stack.is_empty());
    }

    #[test]
    fn test_clear_completed_history() {
        let past = Local::now().date_naive().pred_opt().unwrap();
        let today = Local::now().date_naive();

        let t_past_done = Todo {
            id: 1,
            text: "Past done".to_string(),
            created_date: past,
            done: true,
            done_at: Some(Local::now()),
        };
        let t_past_todo = Todo {
            id: 2,
            text: "Past open".to_string(),
            created_date: past,
            done: false,
            done_at: None,
        };
        let t_today_done = Todo {
            id: 3,
            text: "Today done".to_string(),
            created_date: today,
            done: true,
            done_at: Some(Local::now()),
        };

        let mut app = App::new_for_test(vec![t_past_done, t_past_todo, t_today_done]);
        app.current_view = View::History;
        app.history_pane = HistoryPane::Todos;
        let dates = app.dates_list();
        let date_idx = dates.iter().position(|&d| d == past).unwrap();
        app.history_date_index = date_idx;

        app.handle_key(key_press(KeyCode::Char('D')));
        // Only past done task was cleared
        assert_eq!(app.todos.len(), 2);
        assert_eq!(app.todos[0].text, "Past open");
        assert_eq!(app.todos[1].text, "Today done");
    }

    #[test]
    fn test_jump_keys_today() {
        let today = Local::now().date_naive();
        let todos: Vec<Todo> = (0..10)
            .map(|i| Todo {
                id: i + 1,
                text: format!("Task {i}"),
                created_date: today,
                done: false,
                done_at: None,
            })
            .collect();
        let mut app = App::new_for_test(todos);
        app.visible_list_height = 4; // half page step = 2
        app.today_selected_index = 0;

        // Ctrl+d half page down
        app.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL));
        assert_eq!(app.today_selected_index, 2);
        assert_eq!(app.todos.len(), 10); // Ctrl+d MUST NOT delete!

        // Another Ctrl+d
        app.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL));
        assert_eq!(app.today_selected_index, 4);

        // Jump to bottom 'G'
        app.handle_key(key_press(KeyCode::Char('G')));
        assert_eq!(app.today_selected_index, 9);

        // Ctrl+u half page up
        app.handle_key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
        assert_eq!(app.today_selected_index, 7);

        // Jump to top 'g'
        app.handle_key(key_press(KeyCode::Char('g')));
        assert_eq!(app.today_selected_index, 0);
    }

    #[test]
    fn test_jump_empty_list_clamping() {
        let mut app = App::new_for_test(Vec::new());
        app.visible_list_height = 4;

        // None of these should panic on empty list
        app.handle_key(key_press(KeyCode::Char('g')));
        assert_eq!(app.today_selected_index, 0);

        app.handle_key(key_press(KeyCode::Char('G')));
        assert_eq!(app.today_selected_index, 0);

        app.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL));
        assert_eq!(app.today_selected_index, 0);

        app.handle_key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
        assert_eq!(app.today_selected_index, 0);
    }

    #[test]
    fn test_jump_history() {
        let today = Local::now().date_naive();
        let yesterday = today.pred_opt().unwrap();
        let d2 = yesterday.pred_opt().unwrap();

        let t1 = Todo {
            id: 1,
            text: "Task 1".to_string(),
            created_date: d2,
            done: false,
            done_at: None,
        };
        let t2 = Todo {
            id: 2,
            text: "Task 2".to_string(),
            created_date: d2,
            done: false,
            done_at: None,
        };
        let t3 = Todo {
            id: 3,
            text: "Task 3".to_string(),
            created_date: d2,
            done: false,
            done_at: None,
        };
        let mut app = App::new_for_test(vec![t1, t2, t3]);
        app.current_view = View::History;
        app.history_pane = HistoryPane::Dates;
        app.visible_list_height = 2; // step = 1

        // Dates pane jump to bottom
        app.handle_key(key_press(KeyCode::Char('G')));
        assert_eq!(app.history_date_index, 1); // today and d2 -> 2 dates (index 1)

        // Focus todos pane
        app.history_pane = HistoryPane::Todos;
        assert_eq!(app.history_todo_index, 0);

        // Jump to bottom in todos pane
        app.handle_key(key_press(KeyCode::Char('G')));
        assert_eq!(app.history_todo_index, 2);

        // Jump to top in todos pane
        app.handle_key(key_press(KeyCode::Char('g')));
        assert_eq!(app.history_todo_index, 0);
    }

    #[test]
    fn test_help_overlay_keys() {
        let mut app = App::new_for_test(Vec::new());
        assert!(!app.show_help);

        // Open help with '?'
        let exit = app.handle_key(key_press(KeyCode::Char('?')));
        assert!(!exit);
        assert!(app.show_help);

        // Other keys like 'j' or 'a' should be ignored and not exit
        assert!(!app.handle_key(key_press(KeyCode::Char('j'))));
        assert!(app.show_help);
        assert_eq!(app.input_mode, InputMode::Normal);

        // 'q' should close overlay only and NOT quit the app!
        let exit_on_q = app.handle_key(key_press(KeyCode::Char('q')));
        assert!(!exit_on_q);
        assert!(!app.show_help);

        // Open again, close with Esc
        app.handle_key(key_press(KeyCode::Char('?')));
        assert!(app.show_help);
        assert!(!app.handle_key(key_press(KeyCode::Esc)));
        assert!(!app.show_help);

        // Open again, close with '?'
        app.handle_key(key_press(KeyCode::Char('?')));
        assert!(app.show_help);
        assert!(!app.handle_key(key_press(KeyCode::Char('?'))));
        assert!(!app.show_help);
    }

    #[test]
    fn test_move_task_to_today_from_history() {
        let today = Local::now().date_naive();
        let yesterday = today.pred_opt().unwrap();

        let mut app = App::new_for_test(vec![
            Todo {
                id: 1,
                text: "Today task".to_string(),
                created_date: today,
                done: false,
                done_at: None,
            },
            Todo {
                id: 2,
                text: "Yesterday completed task".to_string(),
                created_date: yesterday,
                done: true,
                done_at: Some(Local::now()),
            },
        ]);

        app.current_view = View::History;
        app.history_pane = HistoryPane::Todos;
        let dates = app.dates_list();
        app.history_date_index = dates.iter().position(|&d| d == yesterday).unwrap();
        app.history_todo_index = 0;

        // Move to today with 'm'
        app.handle_key(key_press(KeyCode::Char('m')));

        assert_eq!(app.transient_message, Some("Moved to Today".to_string()));

        // Check tasks on today
        let today_todos: Vec<&Todo> = app
            .todos
            .iter()
            .filter(|t| t.created_date == today)
            .collect();
        assert_eq!(today_todos.len(), 2);
        assert_eq!(today_todos[0].id, 1);
        assert_eq!(today_todos[1].id, 2);
        assert_eq!(today_todos[1].text, "Yesterday completed task");
        // Completion state reset to unfinished
        assert!(!today_todos[1].done);
        assert_eq!(today_todos[1].done_at, None);

        // Check yesterday has 0 tasks
        let yesterday_count = app
            .todos
            .iter()
            .filter(|t| t.created_date == yesterday)
            .count();
        assert_eq!(yesterday_count, 0);

        // Undo move
        app.undo();
        let yesterday_todos: Vec<&Todo> = app
            .todos
            .iter()
            .filter(|t| t.created_date == yesterday)
            .collect();
        assert_eq!(yesterday_todos.len(), 1);
        assert_eq!(yesterday_todos[0].id, 2);
        assert!(yesterday_todos[0].done);
        assert!(yesterday_todos[0].done_at.is_some());
    }

    #[test]
    fn test_move_task_already_in_today() {
        let today = Local::now().date_naive();
        let mut app = App::new_for_test(vec![Todo {
            id: 1,
            text: "Today task".to_string(),
            created_date: today,
            done: false,
            done_at: None,
        }]);

        // In Today view
        app.current_view = View::Today;
        app.handle_key(key_press(KeyCode::Char('m')));
        assert_eq!(app.transient_message, Some("Already in Today".to_string()));
        assert!(app.undo_stack.is_empty());

        // In History view with today selected
        app.current_view = View::History;
        app.history_pane = HistoryPane::Todos;
        let dates = app.dates_list();
        app.history_date_index = dates.iter().position(|&d| d == today).unwrap();
        app.handle_key(key_press(KeyCode::Char('m')));
        assert_eq!(app.transient_message, Some("Already in Today".to_string()));
        assert!(app.undo_stack.is_empty());
    }

    #[test]
    fn test_characterization_add_toggle_delete() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 27).unwrap();
        let mut app = App::new_for_test_with_date(Vec::new(), today);

        // Add
        app.add_todo("Write tests");
        assert_eq!(app.todos.len(), 1);
        assert_eq!(app.todos[0].text, "Write tests");
        assert_eq!(app.todos[0].created_date, today);
        assert!(!app.todos[0].done);
        assert_eq!(app.today_selected_index, 0);

        // Add second task
        app.add_todo("Run tests");
        assert_eq!(app.todos.len(), 2);
        assert_eq!(app.today_selected_index, 1);

        // Toggle first task
        app.today_selected_index = 0;
        app.toggle_today_todo();
        assert!(app.todos[0].done);
        assert!(app.todos[0].done_at.is_some());

        // Delete at the end of list clamps selection
        app.today_selected_index = 1;
        app.delete_today_todo();
        assert_eq!(app.todos.len(), 1);
        assert_eq!(app.today_selected_index, 0);

        // Delete last remaining task clamps to 0
        app.delete_today_todo();
        assert_eq!(app.todos.len(), 0);
        assert_eq!(app.today_selected_index, 0);
    }

    #[test]
    fn test_characterization_history_grouping_and_view_switching() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 27).unwrap();
        let yesterday = NaiveDate::from_ymd_opt(2026, 10, 26).unwrap();
        let older = NaiveDate::from_ymd_opt(2026, 10, 20).unwrap();

        let todos = vec![
            Todo {
                id: 1,
                text: "Older task".to_string(),
                created_date: older,
                done: true,
                done_at: None,
            },
            Todo {
                id: 2,
                text: "Yesterday task".to_string(),
                created_date: yesterday,
                done: false,
                done_at: None,
            },
            Todo {
                id: 3,
                text: "Today task".to_string(),
                created_date: today,
                done: false,
                done_at: None,
            },
        ];

        let mut app = App::new_for_test_with_date(todos, today);

        // History dates list: newest first, always includes today
        let dates = app.dates_list();
        assert_eq!(dates, vec![today, yesterday, older]);

        // View switching with Tab
        assert_eq!(app.current_view, View::Today);
        app.handle_key(key_press(KeyCode::Tab));
        assert_eq!(app.current_view, View::History);
        app.handle_key(key_press(KeyCode::Tab));
        assert_eq!(app.current_view, View::Today);

        // Empty list clamping
        let mut empty_app = App::new_for_test_with_date(Vec::new(), today);
        empty_app.clamp_selections();
        assert_eq!(empty_app.today_selected_index, 0);
        assert_eq!(empty_app.history_date_index, 0);
        assert_eq!(empty_app.history_todo_index, 0);
    }

    #[test]
    fn test_state_survives_shrink_and_grow_cycle() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 27).unwrap();
        let mut app = App::new_for_test_with_date(Vec::new(), today);

        // Start typing a new todo
        app.handle_key(key_press(KeyCode::Char('a')));
        assert_eq!(app.input_mode, InputMode::Adding);
        for c in "important note".chars() {
            app.handle_key(key_press(KeyCode::Char(c)));
        }
        assert_eq!(app.input_buffer, "important note");
        assert_eq!(app.cursor_position, 14);

        // Shrink terminal to 30x6 (below 40x8 minimum)
        app.on_resize(30, 6);
        assert!(app.is_too_small());

        // Keys while too small should be ignored without affecting state
        assert!(!app.handle_key(key_press(KeyCode::Char('x'))));
        assert!(!app.handle_key(key_press(KeyCode::Char('q')))); // In Adding mode, q is ignored
        assert!(!app.handle_key(key_press(KeyCode::Esc)));
        assert!(!app.handle_key(key_press(KeyCode::Enter)));

        // State is completely preserved
        assert_eq!(app.input_mode, InputMode::Adding);
        assert_eq!(app.input_buffer, "important note");
        assert_eq!(app.cursor_position, 14);

        // Grow terminal back to 80x24
        app.on_resize(80, 24);
        assert!(!app.is_too_small());

        // State is intact and editing can be finished
        assert_eq!(app.input_mode, InputMode::Adding);
        assert_eq!(app.input_buffer, "important note");
        assert_eq!(app.cursor_position, 14);

        // Press Enter to complete task
        assert!(!app.handle_key(key_press(KeyCode::Enter)));
        assert_eq!(app.input_mode, InputMode::Normal);
        assert_eq!(app.todos.len(), 1);
        assert_eq!(app.todos[0].text, "important note");

        // When in Normal mode and terminal is too small, q exits
        app.on_resize(30, 6);
        assert!(app.is_too_small());
        assert!(app.handle_key(key_press(KeyCode::Char('q')))); // exits

        // Ctrl+c exits in any mode
        app.input_mode = InputMode::Adding;
        let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert!(app.handle_key(ctrl_c)); // exits
    }

    #[test]
    fn test_rollover_while_idle_in_today() {
        let day1 = NaiveDate::from_ymd_opt(2026, 10, 27).unwrap();
        let day2 = NaiveDate::from_ymd_opt(2026, 10, 28).unwrap();

        let todos = vec![Todo {
            id: 1,
            text: "Task from day 1".to_string(),
            created_date: day1,
            done: false,
            done_at: None,
        }];

        let mut app = App::new_for_test_with_date(todos, day1);
        app.current_view = View::Today;

        // Rollover to next day
        assert!(app.tick(day2));
        assert_eq!(app.today, day2);
        assert_eq!(
            app.transient_message,
            Some("New day · Wednesday, 28 October".to_string())
        );

        // Today view starts empty
        let today_count = app.todos.iter().filter(|t| t.created_date == day2).count();
        assert_eq!(today_count, 0);

        // Yesterday's task is present in history
        let dates = app.dates_list();
        assert!(dates.contains(&day1));
        let yesterday_count = app.todos.iter().filter(|t| t.created_date == day1).count();
        assert_eq!(yesterday_count, 1);
    }

    #[test]
    fn test_rollover_while_in_history_selected_date_stays_the_same() {
        let day1 = NaiveDate::from_ymd_opt(2026, 10, 26).unwrap();
        let day2 = NaiveDate::from_ymd_opt(2026, 10, 27).unwrap();
        let day3 = NaiveDate::from_ymd_opt(2026, 10, 28).unwrap();

        let todos = vec![
            Todo {
                id: 1,
                text: "Older task".to_string(),
                created_date: day1,
                done: false,
                done_at: None,
            },
            Todo {
                id: 2,
                text: "Yesterday task".to_string(),
                created_date: day2,
                done: false,
                done_at: None,
            },
        ];

        let mut app = App::new_for_test_with_date(todos, day2);
        app.current_view = View::History;
        app.history_pane = HistoryPane::Dates;

        // Select day1 (which is at index 1 in [day2, day1])
        let dates = app.dates_list();
        assert_eq!(dates, vec![day2, day1]);
        app.history_date_index = 1;
        app.clamp_selections();
        assert_eq!(app.selected_history_date, Some(day1));

        // Midnight rollover to day3
        assert!(app.tick(day3));
        assert_eq!(app.today, day3);

        // Dates are now [day3, day2, day1]
        let new_dates = app.dates_list();
        assert_eq!(new_dates, vec![day3, day2, day1]);

        // The selected date is STILL day1, and history_date_index moved to 2
        assert_eq!(app.selected_history_date, Some(day1));
        assert_eq!(app.history_date_index, 2);
        assert_eq!(new_dates[app.history_date_index], day1);
    }

    #[test]
    fn test_rollover_while_prompt_is_open() {
        let day1 = NaiveDate::from_ymd_opt(2026, 10, 27).unwrap();
        let day2 = NaiveDate::from_ymd_opt(2026, 10, 28).unwrap();

        let mut app = App::new_for_test_with_date(Vec::new(), day1);

        // Open input prompt and type text
        app.handle_key(key_press(KeyCode::Char('a')));
        assert_eq!(app.input_mode, InputMode::Adding);
        for c in "Midnight task".chars() {
            app.handle_key(key_press(KeyCode::Char(c)));
        }
        assert_eq!(app.input_buffer, "Midnight task");

        // Midnight strikes
        assert!(app.tick(day2));
        assert_eq!(app.today, day2);

        // Input buffer and mode are preserved
        assert_eq!(app.input_mode, InputMode::Adding);
        assert_eq!(app.input_buffer, "Midnight task");

        // Press Enter to submit task
        assert!(!app.handle_key(key_press(KeyCode::Enter)));
        assert_eq!(app.input_mode, InputMode::Normal);
        assert_eq!(app.todos.len(), 1);

        // Task created_date is the new today (day2)
        assert_eq!(app.todos[0].text, "Midnight task");
        assert_eq!(app.todos[0].created_date, day2);
    }

    #[test]
    fn test_rollover_multi_day_jump_laptop_sleep() {
        let day1 = NaiveDate::from_ymd_opt(2026, 10, 27).unwrap();
        let day_after_sleep = NaiveDate::from_ymd_opt(2026, 11, 1).unwrap();

        let todos = vec![Todo {
            id: 1,
            text: "Pre-sleep task".to_string(),
            created_date: day1,
            done: false,
            done_at: None,
        }];

        let mut app = App::new_for_test_with_date(todos, day1);

        // Multi-day jump
        assert!(app.tick(day_after_sleep));
        assert_eq!(app.today, day_after_sleep);
        assert_eq!(
            app.transient_message,
            Some("New day · Sunday, 1 November".to_string())
        );

        // Today is empty
        assert_eq!(
            app.todos
                .iter()
                .filter(|t| t.created_date == day_after_sleep)
                .count(),
            0
        );

        // Old task is in history
        assert_eq!(
            app.todos.iter().filter(|t| t.created_date == day1).count(),
            1
        );
    }

    #[test]
    fn test_rollover_no_change_when_date_is_same() {
        let day = NaiveDate::from_ymd_opt(2026, 10, 27).unwrap();
        let mut app = App::new_for_test_with_date(Vec::new(), day);

        // Same date tick
        assert!(!app.tick(day));
        assert_eq!(app.today, day);
        assert_eq!(app.transient_message, None);
    }

    #[test]
    fn test_tab_key_no_op_when_show_history_is_false() {
        let day = NaiveDate::from_ymd_opt(2026, 10, 27).unwrap();
        let mut app = App::new_for_test_with_date(Vec::new(), day);
        app.config.ui.show_history = false;

        assert_eq!(app.current_view, View::Today);
        app.handle_key(key_press(KeyCode::Tab));
        assert_eq!(app.current_view, View::Today);

        app.handle_key(key_press(KeyCode::BackTab));
        assert_eq!(app.current_view, View::Today);
    }

    #[test]
    fn test_mouse_support_selection_and_toggle_today() {
        use crate::ui::render;
        use ratatui::backend::TestBackend;
        use ratatui::crossterm::event::{MouseButton, MouseEventKind};
        use ratatui::Terminal;

        let today = NaiveDate::from_ymd_opt(2026, 10, 27).unwrap();
        let todos = vec![
            Todo {
                id: 1,
                text: "Task 1".to_string(),
                created_date: today,
                done: false,
                done_at: None,
            },
            Todo {
                id: 2,
                text: "Task 2".to_string(),
                created_date: today,
                done: false,
                done_at: None,
            },
        ];

        let mut app = App::new_for_test_with_date(todos, today);
        app.config.ui.mouse = true;

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();

        assert_eq!(app.today_selected_index, 0);

        // Click task row 1 -> selects row 1
        let task_rect = app
            .hit_areas
            .iter()
            .find(|a| a.target == crate::ui::HitTarget::TodayTask { index: 1 })
            .expect("TodayTask { index: 1 } should be recorded")
            .rect;
        let ev = ratatui::crossterm::event::MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: task_rect.x + 10,
            row: task_rect.y,
            modifiers: KeyModifiers::NONE,
        };
        assert!(app.handle_mouse(ev));
        assert_eq!(app.today_selected_index, 1);

        // Click checkbox for task 1 -> toggles done state
        let cb_rect = app
            .hit_areas
            .iter()
            .find(|a| a.target == crate::ui::HitTarget::TodayCheckbox { index: 1 })
            .expect("TodayCheckbox { index: 1 } should be recorded")
            .rect;
        let ev_cb = ratatui::crossterm::event::MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: cb_rect.x,
            row: cb_rect.y,
            modifiers: KeyModifiers::NONE,
        };
        assert!(app.handle_mouse(ev_cb));
        assert_eq!(app.today_selected_index, 1);
        assert!(app.todos.iter().find(|t| t.id == 2).unwrap().done);

        // Click again -> toggles back to undone
        assert!(app.handle_mouse(ev_cb));
        assert!(!app.todos.iter().find(|t| t.id == 2).unwrap().done);
    }

    #[test]
    fn test_mouse_support_tabs_and_history_selection_and_toggle() {
        use crate::ui::render;
        use ratatui::backend::TestBackend;
        use ratatui::crossterm::event::{MouseButton, MouseEventKind};
        use ratatui::Terminal;

        let today = NaiveDate::from_ymd_opt(2026, 10, 27).unwrap();
        let past = NaiveDate::from_ymd_opt(2026, 10, 25).unwrap();
        let todos = vec![
            Todo {
                id: 1,
                text: "Today task".to_string(),
                created_date: today,
                done: false,
                done_at: None,
            },
            Todo {
                id: 2,
                text: "Past task".to_string(),
                created_date: past,
                done: false,
                done_at: None,
            },
        ];

        let mut app = App::new_for_test_with_date(todos, today);
        app.config.ui.mouse = true;

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();

        // 1. Click History tab -> switches to History view
        let tab_hist_rect = app
            .hit_areas
            .iter()
            .find(|a| a.target == crate::ui::HitTarget::TabHistory)
            .expect("TabHistory should be recorded")
            .rect;
        let ev = ratatui::crossterm::event::MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: tab_hist_rect.x,
            row: tab_hist_rect.y,
            modifiers: KeyModifiers::NONE,
        };
        assert!(app.handle_mouse(ev));
        assert_eq!(app.current_view, View::History);

        // Render History view so its hit areas are recorded
        terminal.draw(|f| render(&mut app, f)).unwrap();

        // 2. Click a date in the dates column
        let date_rect = app
            .hit_areas
            .iter()
            .find(|a| a.target == crate::ui::HitTarget::HistoryDate { index: 1 })
            .expect("HistoryDate index 1 should be recorded")
            .rect;
        let ev_date = ratatui::crossterm::event::MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: date_rect.x,
            row: date_rect.y,
            modifiers: KeyModifiers::NONE,
        };
        assert!(app.handle_mouse(ev_date));
        assert_eq!(app.history_pane, HistoryPane::Dates);
        assert_eq!(app.history_date_index, 1);

        // Render to update task pane for the selected past date
        terminal.draw(|f| render(&mut app, f)).unwrap();

        // 3. Click task in right pane -> focuses tasks and selects it
        let task_rect = app
            .hit_areas
            .iter()
            .find(|a| a.target == crate::ui::HitTarget::HistoryTask { index: 0 })
            .expect("HistoryTask index 0 should be recorded")
            .rect;
        let ev_task = ratatui::crossterm::event::MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: task_rect.x + 5,
            row: task_rect.y,
            modifiers: KeyModifiers::NONE,
        };
        assert!(app.handle_mouse(ev_task));
        assert_eq!(app.history_pane, HistoryPane::Todos);
        assert_eq!(app.history_todo_index, 0);

        // 4. Click checkbox in history pane -> toggles task done
        let cb_rect = app
            .hit_areas
            .iter()
            .find(|a| a.target == crate::ui::HitTarget::HistoryCheckbox { index: 0 })
            .expect("HistoryCheckbox index 0 should be recorded")
            .rect;
        let ev_cb = ratatui::crossterm::event::MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: cb_rect.x,
            row: cb_rect.y,
            modifiers: KeyModifiers::NONE,
        };
        assert!(app.handle_mouse(ev_cb));
        assert!(app.todos.iter().find(|t| t.id == 2).unwrap().done);

        // 5. Click Today tab -> switches back to Today view
        let tab_today_rect = app
            .hit_areas
            .iter()
            .find(|a| a.target == crate::ui::HitTarget::TabToday)
            .expect("TabToday should be recorded")
            .rect;
        let ev_today = ratatui::crossterm::event::MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: tab_today_rect.x,
            row: tab_today_rect.y,
            modifiers: KeyModifiers::NONE,
        };
        assert!(app.handle_mouse(ev_today));
        assert_eq!(app.current_view, View::Today);
    }

    #[test]
    fn test_mouse_support_scroll() {
        use crate::ui::render;
        use ratatui::backend::TestBackend;
        use ratatui::crossterm::event::MouseEventKind;
        use ratatui::Terminal;

        let today = NaiveDate::from_ymd_opt(2026, 10, 27).unwrap();
        let todos = vec![
            Todo {
                id: 1,
                text: "Task 1".to_string(),
                created_date: today,
                done: false,
                done_at: None,
            },
            Todo {
                id: 2,
                text: "Task 2".to_string(),
                created_date: today,
                done: false,
                done_at: None,
            },
            Todo {
                id: 3,
                text: "Task 3".to_string(),
                created_date: today,
                done: false,
                done_at: None,
            },
        ];

        let mut app = App::new_for_test_with_date(todos, today);
        app.config.ui.mouse = true;

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();

        assert_eq!(app.today_selected_index, 0);

        // Scroll down
        let scroll_down = ratatui::crossterm::event::MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 10,
            row: 5,
            modifiers: KeyModifiers::NONE,
        };
        assert!(app.handle_mouse(scroll_down));
        assert_eq!(app.today_selected_index, 1);

        assert!(app.handle_mouse(scroll_down));
        assert_eq!(app.today_selected_index, 2);

        // At end of list, scroll down returns false and does not overflow
        assert!(!app.handle_mouse(scroll_down));
        assert_eq!(app.today_selected_index, 2);

        // Scroll up
        let scroll_up = ratatui::crossterm::event::MouseEvent {
            kind: MouseEventKind::ScrollUp,
            column: 10,
            row: 5,
            modifiers: KeyModifiers::NONE,
        };
        assert!(app.handle_mouse(scroll_up));
        assert_eq!(app.today_selected_index, 1);

        assert!(app.handle_mouse(scroll_up));
        assert_eq!(app.today_selected_index, 0);

        // At start of list, scroll up returns false and does not underflow
        assert!(!app.handle_mouse(scroll_up));
        assert_eq!(app.today_selected_index, 0);
    }

    #[test]
    fn test_mouse_support_edge_cases_and_ignores() {
        use crate::ui::render;
        use ratatui::backend::TestBackend;
        use ratatui::crossterm::event::{MouseButton, MouseEventKind};
        use ratatui::Terminal;

        let today = NaiveDate::from_ymd_opt(2026, 10, 27).unwrap();
        let mut app = App::new_for_test_with_date(Vec::new(), today);
        app.config.ui.mouse = true;

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();

        let click = ratatui::crossterm::event::MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 20,
            row: 10,
            modifiers: KeyModifiers::NONE,
        };

        // 1. Empty list: clicking list area is no-op
        assert!(!app.handle_mouse(click));

        // 2. Out-of-bounds click
        let oob_click = ratatui::crossterm::event::MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 79,
            row: 23,
            modifiers: KeyModifiers::NONE,
        };
        assert!(!app.handle_mouse(oob_click));

        // Add a todo and render
        app.todos.push(Todo {
            id: 1,
            text: "Task".to_string(),
            created_date: today,
            done: false,
            done_at: None,
        });
        terminal.draw(|f| render(&mut app, f)).unwrap();

        let cb_rect = app
            .hit_areas
            .iter()
            .find(|a| a.target == crate::ui::HitTarget::TodayCheckbox { index: 0 })
            .expect("Checkbox hit area exists")
            .rect;
        let valid_click = ratatui::crossterm::event::MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: cb_rect.x,
            row: cb_rect.y,
            modifiers: KeyModifiers::NONE,
        };

        // 3. Ignored while prompting (Adding)
        app.input_mode = InputMode::Adding;
        assert!(!app.handle_mouse(valid_click));

        // 4. Ignored while prompting (Editing)
        app.input_mode = InputMode::Editing(1);
        assert!(!app.handle_mouse(valid_click));
        app.input_mode = InputMode::Normal;

        // 5. Ignored while help overlay is shown
        app.show_help = true;
        assert!(!app.handle_mouse(valid_click));
        app.show_help = false;

        // 6. Ignored when config.ui.mouse is false
        app.config.ui.mouse = false;
        assert!(!app.handle_mouse(valid_click));
        app.config.ui.mouse = true;

        // 7. Ignored when right click or drag
        let right_click = ratatui::crossterm::event::MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Right),
            column: cb_rect.x,
            row: cb_rect.y,
            modifiers: KeyModifiers::NONE,
        };
        assert!(!app.handle_mouse(right_click));

        let drag = ratatui::crossterm::event::MouseEvent {
            kind: MouseEventKind::Drag(MouseButton::Left),
            column: cb_rect.x,
            row: cb_rect.y,
            modifiers: KeyModifiers::NONE,
        };
        assert!(!app.handle_mouse(drag));
    }
}
