use std::collections::BTreeSet;

use chrono::{Datelike, Local, NaiveDate};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::model::{HistoryPane, InputMode, Todo, View};
use crate::storage::load_todos;

use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub todos: Vec<Todo>,
    pub recurring: Vec<crate::model::Recurring>,
    pub focus: Option<u64>,
}

pub struct App {
    pub data_path: PathBuf,
    pub today: NaiveDate,
    pub todos: Vec<Todo>,
    pub next_id: u64,
    pub recurring: Vec<crate::model::Recurring>,
    pub focus: Option<u64>,
    pub original_version: u32,
    pub current_view: View,
    pub history_pane: HistoryPane,
    pub input_mode: InputMode,
    pub input_buffer: String,
    pub cursor_position: usize,
    pub today_selected_index: usize,
    pub today_selected_id: Option<u64>,
    pub history_date_index: usize,
    pub history_todo_index: usize,
    pub history_todo_id: Option<u64>,
    pub selected_history_date: Option<NaiveDate>,
    pub upcoming_date_index: usize,
    pub upcoming_todo_index: usize,
    pub upcoming_todo_id: Option<u64>,
    pub selected_upcoming_date: Option<NaiveDate>,
    pub upcoming_pane: HistoryPane,
    pub prompt_error: Option<String>,
    pub sort_mode: crate::model::SortMode,
    pub today_filter: crate::model::Filter,
    pub upcoming_filter: crate::model::Filter,
    pub history_filter: crate::model::Filter,
    pub marks: std::collections::HashSet<u64>,
    pub show_detail_pane: bool,
    pub note_buffer: String,
    pub note_cursor: usize,
    pub status_message: Option<String>,
    pub undo_stack: Vec<Snapshot>,
    pub redo_stack: Vec<Snapshot>,
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
    pub config: crate::config::Config,
    pub theme: crate::theme::Theme,
    pub symbols: crate::ui::Symbols,
    pub keymap: crate::keymap::Keymap,
    pub hit_areas: Vec<crate::ui::HitArea>,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn load_or_exit() -> Self {
        let (config, warnings) = crate::config::Config::load();
        Self::load_or_exit_with_config(config, warnings)
    }

    pub fn load_or_exit_with_config(config: crate::config::Config, warnings: Vec<String>) -> Self {
        let data_path =
            crate::storage::get_storage_path().unwrap_or_else(|_| PathBuf::from("todos.json"));
        Self::load_or_exit_with_path_and_config(data_path, config, warnings)
    }

    pub fn load_or_exit_with_path_and_config(
        data_path: PathBuf,
        config: crate::config::Config,
        warnings: Vec<String>,
    ) -> Self {
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
            next_id: load_res.next_id,
            recurring: load_res.recurring,
            focus: load_res.focus,
            original_version: load_res.original_version,
            current_view: View::Today,
            history_pane: HistoryPane::Dates,
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            cursor_position: 0,
            today_selected_index: 0,
            today_selected_id: None,
            history_date_index: 0,
            history_todo_index: 0,
            history_todo_id: None,
            selected_history_date: None,
            upcoming_date_index: 0,
            upcoming_todo_index: 0,
            upcoming_todo_id: None,
            selected_upcoming_date: None,
            upcoming_pane: HistoryPane::Dates,
            prompt_error: None,
            sort_mode: config.tasks.sort,
            today_filter: crate::model::Filter::default(),
            upcoming_filter: crate::model::Filter::default(),
            history_filter: crate::model::Filter::default(),
            marks: std::collections::HashSet::new(),
            show_detail_pane: config.ui.detail_pane,
            note_buffer: String::new(),
            note_cursor: 0,
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
            config,
            theme,
            symbols,
            keymap,
            hit_areas: Vec::new(),
        };

        if !all_warnings.is_empty() {
            app.set_transient_message(all_warnings.join(" · "));
        }

        let added = crate::store::materialize_recurring(
            &mut app.todos,
            &mut app.recurring,
            &mut app.next_id,
            app.today,
        );
        if added > 0 && app.auto_save {
            app.save();
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
        let next_id = todos.iter().map(|t| t.id).max().unwrap_or(0) + 1;
        let mut app = Self {
            data_path: PathBuf::from("dummy_test.json"),
            today,
            todos,
            next_id,
            recurring: Vec::new(),
            focus: None,
            original_version: crate::storage::migrate::CURRENT_SCHEMA_VERSION,
            current_view: View::Today,
            history_pane: HistoryPane::Dates,
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            cursor_position: 0,
            today_selected_index: 0,
            today_selected_id: None,
            history_date_index: 0,
            history_todo_index: 0,
            history_todo_id: None,
            selected_history_date: None,
            upcoming_date_index: 0,
            upcoming_todo_index: 0,
            upcoming_todo_id: None,
            selected_upcoming_date: None,
            upcoming_pane: HistoryPane::Dates,
            prompt_error: None,
            sort_mode: crate::model::SortMode::Manual,
            today_filter: crate::model::Filter::default(),
            upcoming_filter: crate::model::Filter::default(),
            history_filter: crate::model::Filter::default(),
            marks: std::collections::HashSet::new(),
            show_detail_pane: false,
            note_buffer: String::new(),
            note_cursor: 0,
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
        let data = crate::storage::StorageData {
            version: crate::storage::migrate::CURRENT_SCHEMA_VERSION,
            next_id: self.next_id,
            todos: self.todos.clone(),
            recurring: self.recurring.clone(),
            focus: self.focus,
        };
        let res = crate::storage::save_storage_data(
            &self.data_path,
            &data,
            self.original_version,
            self.fingerprint,
            self.is_first_save,
            self.last_backup_time,
        );
        match res {
            Ok(save_res) => {
                self.original_version = crate::storage::migrate::CURRENT_SCHEMA_VERSION;
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

    pub fn current_filter(&self) -> &crate::model::Filter {
        match self.current_view {
            View::Today => &self.today_filter,
            View::Upcoming => &self.upcoming_filter,
            View::History => &self.history_filter,
        }
    }

    pub fn current_filter_mut(&mut self) -> &mut crate::model::Filter {
        match self.current_view {
            View::Today => &mut self.today_filter,
            View::Upcoming => &mut self.upcoming_filter,
            View::History => &mut self.history_filter,
        }
    }

    pub fn clear_current_filter(&mut self) {
        *self.current_filter_mut() = crate::model::Filter::default();
    }

    pub fn visible_rows_today(&self) -> Vec<crate::model::Row> {
        crate::store::visible_rows(&self.todos, self.today, &self.today_filter, self.sort_mode)
    }

    pub fn visible_rows_history(&self) -> Vec<crate::model::Row> {
        let dates = self.dates_list();
        if let Some(&date) = dates.get(self.history_date_index) {
            crate::store::visible_rows(&self.todos, date, &self.history_filter, self.sort_mode)
        } else {
            Vec::new()
        }
    }

    pub fn upcoming_dates_list(&self) -> Vec<NaiveDate> {
        let today = self.today;
        let mut dates: Vec<NaiveDate> = self
            .todos
            .iter()
            .filter(|t| t.created_date > today)
            .map(|t| t.created_date)
            .collect();
        dates.sort_unstable();
        dates.dedup();
        dates
    }

    pub fn visible_rows_upcoming(&self) -> Vec<crate::model::Row> {
        let dates = self.upcoming_dates_list();
        if let Some(&date) = dates.get(self.upcoming_date_index) {
            crate::store::visible_rows(&self.todos, date, &self.upcoming_filter, self.sort_mode)
        } else {
            Vec::new()
        }
    }

    pub fn re_resolve_selections(&mut self) {
        let rows_today = self.visible_rows_today();
        if rows_today.is_empty() {
            self.today_selected_index = 0;
            self.today_selected_id = None;
        } else if let Some(id) = self.today_selected_id {
            if let Some(pos) = rows_today.iter().position(|r| r.id == id) {
                self.today_selected_index = pos;
            } else {
                if self.today_selected_index >= rows_today.len() {
                    self.today_selected_index = rows_today.len() - 1;
                }
                self.today_selected_id = rows_today.get(self.today_selected_index).map(|r| r.id);
            }
        } else {
            if self.today_selected_index >= rows_today.len() {
                self.today_selected_index = rows_today.len() - 1;
            }
            self.today_selected_id = rows_today.get(self.today_selected_index).map(|r| r.id);
        }

        let dates = self.dates_list();
        if dates.is_empty() {
            self.history_date_index = 0;
            self.selected_history_date = None;
            self.history_todo_index = 0;
            self.history_todo_id = None;
        } else {
            if self.history_date_index >= dates.len() {
                self.history_date_index = dates.len() - 1;
            }
            self.selected_history_date = dates.get(self.history_date_index).copied();

            let rows_hist = self.visible_rows_history();
            if rows_hist.is_empty() {
                self.history_todo_index = 0;
                self.history_todo_id = None;
            } else if let Some(id) = self.history_todo_id {
                if let Some(pos) = rows_hist.iter().position(|r| r.id == id) {
                    self.history_todo_index = pos;
                } else {
                    if self.history_todo_index >= rows_hist.len() {
                        self.history_todo_index = rows_hist.len() - 1;
                    }
                    self.history_todo_id = rows_hist.get(self.history_todo_index).map(|r| r.id);
                }
            } else {
                if self.history_todo_index >= rows_hist.len() {
                    self.history_todo_index = rows_hist.len() - 1;
                }
                self.history_todo_id = rows_hist.get(self.history_todo_index).map(|r| r.id);
            }
        }

        let up_dates = self.upcoming_dates_list();
        if up_dates.is_empty() {
            self.upcoming_date_index = 0;
            self.selected_upcoming_date = None;
            self.upcoming_todo_index = 0;
            self.upcoming_todo_id = None;
        } else {
            if self.upcoming_date_index >= up_dates.len() {
                self.upcoming_date_index = up_dates.len() - 1;
            }
            self.selected_upcoming_date = up_dates.get(self.upcoming_date_index).copied();

            let rows_up = self.visible_rows_upcoming();
            if rows_up.is_empty() {
                self.upcoming_todo_index = 0;
                self.upcoming_todo_id = None;
            } else if let Some(id) = self.upcoming_todo_id {
                if let Some(pos) = rows_up.iter().position(|r| r.id == id) {
                    self.upcoming_todo_index = pos;
                } else {
                    if self.upcoming_todo_index >= rows_up.len() {
                        self.upcoming_todo_index = rows_up.len() - 1;
                    }
                    self.upcoming_todo_id = rows_up.get(self.upcoming_todo_index).map(|r| r.id);
                }
            } else {
                if self.upcoming_todo_index >= rows_up.len() {
                    self.upcoming_todo_index = rows_up.len() - 1;
                }
                self.upcoming_todo_id = rows_up.get(self.upcoming_todo_index).map(|r| r.id);
            }
        }
    }

    pub fn clamp_selections(&mut self) {
        let rows_today = self.visible_rows_today();
        if rows_today.is_empty() {
            self.today_selected_index = 0;
            self.today_selected_id = None;
        } else {
            if self.today_selected_index >= rows_today.len() {
                self.today_selected_index = rows_today.len() - 1;
            }
            self.today_selected_id = rows_today.get(self.today_selected_index).map(|r| r.id);
        }

        let dates = self.dates_list();
        if dates.is_empty() {
            self.history_date_index = 0;
            self.selected_history_date = None;
            self.history_todo_index = 0;
            self.history_todo_id = None;
        } else {
            if self.history_date_index >= dates.len() {
                self.history_date_index = dates.len() - 1;
            }
            self.selected_history_date = dates.get(self.history_date_index).copied();

            let rows_hist = self.visible_rows_history();
            if rows_hist.is_empty() {
                self.history_todo_index = 0;
                self.history_todo_id = None;
            } else {
                if self.history_todo_index >= rows_hist.len() {
                    self.history_todo_index = rows_hist.len() - 1;
                }
                self.history_todo_id = rows_hist.get(self.history_todo_index).map(|r| r.id);
            }
        }

        let up_dates = self.upcoming_dates_list();
        if up_dates.is_empty() {
            self.upcoming_date_index = 0;
            self.selected_upcoming_date = None;
            self.upcoming_todo_index = 0;
            self.upcoming_todo_id = None;
        } else {
            if self.upcoming_date_index >= up_dates.len() {
                self.upcoming_date_index = up_dates.len() - 1;
            }
            self.selected_upcoming_date = up_dates.get(self.upcoming_date_index).copied();

            let rows_up = self.visible_rows_upcoming();
            if rows_up.is_empty() {
                self.upcoming_todo_index = 0;
                self.upcoming_todo_id = None;
            } else {
                if self.upcoming_todo_index >= rows_up.len() {
                    self.upcoming_todo_index = rows_up.len() - 1;
                }
                self.upcoming_todo_id = rows_up.get(self.upcoming_todo_index).map(|r| r.id);
            }
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
            self.marks.clear();
            let added = crate::store::materialize_recurring(
                &mut self.todos,
                &mut self.recurring,
                &mut self.next_id,
                self.today,
            );
            if added > 0 && self.auto_save {
                self.save();
            }
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

    pub fn snapshot(&mut self) {
        self.undo_stack.push(Snapshot {
            todos: self.todos.clone(),
            recurring: self.recurring.clone(),
            focus: self.focus,
        });
        if self.undo_stack.len() > 50 {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    pub fn mutate<F>(&mut self, f: F)
    where
        F: FnOnce(&mut Vec<Todo>),
    {
        self.snapshot();
        f(&mut self.todos);
        if let Some(fid) = self.focus {
            if !self.todos.iter().any(|t| t.id == fid && !t.done) {
                self.focus = None;
            }
        }
        self.save();
        self.re_resolve_selections();
    }

    pub fn undo(&mut self) {
        if let Some(prev) = self.undo_stack.pop() {
            self.redo_stack.push(Snapshot {
                todos: self.todos.clone(),
                recurring: self.recurring.clone(),
                focus: self.focus,
            });
            if self.redo_stack.len() > 50 {
                self.redo_stack.remove(0);
            }
            self.todos = prev.todos;
            self.recurring = prev.recurring;
            self.focus = prev.focus;
            self.save();
            self.re_resolve_selections();
            self.set_transient_message("Undone");
        } else {
            self.set_transient_message("Nothing to undo");
        }
    }

    pub fn redo(&mut self) {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(Snapshot {
                todos: self.todos.clone(),
                recurring: self.recurring.clone(),
                focus: self.focus,
            });
            if self.undo_stack.len() > 50 {
                self.undo_stack.remove(0);
            }
            self.todos = next.todos;
            self.recurring = next.recurring;
            self.focus = next.focus;
            self.save();
            self.re_resolve_selections();
            self.set_transient_message("Redone");
        } else {
            self.set_transient_message("Nothing to redo");
        }
    }

    pub fn start_editing_today(&mut self) {
        let rows = self.visible_rows_today();
        if let Some(row) = rows.get(self.today_selected_index) {
            if let Some(todo) = self.todos.iter().find(|t| t.id == row.id) {
                self.input_mode = InputMode::Editing(todo.id);
                self.input_buffer = crate::parser::re_serialize_tokens(todo, self.today);
                self.cursor_position = self.input_buffer.chars().count();
                self.prompt_error = None;
            }
        }
    }

    pub fn start_editing_history(&mut self) {
        let rows = self.visible_rows_history();
        if let Some(row) = rows.get(self.history_todo_index) {
            if let Some(todo) = self.todos.iter().find(|t| t.id == row.id) {
                self.input_mode = InputMode::Editing(todo.id);
                self.input_buffer = crate::parser::re_serialize_tokens(todo, self.today);
                self.cursor_position = self.input_buffer.chars().count();
                self.prompt_error = None;
            }
        }
    }

    pub fn start_editing_upcoming(&mut self) {
        let rows = self.visible_rows_upcoming();
        if let Some(row) = rows.get(self.upcoming_todo_index) {
            if let Some(todo) = self.todos.iter().find(|t| t.id == row.id) {
                self.input_mode = InputMode::Editing(todo.id);
                self.input_buffer = crate::parser::re_serialize_tokens(todo, self.today);
                self.cursor_position = self.input_buffer.chars().count();
                self.prompt_error = None;
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
        let unfinished_parents: Vec<u64> = self
            .todos
            .iter()
            .filter(|t| t.created_date == prev_date && t.parent.is_none() && !t.done)
            .map(|t| t.id)
            .collect();
        if unfinished_parents.is_empty() {
            None
        } else {
            let mut ids_to_carry = Vec::new();
            for &pid in &unfinished_parents {
                ids_to_carry.push(pid);
                for t in self.todos.iter() {
                    if t.created_date == prev_date && t.parent == Some(pid) {
                        ids_to_carry.push(t.id);
                    }
                }
            }
            Some((prev_date, ids_to_carry))
        }
    }

    pub fn carry_over(&mut self) {
        if let Some((_, ids)) = self.uncarried_previous_tasks() {
            let today = self.today;
            let parent_count = self
                .todos
                .iter()
                .filter(|t| ids.contains(&t.id) && t.parent.is_none())
                .count();
            self.mutate(|todos| {
                crate::store::carry_over_tasks(todos, &ids, today);
            });
            self.set_transient_message(format!(
                "Carried over {parent_count} unfinished task{} · u to undo",
                if parent_count == 1 { "" } else { "s" }
            ));
        }
    }

    pub fn reorder_today(&mut self, down: bool) {
        if self.sort_mode != crate::model::SortMode::Manual {
            self.set_transient_message("Reorder works in manual sort");
            return;
        }
        let today = self.today;
        let selected_id = match self
            .visible_rows_today()
            .get(self.today_selected_index)
            .map(|r| r.id)
        {
            Some(id) => id,
            None => return,
        };
        if !crate::store::can_reorder_day_task(&self.todos, today, selected_id, down) {
            return;
        }
        self.today_selected_id = Some(selected_id);
        self.mutate(|todos| {
            crate::store::reorder_day_task(todos, today, selected_id, down);
        });
    }

    pub fn reorder_history(&mut self, down: bool) {
        if self.sort_mode != crate::model::SortMode::Manual {
            self.set_transient_message("Reorder works in manual sort");
            return;
        }
        let dates = self.dates_list();
        if let Some(&selected_date) = dates.get(self.history_date_index) {
            let selected_id = match self
                .visible_rows_history()
                .get(self.history_todo_index)
                .map(|r| r.id)
            {
                Some(id) => id,
                None => return,
            };
            if !crate::store::can_reorder_day_task(&self.todos, selected_date, selected_id, down) {
                return;
            }
            self.history_todo_id = Some(selected_id);
            self.mutate(|todos| {
                crate::store::reorder_day_task(todos, selected_date, selected_id, down);
            });
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
            crate::store::clear_completed(todos, today);
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
                crate::store::clear_completed(todos, selected_date);
            });
            self.set_transient_message(format!("Cleared {completed_count} completed · u to undo"));
        }
    }

    pub fn clear_completed_upcoming(&mut self) {
        let dates = self.upcoming_dates_list();
        if let Some(&selected_date) = dates.get(self.upcoming_date_index) {
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
                crate::store::clear_completed(todos, selected_date);
            });
            self.set_transient_message(format!("Cleared {completed_count} completed · u to undo"));
        }
    }

    pub fn jump_top(&mut self) {
        match self.current_view {
            View::Today => {
                self.today_selected_index = 0;
                self.today_selected_id = self.visible_rows_today().first().map(|r| r.id);
            }
            View::Upcoming => match self.upcoming_pane {
                HistoryPane::Dates => {
                    self.upcoming_date_index = 0;
                    self.upcoming_todo_index = 0;
                    self.upcoming_todo_id = self.visible_rows_upcoming().first().map(|r| r.id);
                    self.clamp_selections();
                }
                HistoryPane::Todos => {
                    self.upcoming_todo_index = 0;
                    self.upcoming_todo_id = self.visible_rows_upcoming().first().map(|r| r.id);
                }
            },
            View::History => match self.history_pane {
                HistoryPane::Dates => {
                    self.history_date_index = 0;
                    self.history_todo_index = 0;
                    self.history_todo_id = self.visible_rows_history().first().map(|r| r.id);
                    self.clamp_selections();
                }
                HistoryPane::Todos => {
                    self.history_todo_index = 0;
                    self.history_todo_id = self.visible_rows_history().first().map(|r| r.id);
                }
            },
        }
    }

    pub fn jump_bottom(&mut self) {
        match self.current_view {
            View::Today => {
                let rows = self.visible_rows_today();
                self.today_selected_index = rows.len().saturating_sub(1);
                self.today_selected_id = rows.get(self.today_selected_index).map(|r| r.id);
            }
            View::Upcoming => match self.upcoming_pane {
                HistoryPane::Dates => {
                    let dates = self.upcoming_dates_list();
                    self.upcoming_date_index = dates.len().saturating_sub(1);
                    self.upcoming_todo_index = 0;
                    self.clamp_selections();
                }
                HistoryPane::Todos => {
                    let rows = self.visible_rows_upcoming();
                    self.upcoming_todo_index = rows.len().saturating_sub(1);
                    self.upcoming_todo_id = rows.get(self.upcoming_todo_index).map(|r| r.id);
                }
            },
            View::History => match self.history_pane {
                HistoryPane::Dates => {
                    let dates = self.dates_list();
                    self.history_date_index = dates.len().saturating_sub(1);
                    self.history_todo_index = 0;
                    self.clamp_selections();
                }
                HistoryPane::Todos => {
                    let rows = self.visible_rows_history();
                    self.history_todo_index = rows.len().saturating_sub(1);
                    self.history_todo_id = rows.get(self.history_todo_index).map(|r| r.id);
                }
            },
        }
    }

    pub fn jump_half_page_down(&mut self) {
        let step = (self.visible_list_height / 2).max(1);
        match self.current_view {
            View::Today => {
                let rows = self.visible_rows_today();
                let count = rows.len();
                if count > 0 {
                    self.today_selected_index = (self.today_selected_index + step).min(count - 1);
                } else {
                    self.today_selected_index = 0;
                }
                self.today_selected_id = rows.get(self.today_selected_index).map(|r| r.id);
            }
            View::Upcoming => match self.upcoming_pane {
                HistoryPane::Dates => {
                    let dates = self.upcoming_dates_list();
                    let count = dates.len();
                    if count > 0 {
                        self.upcoming_date_index = (self.upcoming_date_index + step).min(count - 1);
                    } else {
                        self.upcoming_date_index = 0;
                    }
                    self.upcoming_todo_index = 0;
                    self.clamp_selections();
                }
                HistoryPane::Todos => {
                    let rows = self.visible_rows_upcoming();
                    let count = rows.len();
                    if count > 0 {
                        self.upcoming_todo_index = (self.upcoming_todo_index + step).min(count - 1);
                    } else {
                        self.upcoming_todo_index = 0;
                    }
                    self.upcoming_todo_id = rows.get(self.upcoming_todo_index).map(|r| r.id);
                }
            },
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
                    let rows = self.visible_rows_history();
                    let count = rows.len();
                    if count > 0 {
                        self.history_todo_index = (self.history_todo_index + step).min(count - 1);
                    } else {
                        self.history_todo_index = 0;
                    }
                    self.history_todo_id = rows.get(self.history_todo_index).map(|r| r.id);
                }
            },
        }
    }

    pub fn jump_half_page_up(&mut self) {
        let step = (self.visible_list_height / 2).max(1);
        match self.current_view {
            View::Today => {
                self.today_selected_index = self.today_selected_index.saturating_sub(step);
                self.today_selected_id = self
                    .visible_rows_today()
                    .get(self.today_selected_index)
                    .map(|r| r.id);
            }
            View::Upcoming => match self.upcoming_pane {
                HistoryPane::Dates => {
                    self.upcoming_date_index = self.upcoming_date_index.saturating_sub(step);
                    self.upcoming_todo_index = 0;
                    self.clamp_selections();
                }
                HistoryPane::Todos => {
                    self.upcoming_todo_index = self.upcoming_todo_index.saturating_sub(step);
                    self.upcoming_todo_id = self
                        .visible_rows_upcoming()
                        .get(self.upcoming_todo_index)
                        .map(|r| r.id);
                }
            },
            View::History => match self.history_pane {
                HistoryPane::Dates => {
                    self.history_date_index = self.history_date_index.saturating_sub(step);
                    self.history_todo_index = 0;
                    self.clamp_selections();
                }
                HistoryPane::Todos => {
                    self.history_todo_index = self.history_todo_index.saturating_sub(step);
                    self.history_todo_id = self
                        .visible_rows_history()
                        .get(self.history_todo_index)
                        .map(|r| r.id);
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
        let id = self.next_id;
        self.next_id += 1;
        self.mutate(|todos| {
            crate::store::add_task_with_id(todos, id, clean_text, today);
        });

        self.today_selected_id = Some(id);
        self.re_resolve_selections();
    }

    pub fn toggle_today_todo(&mut self) {
        let rows = self.visible_rows_today();
        if let Some(row) = rows.get(self.today_selected_index) {
            let target_id = row.id;
            let now = Local::now();
            self.mutate(|todos| {
                crate::store::toggle_task(todos, target_id, now);
            });
        }
    }

    pub fn delete_today_todo(&mut self) {
        let rows = self.visible_rows_today();
        if let Some(row) = rows.get(self.today_selected_index) {
            let target_id = row.id;
            if let Some(todo) = self.todos.iter().find(|t| t.id == target_id) {
                if todo.recurring_id.is_some() {
                    self.input_mode = InputMode::ConfirmDeleteRecurring(target_id);
                    return;
                }
            }
            let mut deleted_text = None;
            self.mutate(|todos| {
                if let Some(t) = crate::store::delete_task(todos, target_id) {
                    deleted_text = Some(t.text);
                }
            });
            if let Some(text) = deleted_text {
                self.set_transient_message(format!(
                    "Deleted \"{}\" · u to undo",
                    truncate_str(&text, 24)
                ));
            }
        }
    }

    pub fn toggle_history_todo(&mut self) {
        let rows = self.visible_rows_history();
        if let Some(row) = rows.get(self.history_todo_index) {
            let target_id = row.id;
            let now = Local::now();
            self.mutate(|todos| {
                crate::store::toggle_task(todos, target_id, now);
            });
        }
    }

    pub fn delete_history_todo(&mut self) {
        let rows = self.visible_rows_history();
        if let Some(row) = rows.get(self.history_todo_index) {
            let target_id = row.id;
            if let Some(todo) = self.todos.iter().find(|t| t.id == target_id) {
                if todo.recurring_id.is_some() {
                    self.input_mode = InputMode::ConfirmDeleteRecurring(target_id);
                    return;
                }
            }
            let mut deleted_text = None;
            self.mutate(|todos| {
                if let Some(t) = crate::store::delete_task(todos, target_id) {
                    deleted_text = Some(t.text);
                }
            });
            if let Some(text) = deleted_text {
                self.set_transient_message(format!(
                    "Deleted \"{}\" · u to undo",
                    truncate_str(&text, 24)
                ));
            }
        }
    }

    pub fn move_history_todo_to_today(&mut self) {
        self.move_selected_todo_to_today();
    }

    pub fn toggle_upcoming_todo(&mut self) {
        let rows = self.visible_rows_upcoming();
        if let Some(row) = rows.get(self.upcoming_todo_index) {
            let target_id = row.id;
            let now = Local::now();
            self.mutate(|todos| {
                crate::store::toggle_task(todos, target_id, now);
            });
        }
    }

    pub fn delete_upcoming_todo(&mut self) {
        let rows = self.visible_rows_upcoming();
        if let Some(row) = rows.get(self.upcoming_todo_index) {
            let target_id = row.id;
            if let Some(todo) = self.todos.iter().find(|t| t.id == target_id) {
                if todo.recurring_id.is_some() {
                    self.input_mode = InputMode::ConfirmDeleteRecurring(target_id);
                    return;
                }
            }
            let mut deleted_text = None;
            self.mutate(|todos| {
                if let Some(t) = crate::store::delete_task(todos, target_id) {
                    deleted_text = Some(t.text);
                }
            });
            if let Some(text) = deleted_text {
                self.set_transient_message(format!(
                    "Deleted \"{}\" · u to undo",
                    truncate_str(&text, 24)
                ));
            }
        }
    }

    pub fn delete_recurring_instance(&mut self, target_id: u64) {
        let mut deleted_text = None;
        self.mutate(|todos| {
            if let Some(t) = crate::store::delete_task(todos, target_id) {
                deleted_text = Some(t.text);
            }
        });
        if let Some(text) = deleted_text {
            self.set_transient_message(format!(
                "Deleted \"{}\" · u to undo",
                truncate_str(&text, 24)
            ));
        }
    }

    pub fn delete_recurring_all(&mut self, target_id: u64) {
        let Some(todo) = self.todos.iter().find(|t| t.id == target_id).cloned() else {
            return;
        };
        let tpl_id = todo.recurring_id;

        self.snapshot();

        crate::store::delete_task(&mut self.todos, target_id);

        if let Some(tid) = tpl_id {
            self.recurring.retain(|r| r.id != tid);

            let future_instance_ids: Vec<u64> = self
                .todos
                .iter()
                .filter(|t| t.recurring_id == Some(tid) && t.created_date > todo.created_date)
                .map(|t| t.id)
                .collect();

            for fid in future_instance_ids {
                crate::store::delete_task(&mut self.todos, fid);
            }

            for t in &mut self.todos {
                if t.recurring_id == Some(tid) {
                    t.recurring_id = None;
                }
            }
        }

        if self.focus == Some(target_id) {
            self.focus = None;
        }

        self.save();
        self.re_resolve_selections();
        self.set_transient_message(format!(
            "Deleted recurring \"{}\" and template · u to undo",
            truncate_str(&todo.text, 24)
        ));
    }

    pub fn reorder_upcoming(&mut self, down: bool) {
        if self.sort_mode != crate::model::SortMode::Manual {
            self.set_transient_message("Reorder works in manual sort");
            return;
        }
        let dates = self.upcoming_dates_list();
        if let Some(&selected_date) = dates.get(self.upcoming_date_index) {
            let selected_id = match self
                .visible_rows_upcoming()
                .get(self.upcoming_todo_index)
                .map(|r| r.id)
            {
                Some(id) => id,
                None => return,
            };
            if !crate::store::can_reorder_day_task(&self.todos, selected_date, selected_id, down) {
                return;
            }
            self.upcoming_todo_id = Some(selected_id);
            self.mutate(|todos| {
                crate::store::reorder_day_task(todos, selected_date, selected_id, down);
            });
        }
    }

    pub fn selected_task_id(&self) -> Option<u64> {
        match self.current_view {
            View::Today => self
                .visible_rows_today()
                .get(self.today_selected_index)
                .map(|r| r.id),
            View::Upcoming => {
                if self.upcoming_pane == HistoryPane::Todos {
                    self.visible_rows_upcoming()
                        .get(self.upcoming_todo_index)
                        .map(|r| r.id)
                } else {
                    None
                }
            }
            View::History => {
                if self.history_pane == HistoryPane::Todos {
                    self.visible_rows_history()
                        .get(self.history_todo_index)
                        .map(|r| r.id)
                } else {
                    None
                }
            }
        }
    }

    pub fn cycle_selected_priority(&mut self) {
        if let Some(id) = self.selected_task_id() {
            self.mutate(|todos| {
                if let Some(t) = todos.iter_mut().find(|t| t.id == id) {
                    t.priority = match t.priority {
                        0 => 1,
                        1 => 2,
                        _ => 0,
                    };
                }
            });
        }
    }

    pub fn toggle_focus(&mut self) {
        let Some(target_id) = self.selected_task_id() else {
            return;
        };

        if let Some(todo) = self.todos.iter().find(|t| t.id == target_id) {
            if todo.done {
                return;
            }
        }

        self.snapshot();

        if self.focus == Some(target_id) {
            self.focus = None;
        } else {
            self.focus = Some(target_id);
        }

        self.save();
        self.re_resolve_selections();
    }

    pub fn toggle_mark_selected(&mut self) {
        if let Some(id) = self.selected_task_id() {
            if self.marks.contains(&id) {
                self.marks.remove(&id);
            } else {
                self.marks.insert(id);
            }
        }
    }

    pub fn toggle_mark_all_visible(&mut self) {
        let visible_ids: Vec<u64> = match self.current_view {
            View::Today => self.visible_rows_today().iter().map(|r| r.id).collect(),
            View::Upcoming => {
                if self.upcoming_pane == HistoryPane::Todos {
                    self.visible_rows_upcoming().iter().map(|r| r.id).collect()
                } else {
                    Vec::new()
                }
            }
            View::History => {
                if self.history_pane == HistoryPane::Todos {
                    self.visible_rows_history().iter().map(|r| r.id).collect()
                } else {
                    Vec::new()
                }
            }
        };

        if visible_ids.is_empty() {
            return;
        }

        let all_marked = visible_ids.iter().all(|id| self.marks.contains(id));
        if all_marked {
            for id in &visible_ids {
                self.marks.remove(id);
            }
        } else {
            for id in visible_ids {
                self.marks.insert(id);
            }
        }
    }

    pub fn bulk_toggle(&mut self) {
        if self.marks.is_empty() {
            return;
        }
        let marked_ids: std::collections::HashSet<u64> = self.marks.clone();
        let all_done = self
            .todos
            .iter()
            .filter(|t| marked_ids.contains(&t.id))
            .all(|t| t.done);
        let now = Local::now();

        self.mutate(|todos| {
            if all_done {
                for t in todos.iter_mut() {
                    if marked_ids.contains(&t.id) {
                        t.done = false;
                        t.done_at = None;
                    }
                }
                let undone_parents: Vec<u64> = todos
                    .iter()
                    .filter(|t| !t.done && t.parent.is_some())
                    .filter_map(|t| t.parent)
                    .collect();
                for t in todos.iter_mut() {
                    if undone_parents.contains(&t.id) {
                        t.done = false;
                        t.done_at = None;
                    }
                }
            } else {
                for t in todos.iter_mut() {
                    if marked_ids.contains(&t.id) {
                        t.done = true;
                        t.done_at = Some(now);
                    }
                }
                let done_parents: Vec<u64> = todos
                    .iter()
                    .filter(|t| t.done && t.parent.is_none())
                    .map(|t| t.id)
                    .collect();
                for t in todos.iter_mut() {
                    if let Some(p) = t.parent {
                        if done_parents.contains(&p) {
                            t.done = true;
                            t.done_at = Some(now);
                        }
                    }
                }
            }
        });
        self.marks.clear();
        self.re_resolve_selections();
    }

    pub fn bulk_delete(&mut self) {
        if self.marks.is_empty() {
            return;
        }
        let marked_ids: Vec<u64> = self.marks.iter().copied().collect();
        let count = marked_ids.len();

        self.mutate(|todos| {
            for id in marked_ids {
                crate::store::delete_task(todos, id);
            }
        });

        self.marks.clear();
        self.re_resolve_selections();
        self.set_transient_message(format!("Deleted {count} tasks · u to undo"));
    }

    pub fn bulk_move_to_today(&mut self) {
        if self.marks.is_empty() {
            return;
        }
        let marked_ids: Vec<u64> = self.marks.iter().copied().collect();
        let count = marked_ids.len();
        let today = self.today;

        self.mutate(|todos| {
            for id in marked_ids {
                crate::store::move_task_to_date(todos, id, today);
            }
        });

        self.marks.clear();
        self.re_resolve_selections();
        self.set_transient_message(format!("Moved {count} tasks to Today · u to undo"));
    }

    pub fn set_recurrence_off(&mut self, target_id: u64) {
        let Some(todo) = self.todos.iter().find(|t| t.id == target_id).cloned() else {
            return;
        };
        let Some(tpl_id) = todo.recurring_id else {
            self.set_transient_message("Task is not recurring");
            return;
        };

        self.snapshot();

        self.recurring.retain(|r| r.id != tpl_id);
        for t in &mut self.todos {
            if t.recurring_id == Some(tpl_id) {
                t.recurring_id = None;
            }
        }

        self.save();
        self.re_resolve_selections();
        self.set_transient_message("Recurrence off");
    }

    pub fn set_recurrence_rule(&mut self, target_id: u64, rule: crate::model::RecurrenceRule) {
        let Some(todo) = self.todos.iter().find(|t| t.id == target_id).cloned() else {
            return;
        };
        if todo.parent.is_some() {
            self.set_transient_message("Subtasks cannot be recurring");
            return;
        }

        self.snapshot();

        let rule_name = match rule {
            crate::model::RecurrenceRule::Daily => "daily",
            crate::model::RecurrenceRule::Weekdays => "weekdays",
            crate::model::RecurrenceRule::Weekly => "weekly",
        };

        if let Some(tpl_id) = todo.recurring_id {
            if let Some(tpl) = self.recurring.iter_mut().find(|r| r.id == tpl_id) {
                tpl.rule = rule;
                if rule == crate::model::RecurrenceRule::Weekly {
                    tpl.anchor = todo.created_date;
                }
            }
        } else {
            let tpl_id = self.next_id;
            self.next_id += 1;
            let anchor = match rule {
                crate::model::RecurrenceRule::Weekdays => {
                    let wd = todo.created_date.weekday();
                    if wd == chrono::Weekday::Sat {
                        todo.created_date
                            .succ_opt()
                            .and_then(|d| d.succ_opt())
                            .unwrap_or(todo.created_date)
                    } else if wd == chrono::Weekday::Sun {
                        todo.created_date.succ_opt().unwrap_or(todo.created_date)
                    } else {
                        todo.created_date
                    }
                }
                _ => todo.created_date,
            };

            self.recurring.push(crate::model::Recurring {
                id: tpl_id,
                text: todo.text.clone(),
                tags: todo.tags.clone(),
                priority: todo.priority,
                rule,
                anchor,
                last_materialized: Some(todo.created_date),
            });

            if let Some(t) = self.todos.iter_mut().find(|t| t.id == target_id) {
                t.recurring_id = Some(tpl_id);
            }
        }

        crate::store::materialize_recurring(
            &mut self.todos,
            &mut self.recurring,
            &mut self.next_id,
            self.today,
        );

        self.save();
        self.re_resolve_selections();
        self.set_transient_message(format!("Recurrence set to {rule_name}"));
    }

    pub fn reschedule_task(&mut self, id: u64, target_date: NaiveDate) {
        if target_date < self.today {
            self.set_transient_message("Cannot reschedule to a past date");
            return;
        }
        self.mutate(|todos| {
            crate::store::move_task_to_date(todos, id, target_date);
        });
        self.set_transient_message(format!(
            "Scheduled for {}",
            target_date.format(&self.config.short_date_format)
        ));
    }

    pub fn move_selected_todo_to_today(&mut self) {
        match self.current_view {
            View::Today => {
                self.set_transient_message("Already in Today");
            }
            View::History => {
                if self.history_pane != HistoryPane::Todos {
                    return;
                }
                let dates = self.dates_list();
                let Some(&selected_date) = dates.get(self.history_date_index) else {
                    return;
                };
                if selected_date == self.today {
                    self.set_transient_message("Already in Today");
                    return;
                }
                let rows = self.visible_rows_history();
                if let Some(row) = rows.get(self.history_todo_index) {
                    let todo_id = row.id;
                    let today = self.today;
                    self.mutate(|todos| {
                        crate::store::move_task_to_date(todos, todo_id, today);
                    });
                    self.set_transient_message("Moved to Today");
                }
            }
            View::Upcoming => {
                if self.upcoming_pane != HistoryPane::Todos {
                    return;
                }
                let dates = self.upcoming_dates_list();
                let Some(&selected_date) = dates.get(self.upcoming_date_index) else {
                    return;
                };
                if selected_date == self.today {
                    self.set_transient_message("Already in Today");
                    return;
                }
                let rows = self.visible_rows_upcoming();
                if let Some(row) = rows.get(self.upcoming_todo_index) {
                    let todo_id = row.id;
                    let today = self.today;
                    self.mutate(|todos| {
                        crate::store::move_task_to_date(todos, todo_id, today);
                    });
                    self.set_transient_message("Moved to Today");
                }
            }
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
            InputMode::AddingSubtask(parent_id) => self.handle_adding_subtask_key(parent_id, key),
            InputMode::Editing(id) => self.handle_editing_key(id, key),
            InputMode::Rescheduling(id) => self.handle_rescheduling_key(id, key),
            InputMode::ReschedulingBulk => self.handle_rescheduling_bulk_key(key),
            InputMode::Filtering => self.handle_filtering_key(key),
            InputMode::EditingNote(id) => self.handle_note_key(id, key),
            InputMode::SettingRecurrence(id) => self.handle_recurrence_key(id, key),
            InputMode::ConfirmDeleteRecurring(id) => {
                self.handle_confirm_delete_recurring_key(id, key)
            }
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
                            crate::ui::HitTarget::UpcomingCheckbox { index } => {
                                self.upcoming_pane = HistoryPane::Todos;
                                self.upcoming_todo_index = index;
                                self.toggle_upcoming_todo();
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
                                self.marks.clear();
                                self.clamp_selections();
                                return true;
                            }
                            crate::ui::HitTarget::TabUpcoming => {
                                if self.config.ui.show_upcoming {
                                    self.current_view = View::Upcoming;
                                    self.marks.clear();
                                    self.clamp_selections();
                                    return true;
                                }
                            }
                            crate::ui::HitTarget::TabHistory => {
                                if self.config.ui.show_history {
                                    self.current_view = View::History;
                                    self.marks.clear();
                                    self.clamp_selections();
                                    return true;
                                }
                            }
                            crate::ui::HitTarget::TodayTask { index } => {
                                self.today_selected_index = index;
                                self.today_selected_id =
                                    self.visible_rows_today().get(index).map(|r| r.id);
                                return true;
                            }
                            crate::ui::HitTarget::UpcomingDate { index } => {
                                self.upcoming_pane = HistoryPane::Dates;
                                self.upcoming_date_index = index;
                                self.upcoming_todo_index = 0;
                                self.clamp_selections();
                                return true;
                            }
                            crate::ui::HitTarget::UpcomingTask { index } => {
                                self.upcoming_pane = HistoryPane::Todos;
                                self.upcoming_todo_index = index;
                                self.upcoming_todo_id =
                                    self.visible_rows_upcoming().get(index).map(|r| r.id);
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
                                self.history_todo_id =
                                    self.visible_rows_history().get(index).map(|r| r.id);
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
                        let count = self.visible_rows_today().len();
                        if count > 0 && self.today_selected_index + 1 < count {
                            self.today_selected_index += 1;
                            self.today_selected_id = self
                                .visible_rows_today()
                                .get(self.today_selected_index)
                                .map(|r| r.id);
                            return true;
                        }
                    }
                    View::Upcoming => {
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
                            self.upcoming_pane == HistoryPane::Dates
                        };

                        if scroll_dates {
                            let dates = self.upcoming_dates_list();
                            if !dates.is_empty() && self.upcoming_date_index + 1 < dates.len() {
                                self.upcoming_date_index += 1;
                                self.upcoming_todo_index = 0;
                                self.clamp_selections();
                                return true;
                            }
                        } else {
                            let count = self.visible_rows_upcoming().len();
                            if count > 0 && self.upcoming_todo_index + 1 < count {
                                self.upcoming_todo_index += 1;
                                self.upcoming_todo_id = self
                                    .visible_rows_upcoming()
                                    .get(self.upcoming_todo_index)
                                    .map(|r| r.id);
                                return true;
                            }
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
                            let count = self.visible_rows_history().len();
                            if count > 0 && self.history_todo_index + 1 < count {
                                self.history_todo_index += 1;
                                self.history_todo_id = self
                                    .visible_rows_history()
                                    .get(self.history_todo_index)
                                    .map(|r| r.id);
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
                            self.today_selected_id = self
                                .visible_rows_today()
                                .get(self.today_selected_index)
                                .map(|r| r.id);
                            return true;
                        }
                    }
                    View::Upcoming => {
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
                            self.upcoming_pane == HistoryPane::Dates
                        };

                        if scroll_dates {
                            if self.upcoming_date_index > 0 {
                                self.upcoming_date_index -= 1;
                                self.upcoming_todo_index = 0;
                                self.clamp_selections();
                                return true;
                            }
                        } else if self.upcoming_todo_index > 0 {
                            self.upcoming_todo_index -= 1;
                            self.upcoming_todo_id = self
                                .visible_rows_upcoming()
                                .get(self.upcoming_todo_index)
                                .map(|r| r.id);
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
                            self.history_todo_id = self
                                .visible_rows_history()
                                .get(self.history_todo_index)
                                .map(|r| r.id);
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
                self.prompt_error = None;
            }
            KeyCode::Enter => {
                let input = self.input_buffer.clone();
                let today = self.today;
                match crate::parser::parse_quick_add(&input, today) {
                    Ok(parsed) => {
                        let cur_view = self.current_view;
                        let target_date = parsed.date.unwrap_or_else(|| match cur_view {
                            View::Today => today,
                            View::History => self.selected_history_date.unwrap_or(today),
                            View::Upcoming => self
                                .selected_upcoming_date
                                .unwrap_or_else(|| today.succ_opt().unwrap_or(today)),
                        });

                        let id = self.next_id;
                        self.next_id += 1;
                        let text = parsed.text;
                        let priority = parsed.priority;
                        let tags = parsed.tags;

                        let (recurring_id, actual_date) = if let Some(rule) = parsed.recurrence {
                            self.snapshot();
                            let tpl_id = self.next_id;
                            self.next_id += 1;
                            let anchor = match rule {
                                crate::model::RecurrenceRule::Weekdays => {
                                    let wd = target_date.weekday();
                                    if wd == chrono::Weekday::Sat {
                                        target_date
                                            .succ_opt()
                                            .and_then(|d| d.succ_opt())
                                            .unwrap_or(target_date)
                                    } else if wd == chrono::Weekday::Sun {
                                        target_date.succ_opt().unwrap_or(target_date)
                                    } else {
                                        target_date
                                    }
                                }
                                _ => target_date,
                            };
                            self.recurring.push(crate::model::Recurring {
                                id: tpl_id,
                                text: text.clone(),
                                tags: tags.clone(),
                                priority,
                                rule,
                                anchor,
                                last_materialized: Some(anchor),
                            });
                            (Some(tpl_id), anchor)
                        } else {
                            (None, target_date)
                        };

                        let added_task = Todo {
                            id,
                            text,
                            created_date: actual_date,
                            done: false,
                            done_at: None,
                            priority,
                            tags,
                            note: None,
                            parent: None,
                            recurring_id,
                        };

                        let is_hidden_by_filter = !self.current_filter().matches(&added_task);

                        if recurring_id.is_some() {
                            self.todos.push(added_task);
                            crate::store::materialize_recurring(
                                &mut self.todos,
                                &mut self.recurring,
                                &mut self.next_id,
                                self.today,
                            );
                            self.save();
                            self.re_resolve_selections();
                        } else {
                            self.mutate(|todos| {
                                todos.push(added_task);
                            });
                        }

                        if is_hidden_by_filter {
                            self.set_transient_message("Added (hidden by filter)");
                        } else {
                            match cur_view {
                                View::Today => {
                                    self.today_selected_id = Some(id);
                                    if actual_date != today {
                                        self.set_transient_message(format!(
                                            "Scheduled for {}",
                                            actual_date.format(&self.config.short_date_format)
                                        ));
                                    }
                                }
                                View::Upcoming => {
                                    self.upcoming_todo_id = Some(id);
                                    let cur_up_date = self.selected_upcoming_date.unwrap_or(today);
                                    if actual_date != cur_up_date {
                                        self.set_transient_message(format!(
                                            "Scheduled for {}",
                                            actual_date.format(&self.config.short_date_format)
                                        ));
                                    }
                                }
                                View::History => {
                                    self.history_todo_id = Some(id);
                                }
                            }
                        }

                        self.input_mode = InputMode::Normal;
                        self.input_buffer.clear();
                        self.cursor_position = 0;
                        self.prompt_error = None;
                    }
                    Err(e) => {
                        self.prompt_error = Some(e.to_string());
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
                self.prompt_error = None;
                if self.cursor_position > 0 {
                    let mut chars: Vec<char> = self.input_buffer.chars().collect();
                    chars.remove(self.cursor_position - 1);
                    self.input_buffer = chars.into_iter().collect();
                    self.cursor_position -= 1;
                }
            }
            KeyCode::Delete => {
                self.prompt_error = None;
                let mut chars: Vec<char> = self.input_buffer.chars().collect();
                if self.cursor_position < chars.len() {
                    chars.remove(self.cursor_position);
                    self.input_buffer = chars.into_iter().collect();
                }
            }
            KeyCode::Char(c) => {
                self.prompt_error = None;
                let mut chars: Vec<char> = self.input_buffer.chars().collect();
                chars.insert(self.cursor_position, c);
                self.input_buffer = chars.into_iter().collect();
                self.cursor_position += 1;
            }
            _ => {}
        }
        false
    }

    pub fn start_adding_subtask(&mut self) {
        let Some(selected_id) = self.selected_task_id() else {
            return;
        };
        let Some(task) = self.todos.iter().find(|t| t.id == selected_id) else {
            return;
        };
        let parent_id = task.parent.unwrap_or(task.id);
        self.input_mode = InputMode::AddingSubtask(parent_id);
        self.input_buffer.clear();
        self.cursor_position = 0;
        self.prompt_error = None;
    }

    fn handle_adding_subtask_key(&mut self, parent_id: u64, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.input_buffer.clear();
                self.cursor_position = 0;
                self.prompt_error = None;
            }
            KeyCode::Enter => {
                let input = self.input_buffer.clone();
                let today = self.today;
                match crate::parser::parse_quick_add(&input, today) {
                    Ok(parsed) => {
                        if parsed.date.is_some() || parsed.recurrence.is_some() {
                            self.prompt_error = Some(
                                "Subtasks inherit parent date; date/recurrence tokens not allowed"
                                    .to_string(),
                            );
                            return false;
                        }

                        let parent_date = match self.todos.iter().find(|t| t.id == parent_id) {
                            Some(p) => p.created_date,
                            None => {
                                self.input_mode = InputMode::Normal;
                                return false;
                            }
                        };

                        let id = self.next_id;
                        self.next_id += 1;
                        let text = parsed.text;
                        let priority = parsed.priority;
                        let tags = parsed.tags;

                        let added_subtask = Todo {
                            id,
                            text,
                            created_date: parent_date,
                            done: false,
                            done_at: None,
                            priority,
                            tags,
                            note: None,
                            parent: Some(parent_id),
                            recurring_id: None,
                        };

                        let is_hidden_by_filter = !self.current_filter().matches(&added_subtask);

                        self.mutate(|todos| {
                            let insert_pos = todos
                                .iter()
                                .rposition(|t| t.id == parent_id || t.parent == Some(parent_id))
                                .map(|p| p + 1)
                                .unwrap_or(todos.len());
                            todos.insert(insert_pos, added_subtask);
                        });

                        if is_hidden_by_filter {
                            self.set_transient_message("Added (hidden by filter)");
                        } else {
                            match self.current_view {
                                View::Today => self.today_selected_id = Some(id),
                                View::Upcoming => self.upcoming_todo_id = Some(id),
                                View::History => self.history_todo_id = Some(id),
                            }
                        }

                        self.input_mode = InputMode::Normal;
                        self.input_buffer.clear();
                        self.cursor_position = 0;
                        self.prompt_error = None;
                    }
                    Err(e) => {
                        self.prompt_error = Some(e.to_string());
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
                self.prompt_error = None;
                if self.cursor_position > 0 {
                    let mut chars: Vec<char> = self.input_buffer.chars().collect();
                    chars.remove(self.cursor_position - 1);
                    self.input_buffer = chars.into_iter().collect();
                    self.cursor_position -= 1;
                }
            }
            KeyCode::Delete => {
                self.prompt_error = None;
                let mut chars: Vec<char> = self.input_buffer.chars().collect();
                if self.cursor_position < chars.len() {
                    chars.remove(self.cursor_position);
                    self.input_buffer = chars.into_iter().collect();
                }
            }
            KeyCode::Char(c) => {
                self.prompt_error = None;
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
                self.prompt_error = None;
            }
            KeyCode::Enter => {
                let input = self.input_buffer.clone();
                let today = self.today;
                match crate::parser::parse_quick_add(&input, today) {
                    Ok(parsed) => {
                        self.mutate(|todos| {
                            if let Some(todo) = todos.iter_mut().find(|t| t.id == target_id) {
                                todo.text = parsed.text;
                                todo.priority = parsed.priority;
                                todo.tags = parsed.tags;
                                if let Some(d) = parsed.date {
                                    todo.created_date = d;
                                }
                            }
                        });
                        self.set_transient_message("Edited task · u to undo");
                        self.input_mode = InputMode::Normal;
                        self.input_buffer.clear();
                        self.cursor_position = 0;
                        self.prompt_error = None;
                    }
                    Err(e) => {
                        self.prompt_error = Some(e.to_string());
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
                self.prompt_error = None;
                if self.cursor_position > 0 {
                    let mut chars: Vec<char> = self.input_buffer.chars().collect();
                    chars.remove(self.cursor_position - 1);
                    self.input_buffer = chars.into_iter().collect();
                    self.cursor_position -= 1;
                }
            }
            KeyCode::Delete => {
                self.prompt_error = None;
                let mut chars: Vec<char> = self.input_buffer.chars().collect();
                if self.cursor_position < chars.len() {
                    chars.remove(self.cursor_position);
                    self.input_buffer = chars.into_iter().collect();
                }
            }
            KeyCode::Char(c) => {
                self.prompt_error = None;
                let mut chars: Vec<char> = self.input_buffer.chars().collect();
                chars.insert(self.cursor_position, c);
                self.input_buffer = chars.into_iter().collect();
                self.cursor_position += 1;
            }
            _ => {}
        }
        false
    }

    fn handle_rescheduling_key(&mut self, target_id: u64, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.input_buffer.clear();
                self.cursor_position = 0;
                self.prompt_error = None;
            }
            KeyCode::Enter => {
                let input = self.input_buffer.trim().to_string();
                let today = self.today;
                match crate::parser::parse_date_expression(&input, today) {
                    Ok(target_date) => {
                        if target_date < today {
                            self.prompt_error =
                                Some("Cannot reschedule to a past date".to_string());
                            return false;
                        }
                        self.reschedule_task(target_id, target_date);
                        self.input_mode = InputMode::Normal;
                        self.input_buffer.clear();
                        self.cursor_position = 0;
                        self.prompt_error = None;
                    }
                    Err(e) => {
                        self.prompt_error = Some(e);
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
                self.prompt_error = None;
                if self.cursor_position > 0 {
                    let mut chars: Vec<char> = self.input_buffer.chars().collect();
                    chars.remove(self.cursor_position - 1);
                    self.input_buffer = chars.into_iter().collect();
                    self.cursor_position -= 1;
                }
            }
            KeyCode::Delete => {
                self.prompt_error = None;
                let mut chars: Vec<char> = self.input_buffer.chars().collect();
                if self.cursor_position < chars.len() {
                    chars.remove(self.cursor_position);
                    self.input_buffer = chars.into_iter().collect();
                }
            }
            KeyCode::Char(c) => {
                self.prompt_error = None;
                let mut chars: Vec<char> = self.input_buffer.chars().collect();
                chars.insert(self.cursor_position, c);
                self.input_buffer = chars.into_iter().collect();
                self.cursor_position += 1;
            }
            _ => {}
        }
        false
    }

    fn handle_rescheduling_bulk_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.input_buffer.clear();
                self.cursor_position = 0;
                self.prompt_error = None;
            }
            KeyCode::Enter => {
                let input = self.input_buffer.trim().to_string();
                let today = self.today;
                match crate::parser::parse_date_expression(&input, today) {
                    Ok(target_date) => {
                        if target_date < today {
                            self.prompt_error =
                                Some("Cannot reschedule to a past date".to_string());
                            return false;
                        }
                        let marked_ids: Vec<u64> = self.marks.iter().copied().collect();
                        let count = marked_ids.len();
                        self.mutate(|todos| {
                            for id in marked_ids {
                                crate::store::move_task_to_date(todos, id, target_date);
                            }
                        });
                        self.marks.clear();
                        self.input_mode = InputMode::Normal;
                        self.input_buffer.clear();
                        self.cursor_position = 0;
                        self.prompt_error = None;
                        self.set_transient_message(format!(
                            "Scheduled {count} tasks for {}",
                            target_date.format(&self.config.short_date_format)
                        ));
                    }
                    Err(e) => {
                        self.prompt_error = Some(e);
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
                self.prompt_error = None;
                if self.cursor_position > 0 {
                    let mut chars: Vec<char> = self.input_buffer.chars().collect();
                    chars.remove(self.cursor_position - 1);
                    self.input_buffer = chars.into_iter().collect();
                    self.cursor_position -= 1;
                }
            }
            KeyCode::Delete => {
                self.prompt_error = None;
                let mut chars: Vec<char> = self.input_buffer.chars().collect();
                if self.cursor_position < chars.len() {
                    chars.remove(self.cursor_position);
                    self.input_buffer = chars.into_iter().collect();
                }
            }
            KeyCode::Char(c) => {
                self.prompt_error = None;
                let mut chars: Vec<char> = self.input_buffer.chars().collect();
                chars.insert(self.cursor_position, c);
                self.input_buffer = chars.into_iter().collect();
                self.cursor_position += 1;
            }
            _ => {}
        }
        false
    }

    fn handle_recurrence_key(&mut self, target_id: u64, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.input_buffer.clear();
                self.cursor_position = 0;
                self.prompt_error = None;
            }
            KeyCode::Enter => {
                let input = self.input_buffer.trim().to_lowercase();
                if input == "off" {
                    self.set_recurrence_off(target_id);
                    self.input_mode = InputMode::Normal;
                    self.input_buffer.clear();
                    self.cursor_position = 0;
                    self.prompt_error = None;
                } else if input == "daily" || input == "weekdays" || input == "weekly" {
                    let rule = match input.as_str() {
                        "daily" => crate::model::RecurrenceRule::Daily,
                        "weekdays" => crate::model::RecurrenceRule::Weekdays,
                        "weekly" => crate::model::RecurrenceRule::Weekly,
                        _ => unreachable!(),
                    };
                    self.set_recurrence_rule(target_id, rule);
                    self.input_mode = InputMode::Normal;
                    self.input_buffer.clear();
                    self.cursor_position = 0;
                    self.prompt_error = None;
                } else {
                    self.prompt_error =
                        Some("Expected 'daily', 'weekdays', 'weekly', or 'off'".to_string());
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
                self.prompt_error = None;
                if self.cursor_position > 0 {
                    let mut chars: Vec<char> = self.input_buffer.chars().collect();
                    chars.remove(self.cursor_position - 1);
                    self.input_buffer = chars.into_iter().collect();
                    self.cursor_position -= 1;
                }
            }
            KeyCode::Delete => {
                self.prompt_error = None;
                let mut chars: Vec<char> = self.input_buffer.chars().collect();
                if self.cursor_position < chars.len() {
                    chars.remove(self.cursor_position);
                    self.input_buffer = chars.into_iter().collect();
                }
            }
            KeyCode::Char(c) => {
                self.prompt_error = None;
                let mut chars: Vec<char> = self.input_buffer.chars().collect();
                chars.insert(self.cursor_position, c);
                self.input_buffer = chars.into_iter().collect();
                self.cursor_position += 1;
            }
            _ => {}
        }
        false
    }

    fn handle_confirm_delete_recurring_key(&mut self, target_id: u64, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Char('t') | KeyCode::Char('T') => {
                self.input_mode = InputMode::Normal;
                self.delete_recurring_instance(target_id);
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                self.input_mode = InputMode::Normal;
                self.delete_recurring_all(target_id);
            }
            _ => {}
        }
        false
    }

    fn handle_filtering_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.input_buffer.clear();
                self.cursor_position = 0;
                self.prompt_error = None;
            }
            KeyCode::Enter => {
                let filter = crate::parser::parse_filter(&self.input_buffer);
                *self.current_filter_mut() = filter;
                self.input_mode = InputMode::Normal;
                self.input_buffer.clear();
                self.cursor_position = 0;
                self.prompt_error = None;
                self.re_resolve_selections();
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
                self.prompt_error = None;
                if self.cursor_position > 0 {
                    let mut chars: Vec<char> = self.input_buffer.chars().collect();
                    chars.remove(self.cursor_position - 1);
                    self.input_buffer = chars.into_iter().collect();
                    self.cursor_position -= 1;
                }
            }
            KeyCode::Delete => {
                self.prompt_error = None;
                let mut chars: Vec<char> = self.input_buffer.chars().collect();
                if self.cursor_position < chars.len() {
                    chars.remove(self.cursor_position);
                    self.input_buffer = chars.into_iter().collect();
                }
            }
            KeyCode::Char(c) => {
                self.prompt_error = None;
                let mut chars: Vec<char> = self.input_buffer.chars().collect();
                chars.insert(self.cursor_position, c);
                self.input_buffer = chars.into_iter().collect();
                self.cursor_position += 1;
            }
            _ => {}
        }
        false
    }

    fn note_cursor_up(text: &str, cursor: usize) -> usize {
        let chars: Vec<char> = text.chars().collect();
        let clamped_cursor = cursor.min(chars.len());

        let mut line_starts = vec![0];
        for (i, &c) in chars.iter().enumerate() {
            if c == '\n' {
                line_starts.push(i + 1);
            }
        }

        let cur_line = match line_starts.binary_search(&clamped_cursor) {
            Ok(idx) => idx,
            Err(idx) => idx.saturating_sub(1),
        };

        if cur_line == 0 {
            return 0;
        }

        let cur_col = clamped_cursor - line_starts[cur_line];
        let prev_line_start = line_starts[cur_line - 1];
        let prev_line_end = line_starts[cur_line] - 1;
        let prev_line_len = prev_line_end - prev_line_start;

        prev_line_start + cur_col.min(prev_line_len)
    }

    fn note_cursor_down(text: &str, cursor: usize) -> usize {
        let chars: Vec<char> = text.chars().collect();
        let clamped_cursor = cursor.min(chars.len());

        let mut line_starts = vec![0];
        for (i, &c) in chars.iter().enumerate() {
            if c == '\n' {
                line_starts.push(i + 1);
            }
        }

        let cur_line = match line_starts.binary_search(&clamped_cursor) {
            Ok(idx) => idx,
            Err(idx) => idx.saturating_sub(1),
        };

        if cur_line + 1 >= line_starts.len() {
            return chars.len();
        }

        let cur_col = clamped_cursor - line_starts[cur_line];
        let next_line_start = line_starts[cur_line + 1];
        let next_line_end = if cur_line + 2 < line_starts.len() {
            line_starts[cur_line + 2] - 1
        } else {
            chars.len()
        };
        let next_line_len = next_line_end - next_line_start;

        next_line_start + cur_col.min(next_line_len)
    }

    fn note_cursor_home(text: &str, cursor: usize) -> usize {
        let chars: Vec<char> = text.chars().collect();
        let clamped_cursor = cursor.min(chars.len());
        let mut line_start = 0;
        for (i, &c) in chars.iter().enumerate().take(clamped_cursor) {
            if c == '\n' {
                line_start = i + 1;
            }
        }
        line_start
    }

    fn note_cursor_end(text: &str, cursor: usize) -> usize {
        let chars: Vec<char> = text.chars().collect();
        let clamped_cursor = cursor.min(chars.len());
        for (i, &c) in chars.iter().enumerate().skip(clamped_cursor) {
            if c == '\n' {
                return i;
            }
        }
        chars.len()
    }

    fn handle_note_key(&mut self, target_id: u64, key: KeyEvent) -> bool {
        let is_newline = (key.code == KeyCode::Enter && key.modifiers.contains(KeyModifiers::ALT))
            || (key.code == KeyCode::Char('j') && key.modifiers.contains(KeyModifiers::CONTROL));

        if is_newline {
            let mut chars: Vec<char> = self.note_buffer.chars().collect();
            if chars.len() < 2000 {
                chars.insert(self.note_cursor, '\n');
                self.note_buffer = chars.into_iter().collect();
                self.note_cursor += 1;
            }
            return false;
        }

        match key.code {
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.note_buffer.clear();
                self.note_cursor = 0;
            }
            KeyCode::Enter => {
                let clean = self.note_buffer.trim().to_string();
                let note_val = if clean.is_empty() {
                    None
                } else {
                    Some(self.note_buffer.clone())
                };
                self.mutate(|todos| {
                    if let Some(t) = todos.iter_mut().find(|t| t.id == target_id) {
                        t.note = note_val;
                    }
                });
                self.input_mode = InputMode::Normal;
                self.note_buffer.clear();
                self.note_cursor = 0;
            }
            KeyCode::Up => {
                self.note_cursor = Self::note_cursor_up(&self.note_buffer, self.note_cursor);
            }
            KeyCode::Down => {
                self.note_cursor = Self::note_cursor_down(&self.note_buffer, self.note_cursor);
            }
            KeyCode::Left => {
                self.note_cursor = self.note_cursor.saturating_sub(1);
            }
            KeyCode::Right => {
                let len = self.note_buffer.chars().count();
                if self.note_cursor < len {
                    self.note_cursor += 1;
                }
            }
            KeyCode::Home => {
                self.note_cursor = Self::note_cursor_home(&self.note_buffer, self.note_cursor);
            }
            KeyCode::End => {
                self.note_cursor = Self::note_cursor_end(&self.note_buffer, self.note_cursor);
            }
            KeyCode::Backspace => {
                if self.note_cursor > 0 {
                    let mut chars: Vec<char> = self.note_buffer.chars().collect();
                    chars.remove(self.note_cursor - 1);
                    self.note_buffer = chars.into_iter().collect();
                    self.note_cursor -= 1;
                }
            }
            KeyCode::Delete => {
                let mut chars: Vec<char> = self.note_buffer.chars().collect();
                if self.note_cursor < chars.len() {
                    chars.remove(self.note_cursor);
                    self.note_buffer = chars.into_iter().collect();
                }
            }
            KeyCode::Char(c) => {
                let mut chars: Vec<char> = self.note_buffer.chars().collect();
                if chars.len() < 2000 {
                    chars.insert(self.note_cursor, c);
                    self.note_buffer = chars.into_iter().collect();
                    self.note_cursor += 1;
                }
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

        if key.code == KeyCode::Esc {
            if !self.marks.is_empty() {
                self.marks.clear();
                return false;
            }
            if !self.current_filter().is_empty() {
                self.clear_current_filter();
                self.re_resolve_selections();
                return false;
            }
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
                self.marks.clear();
                let show_up = self.config.ui.show_upcoming;
                let show_hist = self.config.ui.show_history;
                self.current_view = match self.current_view {
                    View::Today => {
                        if show_up {
                            View::Upcoming
                        } else if show_hist {
                            View::History
                        } else {
                            View::Today
                        }
                    }
                    View::Upcoming => {
                        if show_hist {
                            View::History
                        } else {
                            View::Today
                        }
                    }
                    View::History => View::Today,
                };
                self.clamp_selections();
            }
            crate::keymap::Action::Add => {
                if self.current_view == View::History {
                    self.current_view = View::Today;
                }
                self.input_mode = InputMode::Adding;
                self.input_buffer.clear();
                self.cursor_position = 0;
                self.prompt_error = None;
            }
            crate::keymap::Action::AddSubtask => {
                self.start_adding_subtask();
            }
            crate::keymap::Action::Reschedule => {
                if !self.marks.is_empty() {
                    self.input_mode = InputMode::ReschedulingBulk;
                    self.input_buffer.clear();
                    self.cursor_position = 0;
                    self.prompt_error = None;
                } else {
                    let target_id = self.selected_task_id();
                    if let Some(id) = target_id {
                        self.input_mode = InputMode::Rescheduling(id);
                        self.input_buffer.clear();
                        self.cursor_position = 0;
                        self.prompt_error = None;
                    }
                }
            }
            crate::keymap::Action::ToggleMark => {
                self.toggle_mark_selected();
            }
            crate::keymap::Action::ToggleMarkAll => {
                self.toggle_mark_all_visible();
            }
            crate::keymap::Action::Toggle if !self.marks.is_empty() => {
                self.bulk_toggle();
            }
            crate::keymap::Action::Delete if !self.marks.is_empty() => {
                self.bulk_delete();
            }
            crate::keymap::Action::MoveToToday if !self.marks.is_empty() => {
                self.bulk_move_to_today();
            }
            crate::keymap::Action::CyclePriority => {
                self.cycle_selected_priority();
            }
            crate::keymap::Action::CycleSort => {
                self.sort_mode = self.sort_mode.next();
                self.re_resolve_selections();
            }
            crate::keymap::Action::Filter => {
                self.input_mode = InputMode::Filtering;
                self.input_buffer.clear();
                self.cursor_position = 0;
                self.prompt_error = None;
            }
            crate::keymap::Action::ToggleDetail => {
                self.show_detail_pane = !self.show_detail_pane;
            }
            crate::keymap::Action::EditNote => {
                if self.terminal_height < 20 {
                    self.set_transient_message("Terminal too small for note editor");
                } else if let Some(id) = self.selected_task_id() {
                    let existing_note = self
                        .todos
                        .iter()
                        .find(|t| t.id == id)
                        .and_then(|t| t.note.clone())
                        .unwrap_or_default();
                    self.note_cursor = existing_note.chars().count();
                    self.note_buffer = existing_note;
                    self.input_mode = InputMode::EditingNote(id);
                }
            }
            crate::keymap::Action::ToggleFocus => {
                self.toggle_focus();
            }
            crate::keymap::Action::Recurrence => {
                if let Some(id) = self.selected_task_id() {
                    if let Some(todo) = self.todos.iter().find(|t| t.id == id) {
                        if todo.parent.is_some() {
                            self.set_transient_message("Subtasks cannot be recurring");
                        } else {
                            self.input_mode = InputMode::SettingRecurrence(id);
                            self.input_buffer.clear();
                            self.cursor_position = 0;
                            self.prompt_error = None;
                        }
                    }
                }
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
                View::Upcoming => self.handle_upcoming_action(action),
                View::History => self.handle_history_action(action),
            },
        }

        false
    }

    fn handle_today_action(&mut self, action: crate::keymap::Action) {
        let count = self.visible_rows_today().len();

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
                    self.today_selected_id = self
                        .visible_rows_today()
                        .get(self.today_selected_index)
                        .map(|r| r.id);
                }
            }
            crate::keymap::Action::MoveSelectionUp => {
                self.today_selected_index = self.today_selected_index.saturating_sub(1);
                self.today_selected_id = self
                    .visible_rows_today()
                    .get(self.today_selected_index)
                    .map(|r| r.id);
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

    fn handle_upcoming_action(&mut self, action: crate::keymap::Action) {
        match self.upcoming_pane {
            HistoryPane::Dates => {
                let dates = self.upcoming_dates_list();
                match action {
                    crate::keymap::Action::MoveSelectionDown => {
                        if !dates.is_empty() && self.upcoming_date_index + 1 < dates.len() {
                            self.upcoming_date_index += 1;
                            self.upcoming_todo_index = 0;
                            self.clamp_selections();
                        }
                    }
                    crate::keymap::Action::MoveSelectionUp => {
                        if self.upcoming_date_index > 0 {
                            self.upcoming_date_index -= 1;
                            self.upcoming_todo_index = 0;
                            self.clamp_selections();
                        }
                    }
                    crate::keymap::Action::FocusTasks | crate::keymap::Action::Toggle => {
                        self.upcoming_pane = HistoryPane::Todos;
                    }
                    crate::keymap::Action::ClearCompleted => {
                        self.clear_completed_upcoming();
                    }
                    _ => {}
                }
            }
            HistoryPane::Todos => {
                let count = self.visible_rows_upcoming().len();

                match action {
                    crate::keymap::Action::FocusDates => {
                        self.upcoming_pane = HistoryPane::Dates;
                    }
                    crate::keymap::Action::ReorderDown => {
                        self.reorder_upcoming(true);
                    }
                    crate::keymap::Action::ReorderUp => {
                        self.reorder_upcoming(false);
                    }
                    crate::keymap::Action::MoveSelectionDown => {
                        if count > 0 && self.upcoming_todo_index + 1 < count {
                            self.upcoming_todo_index += 1;
                            self.upcoming_todo_id = self
                                .visible_rows_upcoming()
                                .get(self.upcoming_todo_index)
                                .map(|r| r.id);
                        }
                    }
                    crate::keymap::Action::MoveSelectionUp => {
                        self.upcoming_todo_index = self.upcoming_todo_index.saturating_sub(1);
                        self.upcoming_todo_id = self
                            .visible_rows_upcoming()
                            .get(self.upcoming_todo_index)
                            .map(|r| r.id);
                    }
                    crate::keymap::Action::Toggle => {
                        self.toggle_upcoming_todo();
                    }
                    crate::keymap::Action::Edit => {
                        self.start_editing_upcoming();
                    }
                    crate::keymap::Action::ClearCompleted => {
                        self.clear_completed_upcoming();
                    }
                    crate::keymap::Action::MoveToToday => {
                        self.move_selected_todo_to_today();
                    }
                    crate::keymap::Action::Delete => {
                        self.delete_upcoming_todo();
                    }
                    _ => {}
                }
            }
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
                let count = self.visible_rows_history().len();

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
                            self.history_todo_id = self
                                .visible_rows_history()
                                .get(self.history_todo_index)
                                .map(|r| r.id);
                        }
                    }
                    crate::keymap::Action::MoveSelectionUp => {
                        self.history_todo_index = self.history_todo_index.saturating_sub(1);
                        self.history_todo_id = self
                            .visible_rows_history()
                            .get(self.history_todo_index)
                            .map(|r| r.id);
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
            ..Default::default()
        };
        let todo2 = Todo {
            id: 2,
            text: "Second task".to_string(),
            created_date: today,
            done: false,
            done_at: None,
            ..Default::default()
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
            ..Default::default()
        };
        let mut app = App::new_for_test(vec![todo]);
        app.handle_key(key_press(KeyCode::Char('e')));
        assert_eq!(app.input_mode, InputMode::Editing(10));

        // Set to empty text
        app.input_buffer = "   ".to_string();
        app.handle_key(key_press(KeyCode::Enter));

        // Prompt stays open with dim error message, text unchanged
        assert_eq!(app.input_mode, InputMode::Editing(10));
        assert!(app.prompt_error.is_some());
        assert_eq!(app.todos[0].text, "Original text");
        assert_eq!(app.todos.len(), 1);

        // Cancel with Esc
        app.handle_key(key_press(KeyCode::Esc));
        assert_eq!(app.input_mode, InputMode::Normal);
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
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
        };
        let y2 = Todo {
            id: 2,
            text: "Finished".to_string(),
            created_date: yesterday,
            done: true,
            done_at: Some(Local::now()),
            ..Default::default()
        };
        let y3 = Todo {
            id: 3,
            text: "Unfinished 2".to_string(),
            created_date: yesterday,
            done: false,
            done_at: None,
            ..Default::default()
        };
        let t1 = Todo {
            id: 4,
            text: "Today task".to_string(),
            created_date: today,
            done: false,
            done_at: None,
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
        };
        let t2 = Todo {
            id: 2,
            text: "B".to_string(),
            created_date: today,
            done: false,
            done_at: None,
            ..Default::default()
        };
        let t3 = Todo {
            id: 3,
            text: "C".to_string(),
            created_date: today,
            done: false,
            done_at: None,
            ..Default::default()
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
            ..Default::default()
        };
        let t2 = Todo {
            id: 2,
            text: "B".to_string(),
            created_date: today,
            done: false,
            done_at: None,
            ..Default::default()
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
            ..Default::default()
        };
        let t2 = Todo {
            id: 20,
            text: "Past 2".to_string(),
            created_date: past,
            done: false,
            done_at: None,
            ..Default::default()
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
            ..Default::default()
        };
        let t2 = Todo {
            id: 2,
            text: "Unfinished".to_string(),
            created_date: today,
            done: false,
            done_at: None,
            ..Default::default()
        };
        let t3 = Todo {
            id: 3,
            text: "Done 2".to_string(),
            created_date: today,
            done: true,
            done_at: Some(Local::now()),
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
        };
        let t_past_todo = Todo {
            id: 2,
            text: "Past open".to_string(),
            created_date: past,
            done: false,
            done_at: None,
            ..Default::default()
        };
        let t_today_done = Todo {
            id: 3,
            text: "Today done".to_string(),
            created_date: today,
            done: true,
            done_at: Some(Local::now()),
            ..Default::default()
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
                ..Default::default()
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
            ..Default::default()
        };
        let t2 = Todo {
            id: 2,
            text: "Task 2".to_string(),
            created_date: d2,
            done: false,
            done_at: None,
            ..Default::default()
        };
        let t3 = Todo {
            id: 3,
            text: "Task 3".to_string(),
            created_date: d2,
            done: false,
            done_at: None,
            ..Default::default()
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
                ..Default::default()
            },
            Todo {
                id: 2,
                text: "Yesterday completed task".to_string(),
                created_date: yesterday,
                done: true,
                done_at: Some(Local::now()),
                ..Default::default()
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
            ..Default::default()
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
                ..Default::default()
            },
            Todo {
                id: 2,
                text: "Yesterday task".to_string(),
                created_date: yesterday,
                done: false,
                done_at: None,
                ..Default::default()
            },
            Todo {
                id: 3,
                text: "Today task".to_string(),
                created_date: today,
                done: false,
                done_at: None,
                ..Default::default()
            },
        ];

        let mut app = App::new_for_test_with_date(todos, today);

        // History dates list: newest first, always includes today
        let dates = app.dates_list();
        assert_eq!(dates, vec![today, yesterday, older]);

        // View switching with Tab: Today -> Upcoming -> History -> Today
        assert_eq!(app.current_view, View::Today);
        app.handle_key(key_press(KeyCode::Tab));
        assert_eq!(app.current_view, View::Upcoming);
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
            ..Default::default()
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
                ..Default::default()
            },
            Todo {
                id: 2,
                text: "Yesterday task".to_string(),
                created_date: day2,
                done: false,
                done_at: None,
                ..Default::default()
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
            ..Default::default()
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
        app.config.ui.show_upcoming = false;

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
                ..Default::default()
            },
            Todo {
                id: 2,
                text: "Task 2".to_string(),
                created_date: today,
                done: false,
                done_at: None,
                ..Default::default()
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
                ..Default::default()
            },
            Todo {
                id: 2,
                text: "Past task".to_string(),
                created_date: past,
                done: false,
                done_at: None,
                ..Default::default()
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
                ..Default::default()
            },
            Todo {
                id: 2,
                text: "Task 2".to_string(),
                created_date: today,
                done: false,
                done_at: None,
                ..Default::default()
            },
            Todo {
                id: 3,
                text: "Task 3".to_string(),
                created_date: today,
                done: false,
                done_at: None,
                ..Default::default()
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
            ..Default::default()
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

    #[test]
    fn test_upcoming_view_tab_cycling_and_skipping() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        let mut app = App::new_for_test_with_date(Vec::new(), today);

        // Default: Today -> Upcoming -> History -> Today
        assert_eq!(app.current_view, View::Today);
        app.handle_key(key_press(KeyCode::Tab));
        assert_eq!(app.current_view, View::Upcoming);
        app.handle_key(key_press(KeyCode::Tab));
        assert_eq!(app.current_view, View::History);
        app.handle_key(key_press(KeyCode::Tab));
        assert_eq!(app.current_view, View::Today);

        // With show_upcoming = false: Today -> History -> Today
        app.config.ui.show_upcoming = false;
        app.handle_key(key_press(KeyCode::Tab));
        assert_eq!(app.current_view, View::History);
        app.handle_key(key_press(KeyCode::Tab));
        assert_eq!(app.current_view, View::Today);

        // With show_history = false: Today -> Upcoming -> Today
        app.config.ui.show_upcoming = true;
        app.config.ui.show_history = false;
        app.handle_key(key_press(KeyCode::Tab));
        assert_eq!(app.current_view, View::Upcoming);
        app.handle_key(key_press(KeyCode::Tab));
        assert_eq!(app.current_view, View::Today);
    }

    #[test]
    fn test_quick_add_in_today_scheduled_for_future_shows_transient() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        let mut app = App::new_for_test_with_date(Vec::new(), today);

        // Open add prompt
        app.handle_key(key_press(KeyCode::Char('a')));
        assert_eq!(app.input_mode, InputMode::Adding);

        // Type task with future date token
        app.input_buffer = "Doctor appointment @tomorrow ! #health".to_string();
        app.handle_key(key_press(KeyCode::Enter));

        assert_eq!(app.input_mode, InputMode::Normal);
        assert_eq!(app.todos.len(), 1);
        assert_eq!(app.todos[0].text, "Doctor appointment");
        assert_eq!(app.todos[0].priority, 1);
        assert_eq!(app.todos[0].tags, vec!["health"]);
        assert_eq!(
            app.todos[0].created_date,
            NaiveDate::from_ymd_opt(2026, 10, 31).unwrap()
        );

        // Transient message shown: Scheduled for Sat 31 Oct
        assert!(app
            .transient_message
            .as_ref()
            .unwrap()
            .contains("Scheduled for"));

        // Today visible rows should NOT contain the future task
        assert!(app.visible_rows_today().is_empty());

        // Upcoming visible rows should contain it
        let upcoming_dates = app.upcoming_dates_list();
        assert_eq!(
            upcoming_dates,
            vec![NaiveDate::from_ymd_opt(2026, 10, 31).unwrap()]
        );
        let up_rows = app.visible_rows_upcoming();
        assert_eq!(up_rows.len(), 1);
        assert_eq!(up_rows[0].id, app.todos[0].id);
    }

    #[test]
    fn test_add_in_upcoming_defaults_to_selected_upcoming_date() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        let tomorrow = NaiveDate::from_ymd_opt(2026, 10, 31).unwrap();
        let mut app = App::new_for_test_with_date(
            vec![Todo {
                id: 1,
                text: "Existing future task".to_string(),
                created_date: tomorrow,
                ..Default::default()
            }],
            today,
        );

        // Switch to Upcoming
        app.current_view = View::Upcoming;
        app.clamp_selections();
        assert_eq!(app.selected_upcoming_date, Some(tomorrow));

        // Press 'a' in Upcoming
        app.handle_key(key_press(KeyCode::Char('a')));
        assert_eq!(app.input_mode, InputMode::Adding);
        assert_eq!(app.current_view, View::Upcoming);

        // Type task without date token
        app.input_buffer = "Another tomorrow task".to_string();
        app.handle_key(key_press(KeyCode::Enter));

        assert_eq!(app.input_mode, InputMode::Normal);
        let up_rows = app.visible_rows_upcoming();
        assert_eq!(up_rows.len(), 2);
        assert_eq!(app.todos[1].created_date, tomorrow);
    }

    #[test]
    fn test_reschedule_with_t_key() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        let mut app = App::new_for_test_with_date(
            vec![Todo {
                id: 1,
                text: "Buy groceries".to_string(),
                created_date: today,
                ..Default::default()
            }],
            today,
        );

        // Press 't' on selected task in Today
        app.handle_key(key_press(KeyCode::Char('t')));
        assert_eq!(app.input_mode, InputMode::Rescheduling(1));

        // 1. Invalid date expression keeps prompt open
        app.input_buffer = "invalid_date".to_string();
        app.handle_key(key_press(KeyCode::Enter));
        assert_eq!(app.input_mode, InputMode::Rescheduling(1));
        assert!(app.prompt_error.is_some());

        // 2. Past date keeps prompt open
        app.input_buffer = "2026-10-20".to_string();
        app.handle_key(key_press(KeyCode::Enter));
        assert_eq!(app.input_mode, InputMode::Rescheduling(1));
        assert!(app.prompt_error.is_some());

        // 3. Valid date expression: tomorrow
        app.input_buffer = "tomorrow".to_string();
        app.handle_key(key_press(KeyCode::Enter));
        assert_eq!(app.input_mode, InputMode::Normal);
        assert_eq!(
            app.todos[0].created_date,
            NaiveDate::from_ymd_opt(2026, 10, 31).unwrap()
        );
        assert!(app
            .transient_message
            .as_ref()
            .unwrap()
            .contains("Scheduled for"));

        // Task moved to upcoming, today is now empty
        assert!(app.visible_rows_today().is_empty());
    }

    #[test]
    fn test_cycle_priority_with_p_key() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        let mut app = App::new_for_test_with_date(
            vec![Todo {
                id: 1,
                text: "Task".to_string(),
                created_date: today,
                priority: 0,
                ..Default::default()
            }],
            today,
        );

        // Press 'p': 0 -> 1
        app.handle_key(key_press(KeyCode::Char('p')));
        assert_eq!(app.todos[0].priority, 1);

        // Press 'p': 1 -> 2
        app.handle_key(key_press(KeyCode::Char('p')));
        assert_eq!(app.todos[0].priority, 2);

        // Press 'p': 2 -> 0
        app.handle_key(key_press(KeyCode::Char('p')));
        assert_eq!(app.todos[0].priority, 0);

        // Undo restores to priority 2
        app.handle_key(key_press(KeyCode::Char('u')));
        assert_eq!(app.todos[0].priority, 2);
    }

    #[test]
    fn test_cycle_sort_and_reorder_guard() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        let mut app = App::new_for_test_with_date(
            vec![
                Todo {
                    id: 1,
                    text: "Low priority".to_string(),
                    created_date: today,
                    priority: 0,
                    ..Default::default()
                },
                Todo {
                    id: 2,
                    text: "High priority".to_string(),
                    created_date: today,
                    priority: 2,
                    ..Default::default()
                },
            ],
            today,
        );

        // In Manual sort, visible rows are in stored order [1, 2]
        assert_eq!(app.sort_mode, crate::model::SortMode::Manual);
        let rows = app.visible_rows_today();
        assert_eq!(rows[0].id, 1);
        assert_eq!(rows[1].id, 2);

        // Press 's': switch to Priority sort
        app.handle_key(key_press(KeyCode::Char('s')));
        assert_eq!(app.sort_mode, crate::model::SortMode::Priority);

        // In Priority sort, visible rows are [2, 1]
        let rows = app.visible_rows_today();
        assert_eq!(rows[0].id, 2);
        assert_eq!(rows[1].id, 1);

        // Stored order is NOT rewritten
        assert_eq!(app.todos[0].id, 1);
        assert_eq!(app.todos[1].id, 2);

        // Attempting to reorder while in Priority sort should be blocked
        app.handle_key(key_press(KeyCode::Char('J')));
        assert_eq!(
            app.transient_message.as_deref(),
            Some("Reorder works in manual sort")
        );
        // Stored order unchanged
        assert_eq!(app.todos[0].id, 1);
        assert_eq!(app.todos[1].id, 2);

        // Press 's' again: back to Manual sort
        app.handle_key(key_press(KeyCode::Char('s')));
        assert_eq!(app.sort_mode, crate::model::SortMode::Manual);
        let rows = app.visible_rows_today();
        assert_eq!(rows[0].id, 1);
        assert_eq!(rows[1].id, 2);
    }

    #[test]
    fn test_filter_with_f_key_and_esc_clears() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        let mut app = App::new_for_test_with_date(
            vec![
                Todo {
                    id: 1,
                    text: "Work task".to_string(),
                    created_date: today,
                    tags: vec!["work".to_string()],
                    ..Default::default()
                },
                Todo {
                    id: 2,
                    text: "Home task".to_string(),
                    created_date: today,
                    tags: vec!["home".to_string()],
                    ..Default::default()
                },
                Todo {
                    id: 3,
                    text: "Urgent work task".to_string(),
                    created_date: today,
                    tags: vec!["work".to_string()],
                    priority: 1,
                    ..Default::default()
                },
            ],
            today,
        );

        assert_eq!(app.visible_rows_today().len(), 3);

        // Press 'f': open filter prompt
        app.handle_key(key_press(KeyCode::Char('f')));
        assert_eq!(app.input_mode, InputMode::Filtering);

        // Filter by "#work"
        app.input_buffer = "#work".to_string();
        app.handle_key(key_press(KeyCode::Enter));
        assert_eq!(app.input_mode, InputMode::Normal);
        assert_eq!(app.today_filter.tags, vec!["work"]);

        let rows = app.visible_rows_today();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].id, 1);
        assert_eq!(rows[1].id, 3);

        // Esc in normal mode clears filter
        app.handle_key(key_press(KeyCode::Esc));
        assert!(app.today_filter.is_empty());
        assert_eq!(app.visible_rows_today().len(), 3);
    }

    #[test]
    fn test_add_task_hidden_by_filter_shows_transient() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        let mut app = App::new_for_test_with_date(Vec::new(), today);

        // Set active filter for Today
        app.today_filter = crate::parser::parse_filter("#work");

        // Add task without #work tag
        app.handle_key(key_press(KeyCode::Char('a')));
        app.input_buffer = "Buy groceries #home".to_string();
        app.handle_key(key_press(KeyCode::Enter));

        assert_eq!(app.todos.len(), 1);
        assert_eq!(
            app.transient_message.as_deref(),
            Some("Added (hidden by filter)")
        );
        assert!(app.visible_rows_today().is_empty());
    }

    #[test]
    fn test_filter_is_per_view() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        let tomorrow = NaiveDate::from_ymd_opt(2026, 10, 31).unwrap();
        let mut app = App::new_for_test_with_date(
            vec![
                Todo {
                    id: 1,
                    text: "Today work".to_string(),
                    created_date: today,
                    tags: vec!["work".to_string()],
                    ..Default::default()
                },
                Todo {
                    id: 2,
                    text: "Upcoming personal".to_string(),
                    created_date: tomorrow,
                    tags: vec!["personal".to_string()],
                    ..Default::default()
                },
            ],
            today,
        );

        // Set filter on Today
        app.today_filter = crate::parser::parse_filter("#work");
        assert_eq!(app.visible_rows_today().len(), 1);

        // Switch to Upcoming
        app.current_view = View::Upcoming;
        app.clamp_selections();
        // Upcoming filter is empty
        assert!(app.upcoming_filter.is_empty());
        assert_eq!(app.visible_rows_upcoming().len(), 1);

        // Switch back to Today
        app.current_view = View::Today;
        assert_eq!(app.today_filter.tags, vec!["work"]);
        assert_eq!(app.visible_rows_today().len(), 1);
    }

    #[test]
    fn test_note_editor_and_toggle_detail_pane() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        let mut app = App::new_for_test_with_date(
            vec![Todo {
                id: 1,
                text: "Task with note".to_string(),
                created_date: today,
                ..Default::default()
            }],
            today,
        );

        // 1. Toggle detail pane with 'i'
        assert!(!app.show_detail_pane);
        app.handle_key(key_press(KeyCode::Char('i')));
        assert!(app.show_detail_pane);
        app.handle_key(key_press(KeyCode::Char('i')));
        assert!(!app.show_detail_pane);

        // 2. Open note editor with 'n'
        app.handle_key(key_press(KeyCode::Char('n')));
        assert_eq!(app.input_mode, InputMode::EditingNote(1));

        // Type "Line 1"
        for c in "Line 1".chars() {
            app.handle_key(key_press(KeyCode::Char(c)));
        }

        // Insert newline with Alt+Enter
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::ALT));

        // Type "Line 2"
        for c in "Line 2".chars() {
            app.handle_key(key_press(KeyCode::Char(c)));
        }

        // Test cursor home/end/up/down
        app.handle_key(key_press(KeyCode::Home));
        assert_eq!(app.note_cursor, 7); // start of Line 2

        app.handle_key(key_press(KeyCode::Up));
        assert_eq!(app.note_cursor, 0); // start of Line 1

        app.handle_key(key_press(KeyCode::End));
        assert_eq!(app.note_cursor, 6); // end of Line 1

        app.handle_key(key_press(KeyCode::Down));
        assert_eq!(app.note_cursor, 13); // end of Line 2

        // Save with Enter
        app.handle_key(key_press(KeyCode::Enter));
        assert_eq!(app.input_mode, InputMode::Normal);
        assert_eq!(app.todos[0].note.as_deref(), Some("Line 1\nLine 2"));

        // Undo restores note to None
        app.handle_key(key_press(KeyCode::Char('u')));
        assert_eq!(app.todos[0].note, None);

        // Redo restores note
        app.handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL));
        assert_eq!(app.todos[0].note.as_deref(), Some("Line 1\nLine 2"));

        // Clearing note with empty input
        app.handle_key(key_press(KeyCode::Char('n')));
        app.note_buffer.clear();
        app.note_cursor = 0;
        app.handle_key(key_press(KeyCode::Enter));
        assert_eq!(app.todos[0].note, None);
    }

    #[test]
    fn test_subtask_add_prompt_and_token_rejection() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        let mut app = App::new_for_test_with_date(
            vec![Todo {
                id: 1,
                text: "Parent task".to_string(),
                created_date: today,
                ..Default::default()
            }],
            today,
        );

        // Press 'A' (Shift+A) to open add subtask prompt
        app.handle_key(KeyEvent::new(KeyCode::Char('A'), KeyModifiers::NONE));
        assert_eq!(app.input_mode, InputMode::AddingSubtask(1));

        // Subtask rejects date token
        app.input_buffer = "Subtask 1 @tomorrow".to_string();
        app.cursor_position = app.input_buffer.len();
        app.handle_key(key_press(KeyCode::Enter));
        assert_eq!(app.input_mode, InputMode::AddingSubtask(1));
        assert_eq!(
            app.prompt_error.as_deref(),
            Some("Subtasks inherit parent date; date/recurrence tokens not allowed")
        );

        // Subtask rejects recurrence token
        app.input_buffer = "Subtask 1 @daily".to_string();
        app.cursor_position = app.input_buffer.len();
        app.handle_key(key_press(KeyCode::Enter));
        assert_eq!(app.input_mode, InputMode::AddingSubtask(1));
        assert_eq!(
            app.prompt_error.as_deref(),
            Some("Subtasks inherit parent date; date/recurrence tokens not allowed")
        );

        // Valid subtask with priority and tag
        app.input_buffer = "Subtask 1 ! #work".to_string();
        app.cursor_position = app.input_buffer.len();
        app.handle_key(key_press(KeyCode::Enter));
        assert_eq!(app.input_mode, InputMode::Normal);
        assert_eq!(app.todos.len(), 2);
        let sub = &app.todos[1];
        assert_eq!(sub.text, "Subtask 1");
        assert_eq!(sub.priority, 1);
        assert_eq!(sub.tags, vec!["work"]);
        assert_eq!(sub.parent, Some(1));
        assert_eq!(sub.created_date, today);

        // When a subtask is selected, pressing 'A' adds under the parent (one level only)
        app.today_selected_index = 1; // Subtask 1 selected
        app.handle_key(KeyEvent::new(KeyCode::Char('A'), KeyModifiers::NONE));
        assert_eq!(app.input_mode, InputMode::AddingSubtask(1));
        app.input_buffer = "Subtask 2".to_string();
        app.cursor_position = app.input_buffer.len();
        app.handle_key(key_press(KeyCode::Enter));
        assert_eq!(app.todos.len(), 3);
        assert_eq!(app.todos[2].parent, Some(1));
    }

    #[test]
    fn test_subtask_completion_invariants() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        let mut app = App::new_for_test_with_date(
            vec![
                Todo {
                    id: 1,
                    text: "Parent".to_string(),
                    created_date: today,
                    done: false,
                    ..Default::default()
                },
                Todo {
                    id: 2,
                    text: "Child 1".to_string(),
                    created_date: today,
                    done: false,
                    parent: Some(1),
                    ..Default::default()
                },
                Todo {
                    id: 3,
                    text: "Child 2".to_string(),
                    created_date: today,
                    done: false,
                    parent: Some(1),
                    ..Default::default()
                },
            ],
            today,
        );

        // 1. Marking parent done completes all subtasks
        app.today_selected_index = 0; // Parent selected
        app.handle_key(key_press(KeyCode::Char(' ')));
        assert!(app.todos[0].done);
        assert!(app.todos[1].done);
        assert!(app.todos[2].done);

        // Undo reverts all
        app.handle_key(key_press(KeyCode::Char('u')));
        assert!(!app.todos[0].done);
        assert!(!app.todos[1].done);
        assert!(!app.todos[2].done);

        // 2. Completing last subtask does NOT auto-complete parent
        app.today_selected_index = 1; // Child 1
        app.handle_key(key_press(KeyCode::Char(' ')));
        assert!(app.todos[1].done);
        assert!(!app.todos[0].done);

        app.today_selected_index = 2; // Child 2
        app.handle_key(key_press(KeyCode::Char(' ')));
        assert!(app.todos[2].done);
        assert!(!app.todos[0].done); // Parent still unfinished

        // Mark parent done
        app.today_selected_index = 0;
        app.handle_key(key_press(KeyCode::Char(' ')));
        assert!(app.todos[0].done);

        // 3. Undoing a subtask reopens a done parent
        app.today_selected_index = 1; // Child 1
        app.handle_key(key_press(KeyCode::Char(' '))); // Child 1 becomes undone
        assert!(!app.todos[1].done);
        assert!(!app.todos[0].done); // Parent reopened!
    }

    #[test]
    fn test_subtask_delete_parent_deletes_subtasks() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        let mut app = App::new_for_test_with_date(
            vec![
                Todo {
                    id: 1,
                    text: "Parent".to_string(),
                    created_date: today,
                    ..Default::default()
                },
                Todo {
                    id: 2,
                    text: "Child 1".to_string(),
                    created_date: today,
                    parent: Some(1),
                    ..Default::default()
                },
                Todo {
                    id: 3,
                    text: "Standalone".to_string(),
                    created_date: today,
                    ..Default::default()
                },
            ],
            today,
        );

        // Delete parent (id 1)
        app.today_selected_index = 0;
        app.handle_key(key_press(KeyCode::Char('d')));
        assert_eq!(app.todos.len(), 1);
        assert_eq!(app.todos[0].id, 3);

        // Undo restores parent and its subtasks
        app.handle_key(key_press(KeyCode::Char('u')));
        assert_eq!(app.todos.len(), 3);
        assert_eq!(app.todos[0].id, 1);
        assert_eq!(app.todos[1].id, 2);
        assert_eq!(app.todos[2].id, 3);
    }

    #[test]
    fn test_subtask_and_parent_reorder() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        let mut app = App::new_for_test_with_date(
            vec![
                Todo {
                    id: 1,
                    text: "Parent A".to_string(),
                    created_date: today,
                    ..Default::default()
                },
                Todo {
                    id: 2,
                    text: "Child A1".to_string(),
                    created_date: today,
                    parent: Some(1),
                    ..Default::default()
                },
                Todo {
                    id: 3,
                    text: "Child A2".to_string(),
                    created_date: today,
                    parent: Some(1),
                    ..Default::default()
                },
                Todo {
                    id: 4,
                    text: "Parent B".to_string(),
                    created_date: today,
                    ..Default::default()
                },
            ],
            today,
        );

        // Reordering subtask moves among siblings only
        app.today_selected_index = 1; // Child A1
        app.handle_key(key_press(KeyCode::Char('J'))); // Move down
        assert_eq!(app.todos[1].id, 3); // Child A2 is now before A1
        assert_eq!(app.todos[2].id, 2);

        // Child A1 is at bottom of siblings; moving down should be no-op
        app.handle_key(key_press(KeyCode::Char('J')));
        assert_eq!(app.todos[1].id, 3);
        assert_eq!(app.todos[2].id, 2);

        // Reordering parent moves as a block
        app.today_selected_index = 0; // Parent A (or rows[0])
        app.today_selected_id = Some(1);
        app.handle_key(key_press(KeyCode::Char('J'))); // Move block A down
        let ids: Vec<u64> = app.todos.iter().map(|t| t.id).collect();
        assert_eq!(ids, vec![4, 1, 3, 2]); // Parent B is now before block A
    }

    #[test]
    fn test_subtask_carry_over_and_move_together() {
        let yesterday = NaiveDate::from_ymd_opt(2026, 10, 29).unwrap();
        let today = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        let mut app = App::new_for_test_with_date(
            vec![
                Todo {
                    id: 1,
                    text: "Old Parent".to_string(),
                    created_date: yesterday,
                    done: false,
                    ..Default::default()
                },
                Todo {
                    id: 2,
                    text: "Old Child".to_string(),
                    created_date: yesterday,
                    done: true,
                    parent: Some(1),
                    ..Default::default()
                },
            ],
            today,
        );

        // Carry-over triggers on unfinished parent
        let uncarried = app.uncarried_previous_tasks();
        assert!(uncarried.is_some());
        let (prev_date, ids) = uncarried.unwrap();
        assert_eq!(prev_date, yesterday);
        assert_eq!(ids, vec![1, 2]); // parent + child

        // Press 'c' to carry over
        app.handle_key(key_press(KeyCode::Char('c')));
        assert_eq!(app.todos[0].created_date, today);
        assert_eq!(app.todos[1].created_date, today);
        assert!(!app.todos[0].done);
        assert!(!app.todos[1].done); // Completion status reset
    }

    #[test]
    fn test_focus_pin_toggle_replace_clear_and_undo() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        let mut app = App::new_for_test_with_date(
            vec![
                Todo {
                    id: 1,
                    text: "Task 1".to_string(),
                    created_date: today,
                    ..Default::default()
                },
                Todo {
                    id: 2,
                    text: "Task 2".to_string(),
                    created_date: today,
                    ..Default::default()
                },
                Todo {
                    id: 3,
                    text: "Subtask of 1".to_string(),
                    created_date: today,
                    parent: Some(1),
                    ..Default::default()
                },
            ],
            today,
        );

        assert_eq!(app.focus, None);

        // 1. Focus Task 1
        app.today_selected_index = 0; // Task 1
        app.today_selected_id = Some(1);
        app.handle_key(key_press(KeyCode::Char('P')));
        assert_eq!(app.focus, Some(1));

        // 2. Pinning another task replaces previous
        // rows: [1, 3, 2] -> index 2 is Task 2
        app.today_selected_index = 2;
        app.today_selected_id = Some(2);
        app.handle_key(key_press(KeyCode::Char('P')));
        assert_eq!(app.focus, Some(2));

        // 3. Pinning same task toggles it off
        app.handle_key(key_press(KeyCode::Char('P')));
        assert_eq!(app.focus, None);

        // 4. Undo restores focus to Some(2)
        app.handle_key(key_press(KeyCode::Char('u')));
        assert_eq!(app.focus, Some(2));

        // Undo again restores focus to Some(1)
        app.handle_key(key_press(KeyCode::Char('u')));
        assert_eq!(app.focus, Some(1));

        // 5. Completing the focus task clears it
        app.today_selected_index = 0; // Task 1
        app.today_selected_id = Some(1);
        app.handle_key(key_press(KeyCode::Char(' '))); // toggle done
        assert!(app.todos[0].done);
        assert_eq!(app.focus, None);

        // Undo restores completion and focus
        app.handle_key(key_press(KeyCode::Char('u')));
        assert!(!app.todos[0].done);
        assert_eq!(app.focus, Some(1));

        // 6. Deleting the focus task clears it
        app.today_selected_index = 0; // Task 1
        app.today_selected_id = Some(1);
        app.handle_key(key_press(KeyCode::Char('d'))); // delete
        assert_eq!(app.focus, None);

        // Undo restores deletion and focus
        app.handle_key(key_press(KeyCode::Char('u')));
        assert_eq!(app.focus, Some(1));

        // 7. Focus subtask
        app.today_selected_index = 1; // Subtask 3
        app.today_selected_id = Some(3);
        app.handle_key(key_press(KeyCode::Char('P')));
        assert_eq!(app.focus, Some(3));

        // Completing parent marks subtask done -> clears focus
        app.today_selected_index = 0; // Parent 1
        app.today_selected_id = Some(1);
        app.handle_key(key_press(KeyCode::Char(' ')));
        assert!(app.todos.iter().find(|t| t.id == 3).unwrap().done);
        assert_eq!(app.focus, None);

        // Undo restores parent and subtask undone, and focus
        app.handle_key(key_press(KeyCode::Char('u')));
        assert_eq!(app.focus, Some(3));

        // Deleting parent deletes subtasks -> clears focus
        app.today_selected_index = 0;
        app.today_selected_id = Some(1);
        app.handle_key(key_press(KeyCode::Char('d')));
        assert_eq!(app.focus, None);

        // Undo restores parent, subtask, and focus
        app.handle_key(key_press(KeyCode::Char('u')));
        assert_eq!(app.focus, Some(3));
    }

    #[test]
    fn test_recurring_tasks_full_lifecycle() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 26).unwrap();
        let mut app = App::new_for_test_with_date(Vec::new(), today);

        // 1. Quick-add @daily
        app.handle_key(key_press(KeyCode::Char('a')));
        for c in "Daily habit @daily".chars() {
            app.handle_key(key_press(KeyCode::Char(c)));
        }
        app.handle_key(key_press(KeyCode::Enter));

        assert_eq!(app.todos.len(), 1);
        assert_eq!(app.recurring.len(), 1);
        let tpl_id = app.recurring[0].id;
        assert_eq!(app.todos[0].recurring_id, Some(tpl_id));
        assert_eq!(app.recurring[0].rule, crate::model::RecurrenceRule::Daily);

        // Undo removes both task and template
        app.handle_key(key_press(KeyCode::Char('u')));
        assert_eq!(app.todos.len(), 0);
        assert_eq!(app.recurring.len(), 0);

        // Redo restores both
        app.handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL));
        assert_eq!(app.todos.len(), 1);
        assert_eq!(app.recurring.len(), 1);

        // 2. Add plain task and turn it into recurring via `r`
        app.handle_key(key_press(KeyCode::Char('a')));
        for c in "Weekly report".chars() {
            app.handle_key(key_press(KeyCode::Char(c)));
        }
        app.handle_key(key_press(KeyCode::Enter));
        assert_eq!(app.todos.len(), 2);

        // Select task 2 (Weekly report)
        app.today_selected_index = 1;
        app.today_selected_id = Some(app.todos[1].id);
        app.handle_key(key_press(KeyCode::Char('r')));
        assert!(matches!(app.input_mode, InputMode::SettingRecurrence(_)));

        // Type "weekly"
        for c in "weekly".chars() {
            app.handle_key(key_press(KeyCode::Char(c)));
        }
        app.handle_key(key_press(KeyCode::Enter));
        assert_eq!(app.input_mode, InputMode::Normal);
        assert_eq!(app.recurring.len(), 2);
        let weekly_tpl = app
            .recurring
            .iter()
            .find(|r| r.rule == crate::model::RecurrenceRule::Weekly)
            .unwrap();
        assert_eq!(app.todos[1].recurring_id, Some(weekly_tpl.id));

        // 3. Subtask cannot be recurring
        app.handle_key(key_press(KeyCode::Char('A')));
        for c in "Child".chars() {
            app.handle_key(key_press(KeyCode::Char(c)));
        }
        app.handle_key(key_press(KeyCode::Enter));
        let subtask_id = app.todos.iter().find(|t| t.text == "Child").unwrap().id;

        let rows = app.visible_rows_today();
        let sub_idx = rows.iter().position(|r| r.id == subtask_id).unwrap();
        app.today_selected_index = sub_idx;
        app.today_selected_id = Some(subtask_id);
        app.handle_key(key_press(KeyCode::Char('r')));
        assert_eq!(
            app.transient_message.as_deref(),
            Some("Subtasks cannot be recurring")
        );
        assert_eq!(app.input_mode, InputMode::Normal);

        // 4. Edit instance affects only instance, not template
        app.today_selected_index = 0;
        app.today_selected_id = Some(app.todos[0].id);
        app.handle_key(key_press(KeyCode::Char('e')));
        app.input_buffer = "Modified habit instance".to_string();
        app.handle_key(key_press(KeyCode::Enter));
        assert_eq!(app.todos[0].text, "Modified habit instance");
        assert_eq!(app.recurring[0].text, "Daily habit");

        // 5. Deleting an instance prompts ConfirmDeleteRecurring
        app.handle_key(key_press(KeyCode::Char('d')));
        assert!(matches!(
            app.input_mode,
            InputMode::ConfirmDeleteRecurring(_)
        ));

        // Pressing Esc cancels
        app.handle_key(key_press(KeyCode::Esc));
        assert_eq!(app.input_mode, InputMode::Normal);
        assert_eq!(app.todos.len(), 3);
        assert_eq!(app.recurring.len(), 2);

        // Press 'd' again, choose 't' (this instance)
        app.handle_key(key_press(KeyCode::Char('d')));
        app.handle_key(key_press(KeyCode::Char('t')));
        assert_eq!(app.input_mode, InputMode::Normal);
        assert!(!app
            .todos
            .iter()
            .any(|t| t.text == "Modified habit instance"));
        assert!(app.recurring.iter().any(|r| r.id == tpl_id));

        // Undo restores instance
        app.handle_key(key_press(KeyCode::Char('u')));
        assert!(app
            .todos
            .iter()
            .any(|t| t.text == "Modified habit instance"));

        let mod_id = app
            .todos
            .iter()
            .find(|t| t.text == "Modified habit instance")
            .unwrap()
            .id;
        let rows = app.visible_rows_today();
        let mod_idx = rows.iter().position(|r| r.id == mod_id).unwrap();
        app.today_selected_index = mod_idx;
        app.today_selected_id = Some(mod_id);

        // Press 'd' again, choose 'a' (all)
        app.handle_key(key_press(KeyCode::Char('d')));
        app.handle_key(key_press(KeyCode::Char('a')));
        assert_eq!(app.input_mode, InputMode::Normal);
        assert!(!app
            .todos
            .iter()
            .any(|t| t.text == "Modified habit instance"));
        assert!(!app.recurring.iter().any(|r| r.id == tpl_id));

        // Undo restores template and task
        app.handle_key(key_press(KeyCode::Char('u')));
        assert!(app
            .todos
            .iter()
            .any(|t| t.text == "Modified habit instance"));
        assert!(app.recurring.iter().any(|r| r.id == tpl_id));

        let rows = app.visible_rows_today();
        let mod_idx = rows.iter().position(|r| r.id == mod_id).unwrap();
        app.today_selected_index = mod_idx;
        app.today_selected_id = Some(mod_id);

        // 6. Test 'off' on instance
        app.handle_key(key_press(KeyCode::Char('r')));
        for c in "off".chars() {
            app.handle_key(key_press(KeyCode::Char(c)));
        }
        app.handle_key(key_press(KeyCode::Enter));
        assert_eq!(app.transient_message.as_deref(), Some("Recurrence off"));
        assert!(!app.recurring.iter().any(|r| r.id == tpl_id));
        let restored_todo = app
            .todos
            .iter()
            .find(|t| t.text == "Modified habit instance")
            .unwrap();
        assert_eq!(restored_todo.recurring_id, None);
    }

    #[test]
    fn test_multiselect_and_bulk_actions() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        let tomorrow = NaiveDate::from_ymd_opt(2026, 10, 31).unwrap();
        let mut app = App::new_for_test_with_date(
            vec![
                Todo {
                    id: 1,
                    text: "Task 1".to_string(),
                    created_date: today,
                    done: false,
                    ..Default::default()
                },
                Todo {
                    id: 2,
                    text: "Task 2".to_string(),
                    created_date: today,
                    done: false,
                    ..Default::default()
                },
                Todo {
                    id: 3,
                    text: "Task 3".to_string(),
                    created_date: today,
                    done: false,
                    ..Default::default()
                },
            ],
            today,
        );

        // 1. `v` toggles mark on selected row
        app.today_selected_index = 0;
        app.today_selected_id = Some(1);
        app.handle_key(key_press(KeyCode::Char('v')));
        assert_eq!(app.marks, [1].into_iter().collect());

        app.today_selected_index = 1;
        app.today_selected_id = Some(2);
        app.handle_key(key_press(KeyCode::Char('v')));
        assert_eq!(app.marks, [1, 2].into_iter().collect());

        // Toggle task 1 off with `v`
        app.today_selected_index = 0;
        app.today_selected_id = Some(1);
        app.handle_key(key_press(KeyCode::Char('v')));
        assert_eq!(app.marks, [2].into_iter().collect());

        // 2. `V` marks all visible rows, or unmarks all if all are already marked
        app.handle_key(key_press(KeyCode::Char('V')));
        assert_eq!(app.marks, [1, 2, 3].into_iter().collect());

        app.handle_key(key_press(KeyCode::Char('V')));
        assert!(app.marks.is_empty());

        // 3. `Esc` clears marks first, then filter
        app.marks.insert(1);
        app.today_filter = crate::parser::parse_filter("#tag");
        app.handle_key(key_press(KeyCode::Esc));
        assert!(app.marks.is_empty());
        assert!(!app.today_filter.is_empty());

        app.handle_key(key_press(KeyCode::Esc));
        assert!(app.today_filter.is_empty());

        // 4. Bulk toggle (space)
        app.marks.insert(1);
        app.marks.insert(2);
        app.handle_key(key_press(KeyCode::Char(' ')));
        assert!(app.todos.iter().find(|t| t.id == 1).unwrap().done);
        assert!(app.todos.iter().find(|t| t.id == 2).unwrap().done);
        assert!(!app.todos.iter().find(|t| t.id == 3).unwrap().done);
        assert!(app.marks.is_empty());

        // Undo restores both undone in one step
        app.handle_key(key_press(KeyCode::Char('u')));
        assert!(!app.todos.iter().find(|t| t.id == 1).unwrap().done);
        assert!(!app.todos.iter().find(|t| t.id == 2).unwrap().done);

        // When all marked are done -> toggles all to undone
        app.todos[0].done = true;
        app.todos[1].done = true;
        app.marks.insert(1);
        app.marks.insert(2);
        app.handle_key(key_press(KeyCode::Char(' ')));
        assert!(!app.todos.iter().find(|t| t.id == 1).unwrap().done);
        assert!(!app.todos.iter().find(|t| t.id == 2).unwrap().done);
        assert!(app.marks.is_empty());

        // 5. Bulk reschedule (`t`)
        app.marks.insert(1);
        app.marks.insert(2);
        app.handle_key(key_press(KeyCode::Char('t')));
        assert_eq!(app.input_mode, InputMode::ReschedulingBulk);

        // Past date rejected
        for c in "2026-10-01".chars() {
            app.handle_key(key_press(KeyCode::Char(c)));
        }
        app.handle_key(key_press(KeyCode::Enter));
        assert!(app
            .prompt_error
            .as_deref()
            .unwrap()
            .contains("cannot be in the past"));
        assert_eq!(app.input_mode, InputMode::ReschedulingBulk);

        // Cancel with Esc leaves marks intact
        app.handle_key(key_press(KeyCode::Esc));
        assert_eq!(app.input_mode, InputMode::Normal);
        assert_eq!(app.marks, [1, 2].into_iter().collect());

        // Valid future date
        app.handle_key(key_press(KeyCode::Char('t')));
        for c in "tomorrow".chars() {
            app.handle_key(key_press(KeyCode::Char(c)));
        }
        app.handle_key(key_press(KeyCode::Enter));
        assert_eq!(app.input_mode, InputMode::Normal);
        assert!(app.marks.is_empty());
        assert_eq!(
            app.todos.iter().find(|t| t.id == 1).unwrap().created_date,
            tomorrow
        );
        assert_eq!(
            app.todos.iter().find(|t| t.id == 2).unwrap().created_date,
            tomorrow
        );
        assert_eq!(
            app.todos.iter().find(|t| t.id == 3).unwrap().created_date,
            today
        );

        // Undo restores dates in one step
        app.handle_key(key_press(KeyCode::Char('u')));
        assert_eq!(
            app.todos.iter().find(|t| t.id == 1).unwrap().created_date,
            today
        );
        assert_eq!(
            app.todos.iter().find(|t| t.id == 2).unwrap().created_date,
            today
        );

        // 6. Bulk move to Today (`m`)
        app.todos[0].created_date = tomorrow;
        app.todos[1].created_date = tomorrow;
        app.marks.insert(1);
        app.marks.insert(2);
        app.handle_key(key_press(KeyCode::Char('m')));
        assert_eq!(
            app.todos.iter().find(|t| t.id == 1).unwrap().created_date,
            today
        );
        assert_eq!(
            app.todos.iter().find(|t| t.id == 2).unwrap().created_date,
            today
        );
        assert!(app.marks.is_empty());

        // Undo restores dates in one step
        app.handle_key(key_press(KeyCode::Char('u')));
        assert_eq!(
            app.todos.iter().find(|t| t.id == 1).unwrap().created_date,
            tomorrow
        );
        assert_eq!(
            app.todos.iter().find(|t| t.id == 2).unwrap().created_date,
            tomorrow
        );

        // 7. Bulk delete (`d`)
        app.marks.insert(1);
        app.marks.insert(2);
        app.handle_key(key_press(KeyCode::Char('d')));
        assert_eq!(app.todos.len(), 1);
        assert_eq!(app.todos[0].id, 3);
        assert!(app.marks.is_empty());

        // Undo restores deleted tasks in one step
        app.handle_key(key_press(KeyCode::Char('u')));
        assert_eq!(app.todos.len(), 3);

        // 8. View switch clears marks
        app.marks.insert(3);
        app.handle_key(key_press(KeyCode::Tab));
        assert!(app.marks.is_empty());
    }
}
