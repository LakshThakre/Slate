use chrono::{DateTime, Local, NaiveDate};

use crate::model::{Filter, Row, SortMode, Todo};

/// Returns all tasks created on `date`, preserving their order in `todos`.
pub fn tasks_for_date(todos: &[Todo], date: NaiveDate) -> Vec<&Todo> {
    todos.iter().filter(|t| t.created_date == date).collect()
}

/// Computes the visible projected rows for a given date, filter, and sort mode.
pub fn visible_rows(todos: &[Todo], date: NaiveDate, filter: &Filter, sort: SortMode) -> Vec<Row> {
    let day_tasks: Vec<&Todo> = todos.iter().filter(|t| t.created_date == date).collect();

    let mut top_level: Vec<&Todo> = Vec::new();
    let mut subtasks_map: std::collections::HashMap<u64, Vec<&Todo>> =
        std::collections::HashMap::new();

    for &task in &day_tasks {
        if let Some(parent_id) = task.parent {
            subtasks_map.entry(parent_id).or_default().push(task);
        } else {
            top_level.push(task);
        }
    }

    let mut filtered_top: Vec<&Todo> = Vec::new();
    for &parent in &top_level {
        let parent_matches = filter.matches(parent);
        let subtasks = subtasks_map.get(&parent.id);
        let any_subtask_matches =
            subtasks.is_some_and(|subs| subs.iter().any(|s| filter.matches(s)));

        if filter.is_empty() || parent_matches || any_subtask_matches {
            filtered_top.push(parent);
        }
    }

    match sort {
        SortMode::Manual => {}
        SortMode::Priority => {
            filtered_top.sort_by_key(|b| std::cmp::Reverse(b.priority));
        }
    }

    let mut rows = Vec::new();
    for parent in filtered_top {
        rows.push(Row {
            id: parent.id,
            depth: 0,
        });

        if let Some(subtasks) = subtasks_map.get(&parent.id) {
            let parent_matches = filter.matches(parent);
            for &subtask in subtasks {
                if filter.is_empty() || parent_matches || filter.matches(subtask) {
                    rows.push(Row {
                        id: subtask.id,
                        depth: 1,
                    });
                }
            }
        }
    }

    rows
}

/// Generates the next sequential ID for a task based on current slice.
pub fn next_task_id(todos: &[Todo]) -> u64 {
    todos.iter().map(|t| t.id).max().unwrap_or(0) + 1
}

/// Adds a new task for the given date with a specific ID.
pub fn add_task_with_id(
    todos: &mut Vec<Todo>,
    id: u64,
    text: &str,
    date: NaiveDate,
) -> Option<u64> {
    let clean_text = text.trim();
    if clean_text.is_empty() {
        return None;
    }

    todos.push(Todo {
        id,
        text: clean_text.to_string(),
        created_date: date,
        done: false,
        done_at: None,
        priority: 0,
        tags: Vec::new(),
        note: None,
        parent: None,
        recurring_id: None,
    });
    Some(id)
}

/// Adds a new task for the given date. Returns the new task's ID if added.
pub fn add_task(todos: &mut Vec<Todo>, text: &str, date: NaiveDate) -> Option<u64> {
    let clean_text = text.trim();
    if clean_text.is_empty() {
        return None;
    }

    let id = next_task_id(todos);
    add_task_with_id(todos, id, clean_text, date)
}

/// Sets the completion state of a task by ID.
pub fn set_task_done(todos: &mut [Todo], id: u64, done: bool, now: DateTime<Local>) -> bool {
    if let Some(todo) = todos.iter_mut().find(|t| t.id == id) {
        if done {
            if !todo.done {
                todo.done = true;
                todo.done_at = Some(now);
            }
        } else {
            todo.done = false;
            todo.done_at = None;
        }
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

/// Moves a task and its subtasks to a target date, resetting completion status.
pub fn move_task_to_date(todos: &mut [Todo], id: u64, target_date: NaiveDate) -> bool {
    let root_id = if let Some(t) = todos.iter().find(|t| t.id == id) {
        t.parent.unwrap_or(id)
    } else {
        return false;
    };

    let ids_to_move: Vec<u64> = todos
        .iter()
        .filter(|t| t.id == root_id || t.parent == Some(root_id))
        .map(|t| t.id)
        .collect();

    if ids_to_move.is_empty() {
        return false;
    }

    for todo in todos.iter_mut() {
        if ids_to_move.contains(&todo.id) {
            todo.created_date = target_date;
            todo.done = false;
            todo.done_at = None;
        }
    }
    true
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
        assert!(!edit_task(&mut todos, id, "  "));

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

        let m = match_task_by_text(&todos, date, "milk").unwrap();
        assert_eq!(m.unwrap().text, "Buy milk");

        let none = match_task_by_text(&todos, date, "cookies").unwrap();
        assert!(none.is_none());

        let err = match_task_by_text(&todos, date, "buy").unwrap_err();
        assert_eq!(err.len(), 2);
        assert_eq!(err[0], (1, "Buy milk".to_string()));
        assert_eq!(err[1], (2, "Buy fresh bread".to_string()));
    }

    #[test]
    fn test_visible_rows_projection() {
        let date = NaiveDate::from_ymd_opt(2026, 10, 27).unwrap();
        let todos = vec![
            Todo {
                id: 1,
                text: "Top 1".to_string(),
                created_date: date,
                done: false,
                priority: 0,
                ..Default::default()
            },
            Todo {
                id: 2,
                text: "Sub 1.1".to_string(),
                created_date: date,
                done: false,
                parent: Some(1),
                ..Default::default()
            },
            Todo {
                id: 3,
                text: "Top 2".to_string(),
                created_date: date,
                done: false,
                priority: 2,
                ..Default::default()
            },
        ];

        // Manual sort: Top 1 (with sub 1.1), then Top 2
        let rows_manual = visible_rows(&todos, date, &Filter::default(), SortMode::Manual);
        assert_eq!(
            rows_manual,
            vec![
                Row { id: 1, depth: 0 },
                Row { id: 2, depth: 1 },
                Row { id: 3, depth: 0 },
            ]
        );

        // Priority sort: Top 2 (prio 2), then Top 1 (prio 0 with sub 1.1)
        let rows_priority = visible_rows(&todos, date, &Filter::default(), SortMode::Priority);
        assert_eq!(
            rows_priority,
            vec![
                Row { id: 3, depth: 0 },
                Row { id: 1, depth: 0 },
                Row { id: 2, depth: 1 },
            ]
        );
    }
}
