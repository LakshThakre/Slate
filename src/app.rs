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
    pub undo_stack: Vec<Vec<Todo>>,
    pub redo_stack: Vec<Vec<Todo>>,
    pub transient_message: Option<String>,
    pub auto_save: bool,
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
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            transient_message: None,
            auto_save: true,
        };

        app.clamp_selections();
        app
    }

    #[cfg(test)]
    pub fn new_for_test(todos: Vec<Todo>) -> Self {
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
            status_message: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            transient_message: None,
            auto_save: false,
        };
        app.clamp_selections();
        app
    }

    pub fn save(&mut self) {
        if !self.auto_save {
            return;
        }
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
        let today = Local::now().date_naive();
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
        let today = Local::now().date_naive();
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
            let today = Local::now().date_naive();
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
        let today = Local::now().date_naive();
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
        let today = Local::now().date_naive();
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

    /// Handles a key event. Returns true if the app should exit.
    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        self.transient_message = None;
        match self.input_mode {
            InputMode::Adding => self.handle_adding_key(key),
            InputMode::Editing(id) => self.handle_editing_key(id, key),
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
        if key
            .modifiers
            .contains(ratatui::crossterm::event::KeyModifiers::CONTROL)
        {
            match key.code {
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    self.redo();
                    return false;
                }
                _ => {}
            }
        }

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
            KeyCode::Char('u') => {
                self.undo();
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
            KeyCode::Char('e') => {
                self.start_editing_today();
            }
            KeyCode::Char('c') => {
                self.carry_over();
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
                    KeyCode::Char('e') => {
                        self.start_editing_history();
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
}
