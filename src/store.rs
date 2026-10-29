use chrono::{DateTime, Local, NaiveDate};

use crate::model::Todo;

/// Returns all tasks created on `date`, preserving their order in `todos`.
pub fn tasks_for_date(todos: &[Todo], date: NaiveDate) -> Vec<&Todo> {
    todos.iter().filter(|t| t.created_date == date).collect()
}

/// Generates the next sequential ID for a task.
pub fn next_task_id(todos: &[Todo]) -> u64 {
    todos.iter().map(|t| t.id).max().unwrap_or(0) + 1
}

/// Adds a new task for the given date. Returns the new task's ID if added.
pub fn add_task(todos: &mut Vec<Todo>, text: &str, date: NaiveDate) -> Option<u64> {
    let clean_text = text.trim();
    if clean_text.is_empty() {
        return None;
    }

    let id = next_task_id(todos);
    todos.push(Todo {
        id,
        text: clean_text.to_string(),
        created_date: date,
        done: false,
        done_at: None,
    });
    Some(id)
}

/// Sets the completion state of a task by ID.
pub fn set_task_done(todos: &mut [Todo], id: u64, done: bool, now: DateTime<Local>) -> bool {
    if let Some(todo) = todos.iter_mut().find(|t| t.id == id) {
        todo.done = done;
        todo.done_at = if done { Some(now) } else { None };
        true
    } else {
        false
    }
}

/// Toggles the completion state of a task by ID. Returns the new `done` state if found.
pub fn toggle_task(todos: &mut [Todo], id: u64, now: DateTime<Local>) -> Option<bool> {
    if let Some(todo) = todos.iter_mut().find(|t| t.id == id) {
        todo.done = !todo.done;
        todo.done_at = if todo.done { Some(now) } else { None };
        Some(todo.done)
    } else {
        None
    }
}

/// Deletes a task by ID. Returns the removed task if it existed.
pub fn delete_task(todos: &mut Vec<Todo>, id: u64) -> Option<Todo> {
    todos
        .iter()
        .position(|t| t.id == id)
        .map(|pos| todos.remove(pos))
}

/// Updates the text of a task by ID.
pub fn edit_task(todos: &mut [Todo], id: u64, new_text: &str) -> bool {
    let clean_text = new_text.trim();
    if clean_text.is_empty() {
        return false;
    }
    if let Some(todo) = todos.iter_mut().find(|t| t.id == id) {
        todo.text = clean_text.to_string();
        true
    } else {
        false
    }
}

/// Clears all completed tasks for a specific date. Returns the count of removed tasks.
pub fn clear_completed(todos: &mut Vec<Todo>, date: NaiveDate) -> usize {
    let before = todos.len();
    todos.retain(|t| !(t.created_date == date && t.done));
    before - todos.len()
}

/// Moves a task to a target date, resetting its completion status.
pub fn move_task_to_date(todos: &mut Vec<Todo>, id: u64, target_date: NaiveDate) -> bool {
    if let Some(pos) = todos.iter().position(|t| t.id == id) {
        let mut todo = todos.remove(pos);
        todo.created_date = target_date;
        todo.done = false;
        todo.done_at = None;
        todos.push(todo);
        true
    } else {
        false
    }
}

/// Carries over specified task IDs to a new date, preserving their text and resetting completion.
pub fn carry_over_tasks(todos: &mut Vec<Todo>, task_ids: &[u64], target_date: NaiveDate) -> usize {
    let mut count = 0;
    let mut carried = Vec::new();
    let mut i = 0;
    while i < todos.len() {
        if task_ids.contains(&todos[i].id) {
            let mut t = todos.remove(i);
            t.created_date = target_date;
            carried.push(t);
            count += 1;
        } else {
            i += 1;
        }
    }
    todos.extend(carried);
    count
}

/// Reorders a task within its day list (down = true moves towards higher index, down = false moves towards 0).
pub fn reorder_day_task(
    todos: &mut [Todo],
    date: NaiveDate,
    selected_index: usize,
    down: bool,
) -> bool {
    let day_indices: Vec<usize> = todos
        .iter()
        .enumerate()
        .filter(|(_, t)| t.created_date == date)
        .map(|(i, _)| i)
        .collect();

    if day_indices.len() <= 1 {
        return false;
    }

    if down {
        if selected_index + 1 < day_indices.len() {
            let pos_curr = day_indices[selected_index];
            let pos_next = day_indices[selected_index + 1];
            todos.swap(pos_curr, pos_next);
            return true;
        }
    } else if selected_index > 0 {
        let pos_curr = day_indices[selected_index];
        let pos_prev = day_indices[selected_index - 1];
        todos.swap(pos_curr, pos_prev);
        return true;
    }
    false
}

/// Checks whether a task at `selected_index` within `date` can be moved up or down.
pub fn can_reorder_day_task(
    todos: &[Todo],
    date: NaiveDate,
    selected_index: usize,
    down: bool,
) -> bool {
    let day_count = todos.iter().filter(|t| t.created_date == date).count();
    if day_count <= 1 {
        return false;
    }
    if down {
        selected_index + 1 < day_count
    } else {
        selected_index > 0 && selected_index < day_count
    }
}

/// Resolves a 1-based index against the tasks for `date`.
pub fn get_task_by_1based_index(
    todos: &[Todo],
    date: NaiveDate,
    index_1based: usize,
) -> Option<&Todo> {
    if index_1based == 0 {
        return None;
    }
    tasks_for_date(todos, date).get(index_1based - 1).copied()
}

/// Resolves a case-insensitive substring match against the tasks for `date`.
/// Returns:
/// - `Ok(Some(&Todo))` if exactly one task matches.
/// - `Ok(None)` if no task matches.
/// - `Err(candidates)` if multiple tasks match, where candidates are `(1-based-index, task_text)`.
pub fn match_task_by_text<'a>(
    todos: &'a [Todo],
    date: NaiveDate,
    query: &str,
) -> Result<Option<&'a Todo>, Vec<(usize, String)>> {
    let day_tasks = tasks_for_date(todos, date);
    let q = query.to_lowercase();

    let mut matches = Vec::new();
    for (idx, task) in day_tasks.iter().enumerate() {
        if task.text.to_lowercase().contains(&q) {
            matches.push((idx + 1, task));
        }
    }

    match matches.len() {
        0 => Ok(None),
        1 => Ok(Some(matches[0].1)),
        _ => Err(matches
            .into_iter()
            .map(|(i, t)| (i, t.text.clone()))
            .collect()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_query_tasks() {
        let mut todos = Vec::new();
        let date = NaiveDate::from_ymd_opt(2026, 10, 27).unwrap();

        assert_eq!(add_task(&mut todos, "", date), None);
        assert_eq!(add_task(&mut todos, "   ", date), None);

        let id1 = add_task(&mut todos, "First task", date).unwrap();
        assert_eq!(id1, 1);
        let id2 = add_task(&mut todos, "Second task", date).unwrap();
        assert_eq!(id2, 2);

        let day_tasks = tasks_for_date(&todos, date);
        assert_eq!(day_tasks.len(), 2);
        assert_eq!(day_tasks[0].text, "First task");
        assert_eq!(day_tasks[1].text, "Second task");
    }

    #[test]
    fn test_toggle_and_set_done() {
        let mut todos = Vec::new();
        let date = NaiveDate::from_ymd_opt(2026, 10, 27).unwrap();
        let now = Local::now();

        let id = add_task(&mut todos, "Task", date).unwrap();
        assert!(!todos[0].done);

        assert_eq!(toggle_task(&mut todos, id, now), Some(true));
        assert!(todos[0].done);
        assert!(todos[0].done_at.is_some());

        assert_eq!(toggle_task(&mut todos, id, now), Some(false));
        assert!(!todos[0].done);
        assert!(todos[0].done_at.is_none());

        assert!(set_task_done(&mut todos, id, true, now));
        assert!(todos[0].done);

        assert!(!set_task_done(&mut todos, 999, true, now));
    }

    #[test]
    fn test_delete_and_edit() {
        let mut todos = Vec::new();
        let date = NaiveDate::from_ymd_opt(2026, 10, 27).unwrap();

        let id = add_task(&mut todos, "Task to edit", date).unwrap();
        assert!(edit_task(&mut todos, id, "Updated text"));
        assert_eq!(todos[0].text, "Updated text");
        assert!(!edit_task(&mut todos, id, "  ")); // Empty edit rejected

        let deleted = delete_task(&mut todos, id);
        assert!(deleted.is_some());
        assert_eq!(deleted.unwrap().text, "Updated text");
        assert!(todos.is_empty());
    }

    #[test]
    fn test_clear_completed() {
        let mut todos = Vec::new();
        let date = NaiveDate::from_ymd_opt(2026, 10, 27).unwrap();
        let now = Local::now();

        let id1 = add_task(&mut todos, "Task 1", date).unwrap();
        let _id2 = add_task(&mut todos, "Task 2", date).unwrap();
        set_task_done(&mut todos, id1, true, now);

        let removed = clear_completed(&mut todos, date);
        assert_eq!(removed, 1);
        assert_eq!(todos.len(), 1);
        assert_eq!(todos[0].text, "Task 2");
    }

    #[test]
    fn test_1based_index_and_matching() {
        let mut todos = Vec::new();
        let date = NaiveDate::from_ymd_opt(2026, 10, 27).unwrap();

        add_task(&mut todos, "Buy milk", date);
        add_task(&mut todos, "Buy fresh bread", date);
        add_task(&mut todos, "Call doctor", date);

        assert_eq!(get_task_by_1based_index(&todos, date, 0), None);
        assert_eq!(
            get_task_by_1based_index(&todos, date, 1).unwrap().text,
            "Buy milk"
        );
        assert_eq!(
            get_task_by_1based_index(&todos, date, 3).unwrap().text,
            "Call doctor"
        );
        assert_eq!(get_task_by_1based_index(&todos, date, 4), None);

        // Unique match
        let m = match_task_by_text(&todos, date, "milk").unwrap();
        assert_eq!(m.unwrap().text, "Buy milk");

        // No match
        let none = match_task_by_text(&todos, date, "cookies").unwrap();
        assert!(none.is_none());

        // Ambiguous match
        let err = match_task_by_text(&todos, date, "buy").unwrap_err();
        assert_eq!(err.len(), 2);
        assert_eq!(err[0], (1, "Buy milk".to_string()));
        assert_eq!(err[1], (2, "Buy fresh bread".to_string()));
    }
}
