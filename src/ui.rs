use chrono::{Local, NaiveDate};
use ratatui::layout::{Constraint, Direction, Layout, Position, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, ListState, Paragraph};
use ratatui::Frame;

use crate::app::App;
use crate::model::{HistoryPane, Todo, View};
use crate::theme::THEME;

pub fn render(app: &mut App, frame: &mut Frame) {
    app.clamp_selections();

    let terminal_area = frame.area();
    if terminal_area.width < 24 || terminal_area.height < 6 {
        let msg = Paragraph::new("Terminal too small").style(Style::default().fg(THEME.dim));
        frame.render_widget(msg, terminal_area);
        return;
    }

    // Add 1 or 2 columns of breathable horizontal padding
    let h_pad = if terminal_area.width > 75 { 2 } else { 1 };
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(h_pad),
            Constraint::Min(0),
            Constraint::Length(h_pad),
        ])
        .split(terminal_area);

    let content_area = cols[1];

    // Calm, unboxed vertical layout:
    // [Header (1)] -> [Divider (1)] -> [Spacing (1)] -> [Main (Min 0)] -> [Divider/Prompt or Spacing (1)] -> [Footer / Input (1)]
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Slim header line
            Constraint::Length(1), // Thin divider rule
            Constraint::Length(1), // Blank line spacing
            Constraint::Min(0),    // Main content (Today or History)
            Constraint::Length(1), // Divider line above prompt (or blank spacing)
            Constraint::Length(1), // Footer key hints or inline input prompt
        ])
        .split(content_area);

    let header_area = rows[0];
    let header_divider_area = rows[1];
    let main_area = rows[3];
    let prompt_divider_area = rows[4];
    let bottom_area = rows[5];

    // 1. Draw Slim Header & Rule
    draw_header(app, frame, header_area);
    frame.render_widget(
        Paragraph::new("─".repeat(header_divider_area.width as usize))
            .style(Style::default().fg(THEME.divider)),
        header_divider_area,
    );

    // 2. Draw Main View (Today or History)
    match app.current_view {
        View::Today => draw_today(app, frame, main_area),
        View::History => draw_history(app, frame, main_area),
    }

    // 3. Draw Bottom Area (Inline prompt when adding or editing, otherwise subtle key hints)
    if matches!(
        app.input_mode,
        crate::model::InputMode::Adding | crate::model::InputMode::Editing(_)
    ) {
        frame.render_widget(
            Paragraph::new("─".repeat(prompt_divider_area.width as usize))
                .style(Style::default().fg(THEME.divider)),
            prompt_divider_area,
        );
        draw_input(app, frame, bottom_area);
    } else {
        draw_footer(app, frame, bottom_area);
    }
}

/// Slim top header line: App name + plain text view tabs
fn draw_header(app: &App, frame: &mut Frame, area: Rect) {
    let today_tab = if app.current_view == View::Today {
        Span::styled(
            "Today",
            Style::default()
                .fg(THEME.accent)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )
    } else {
        Span::styled("Today", Style::default().fg(THEME.dim))
    };

    let history_tab = if app.current_view == View::History {
        Span::styled(
            "History",
            Style::default()
                .fg(THEME.accent)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )
    } else {
        Span::styled("History", Style::default().fg(THEME.dim))
    };

    let line = Line::from(vec![
        Span::styled(
            "Slate",
            Style::default()
                .fg(THEME.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("    "),
        today_tab,
        Span::raw("   "),
        history_tab,
    ]);

    frame.render_widget(Paragraph::new(line), area);
}

/// Quiet Today view: Title row with date, count, slim block progress bar, and todo list
fn draw_today(app: &App, frame: &mut Frame, area: Rect) {
    let today = Local::now().date_naive();
    let today_todos: Vec<&Todo> = app
        .todos
        .iter()
        .filter(|t| t.created_date == today)
        .collect();
    let total = today_todos.len();
    let done = today_todos.iter().filter(|t| t.done).count();

    let uncarried = app.uncarried_previous_tasks();
    let has_uncarried = uncarried.is_some();

    let parts = if has_uncarried {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Title row
                Constraint::Length(1), // Carry over hint line
                Constraint::Length(1), // Blank line
                Constraint::Min(0),    // Todos list
            ])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Title row
                Constraint::Length(1), // Blank line
                Constraint::Min(0),    // Todos list
            ])
            .split(area)
    };

    // Friendly title row: "Friday, 23 October   3 of 5 done  ████░░░░░░"
    let date_str = today.format("%A, %-d %B").to_string();
    let bar_width = if area.width > 70 { 10 } else { 6 };
    let mut title_spans = vec![
        Span::styled(
            date_str,
            Style::default().fg(THEME.text).add_modifier(Modifier::BOLD),
        ),
        Span::raw("    "),
        Span::styled(
            format!("{done} of {total} done"),
            Style::default().fg(THEME.dim),
        ),
    ];

    if total > 0 {
        title_spans.push(Span::raw("  "));
        title_spans.extend(render_progress_blocks(done, total, bar_width));
    }

    frame.render_widget(Paragraph::new(Line::from(title_spans)), parts[0]);

    if let Some((prev_date, ref ids)) = uncarried {
        let count = ids.len();
        let date_label = if prev_date == today.pred_opt().unwrap_or(today) {
            "yesterday".to_string()
        } else {
            format_friendly_date(prev_date, today)
        };
        let hint_line = Line::from(vec![
            Span::styled(
                format!("{count} unfinished from {date_label}  ·  "),
                Style::default().fg(THEME.dim),
            ),
            Span::styled(
                "c",
                Style::default()
                    .fg(THEME.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" to carry over", Style::default().fg(THEME.dim)),
        ]);
        frame.render_widget(Paragraph::new(hint_line), parts[1]);
    }

    let list_area = if has_uncarried { parts[3] } else { parts[2] };

    // Empty state or task list
    if today_todos.is_empty() {
        let empty_text = Line::from(vec![
            Span::styled("No todos yet. Press ", Style::default().fg(THEME.dim)),
            Span::styled(
                "a",
                Style::default()
                    .fg(THEME.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" to add your first one.", Style::default().fg(THEME.dim)),
        ]);
        frame.render_widget(Paragraph::new(empty_text), list_area);
        return;
    }

    let items: Vec<ListItem> = today_todos
        .iter()
        .enumerate()
        .map(|(idx, todo)| {
            let is_selected = idx == app.today_selected_index;
            format_todo_row(todo, is_selected, list_area.width as usize)
        })
        .collect();

    let list = List::new(items);
    let mut list_state = ListState::default();
    list_state.select(Some(app.today_selected_index));
    frame.render_stateful_widget(list, list_area, &mut list_state);
}

/// History view: Narrow Date column + thin vertical rule + Todo list for selected date
fn draw_history(app: &App, frame: &mut Frame, area: Rect) {
    let dates = app.dates_list();
    let today = Local::now().date_naive();

    // Responsive date column width: 18 cols on narrow screens, 24 on wide screens
    let date_col_width = if area.width < 70 { 18 } else { 24 }.min(area.width.saturating_sub(15));

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(date_col_width),
            Constraint::Length(1), // Vertical divider
            Constraint::Length(2), // Padding
            Constraint::Min(0),    // Right todo pane
        ])
        .split(area);

    let date_area = cols[0];
    let divider_area = cols[1];
    let todo_area = cols[3];

    let is_date_pane_active = app.history_pane == HistoryPane::Dates;
    let is_todo_pane_active = app.history_pane == HistoryPane::Todos;

    // 1. Draw Left Date Column
    let date_items: Vec<ListItem> = dates
        .iter()
        .enumerate()
        .map(|(idx, &date)| {
            let is_selected = idx == app.history_date_index;
            let date_todos: Vec<&Todo> = app
                .todos
                .iter()
                .filter(|t| t.created_date == date)
                .collect();
            let total = date_todos.len();
            let done = date_todos.iter().filter(|t| t.done).count();

            let marker = if is_selected {
                if is_date_pane_active {
                    Span::styled(
                        "› ",
                        Style::default()
                            .fg(THEME.accent)
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    Span::styled("› ", Style::default().fg(THEME.dim))
                }
            } else {
                Span::raw("  ")
            };

            let friendly_date = format_friendly_date(date, today);
            let count_str = format!("{done}/{total}");

            // Calculate spacing so count aligns to the right edge of the date column
            let text_space = (date_area.width as usize).saturating_sub(2 + count_str.len());
            let date_truncated = truncate_text(&friendly_date, text_space.saturating_sub(1));
            let pad_spaces = text_space.saturating_sub(date_truncated.chars().count());

            let date_style = if is_selected && is_date_pane_active {
                Style::default().fg(THEME.text).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(THEME.text)
            };

            let line = Line::from(vec![
                marker,
                Span::styled(date_truncated, date_style),
                Span::raw(" ".repeat(pad_spaces)),
                Span::styled(count_str, Style::default().fg(THEME.dim)),
            ]);

            ListItem::new(line)
        })
        .collect();

    let date_list = List::new(date_items);
    let mut date_state = ListState::default();
    if !dates.is_empty() {
        date_state.select(Some(app.history_date_index));
    }
    frame.render_stateful_widget(date_list, date_area, &mut date_state);

    // 2. Draw Thin Vertical Divider
    let divider_line = "│\n".repeat(area.height as usize);
    frame.render_widget(
        Paragraph::new(divider_line).style(Style::default().fg(THEME.divider)),
        divider_area,
    );

    // 3. Draw Right Todo Pane
    let selected_date = dates.get(app.history_date_index).copied();
    if let Some(date) = selected_date {
        let date_todos: Vec<&Todo> = app
            .todos
            .iter()
            .filter(|t| t.created_date == date)
            .collect();
        let total = date_todos.len();
        let done = date_todos.iter().filter(|t| t.done).count();

        let right_rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Heading row
                Constraint::Length(1), // Blank line spacing
                Constraint::Min(0),    // Todos list
            ])
            .split(todo_area);

        // Heading: "Monday, 19 October  •  3 of 5 done"
        let full_date_str = date.format("%A, %-d %B").to_string();
        let heading_line = Line::from(vec![
            Span::styled(
                full_date_str,
                Style::default().fg(THEME.text).add_modifier(Modifier::BOLD),
            ),
            Span::styled("   •   ", Style::default().fg(THEME.dim)),
            Span::styled(
                format!("{done} of {total} done"),
                Style::default().fg(THEME.dim),
            ),
        ]);
        frame.render_widget(Paragraph::new(heading_line), right_rows[0]);

        if date_todos.is_empty() {
            frame.render_widget(
                Paragraph::new(Span::styled(
                    "No todos recorded for this date.",
                    Style::default().fg(THEME.dim),
                )),
                right_rows[2],
            );
        } else {
            let items: Vec<ListItem> = date_todos
                .iter()
                .enumerate()
                .map(|(idx, todo)| {
                    let is_selected = idx == app.history_todo_index;
                    let active_marker = is_selected && is_todo_pane_active;
                    let time_str = if todo.done {
                        match todo.done_at {
                            Some(dt) => Some(format!("done {}", dt.format("%H:%M"))),
                            None => Some("done".to_string()),
                        }
                    } else {
                        None
                    };

                    format_todo_row_history(
                        todo,
                        is_selected,
                        active_marker,
                        right_rows[2].width as usize,
                        time_str.as_deref(),
                    )
                })
                .collect();

            let list = List::new(items);
            let mut todo_state = ListState::default();
            todo_state.select(Some(app.history_todo_index));
            frame.render_stateful_widget(list, right_rows[2], &mut todo_state);
        }
    }
}

/// Single line modern prompt at the bottom: "› Text typed...       enter save · esc cancel"
fn draw_input(app: &App, frame: &mut Frame, area: Rect) {
    let prompt_label = match app.input_mode {
        crate::model::InputMode::Editing(_) => "› Edit task ",
        _ => "› ",
    };

    let prompt_symbol = Span::styled(
        prompt_label,
        Style::default()
            .fg(THEME.accent)
            .add_modifier(Modifier::BOLD),
    );

    let hints = "enter save  ·  esc cancel";
    let hints_len = hints.chars().count();
    let prompt_len = prompt_label.chars().count();

    // Calculate available text width between prompt symbol and right hints
    let available_for_text = (area.width as usize).saturating_sub(prompt_len + hints_len + 3);

    let chars: Vec<char> = app.input_buffer.chars().collect();
    let visible_start = if app.cursor_position >= available_for_text && available_for_text > 0 {
        app.cursor_position - available_for_text + 1
    } else {
        0
    };

    let visible_input: String = chars
        .iter()
        .skip(visible_start)
        .take(available_for_text)
        .collect();

    let mut spans = vec![prompt_symbol];

    if app.input_buffer.is_empty() {
        let placeholder = match app.input_mode {
            crate::model::InputMode::Editing(_) => "",
            _ => "What needs doing today?",
        };
        spans.push(Span::styled(placeholder, Style::default().fg(THEME.dim)));
        let placeholder_len = placeholder.chars().count();
        let pad = (area.width as usize).saturating_sub(prompt_len + placeholder_len + hints_len);
        spans.push(Span::raw(" ".repeat(pad)));
    } else {
        spans.push(Span::styled(
            &visible_input,
            Style::default().fg(THEME.text),
        ));
        let pad = (area.width as usize)
            .saturating_sub(prompt_len + visible_input.chars().count() + hints_len);
        spans.push(Span::raw(" ".repeat(pad)));
    }

    spans.push(Span::styled(hints, Style::default().fg(THEME.dim)));

    frame.render_widget(Paragraph::new(Line::from(spans)), area);

    // Position the cursor right after prompt / typed character
    let cursor_offset = (app.cursor_position.saturating_sub(visible_start)) as u16;
    let cursor_x = (area.x + prompt_len as u16 + cursor_offset).min(area.right().saturating_sub(1));
    let cursor_y = area.y;
    frame.set_cursor_position(Position::new(cursor_x, cursor_y));
}

/// Single dim line of key hints that is short and context-aware
fn draw_footer(app: &App, frame: &mut Frame, area: Rect) {
    if let Some(ref err) = app.status_message {
        let error_line = Line::from(vec![
            Span::styled(
                "save error: ",
                Style::default()
                    .fg(THEME.error)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(err, Style::default().fg(THEME.error)),
        ]);
        frame.render_widget(Paragraph::new(error_line), area);
        return;
    }

    if let Some(ref msg) = app.transient_message {
        let msg_line = Line::from(vec![Span::styled(msg, Style::default().fg(THEME.subtext))]);
        frame.render_widget(Paragraph::new(msg_line), area);
        return;
    }

    let hints: Vec<(&str, &str)> = match app.current_view {
        View::Today => vec![
            ("a", "add"),
            ("space", "done"),
            ("d", "delete"),
            ("tab", "history"),
            ("q", "quit"),
        ],
        View::History => match app.history_pane {
            HistoryPane::Dates => vec![
                ("enter/l", "todos"),
                ("j/k", "navigate"),
                ("a", "add"),
                ("tab", "today"),
                ("q", "quit"),
            ],
            HistoryPane::Todos => vec![
                ("h", "dates"),
                ("space", "done"),
                ("d", "delete"),
                ("j/k", "navigate"),
                ("tab", "today"),
                ("q", "quit"),
            ],
        },
    };

    let mut spans: Vec<Span> = Vec::new();
    for (i, (key, action)) in hints.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  ·  ", Style::default().fg(THEME.divider)));
        }
        spans.push(Span::styled(*key, Style::default().fg(THEME.subtext)));
        spans.push(Span::raw(" "));
        spans.push(Span::styled(*action, Style::default().fg(THEME.dim)));
    }

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn format_todo_row(todo: &Todo, is_selected: bool, max_width: usize) -> ListItem<'static> {
    let marker = if is_selected {
        Span::styled(
            "› ",
            Style::default()
                .fg(THEME.accent)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::raw("  ")
    };

    let checkbox = if todo.done {
        Span::styled("✓ ", Style::default().fg(THEME.accent))
    } else {
        Span::styled("○ ", Style::default().fg(THEME.dim))
    };

    // Available space for text = width - marker (2) - checkbox (2)
    let avail = max_width.saturating_sub(5);
    let text_truncated = truncate_text(&todo.text, avail);

    let text_span = if todo.done {
        Span::styled(
            text_truncated,
            Style::default()
                .fg(THEME.dim)
                .add_modifier(Modifier::CROSSED_OUT),
        )
    } else {
        Span::styled(text_truncated, Style::default().fg(THEME.text))
    };

    ListItem::new(Line::from(vec![marker, checkbox, text_span]))
}

fn format_todo_row_history(
    todo: &Todo,
    is_selected: bool,
    is_active: bool,
    max_width: usize,
    time_str: Option<&str>,
) -> ListItem<'static> {
    let marker = if is_selected {
        if is_active {
            Span::styled(
                "› ",
                Style::default()
                    .fg(THEME.accent)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled("› ", Style::default().fg(THEME.dim))
        }
    } else {
        Span::raw("  ")
    };

    let checkbox = if todo.done {
        Span::styled("✓ ", Style::default().fg(THEME.accent))
    } else {
        Span::styled("○ ", Style::default().fg(THEME.dim))
    };

    let suffix_str = time_str.unwrap_or("").to_string();
    let suffix_len = if suffix_str.is_empty() {
        0
    } else {
        suffix_str.chars().count() + 2
    };

    // Available space for text = max_width - marker (2) - checkbox (2) - suffix
    let avail = max_width.saturating_sub(4 + suffix_len + 1);
    let text_truncated = truncate_text(&todo.text, avail);

    let text_span = if todo.done {
        Span::styled(
            text_truncated.clone(),
            Style::default()
                .fg(THEME.dim)
                .add_modifier(Modifier::CROSSED_OUT),
        )
    } else {
        Span::styled(text_truncated.clone(), Style::default().fg(THEME.text))
    };

    let pad_len = max_width.saturating_sub(4 + text_truncated.chars().count() + suffix_len);

    let mut spans = vec![marker, checkbox, text_span, Span::raw(" ".repeat(pad_len))];

    if !suffix_str.is_empty() {
        spans.push(Span::styled(suffix_str, Style::default().fg(THEME.dim)));
    }

    ListItem::new(Line::from(spans))
}

/// Formats dates friendly: "Today", "Yesterday", or "Mon 19 Oct"
fn format_friendly_date(date: NaiveDate, today: NaiveDate) -> String {
    if date == today {
        "Today".to_string()
    } else if today.pred_opt() == Some(date) {
        "Yesterday".to_string()
    } else {
        date.format("%a %-d %b").to_string()
    }
}

/// Truncates string gracefully with ellipsis "…" if it exceeds max_len
fn truncate_text(text: &str, max_len: usize) -> String {
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

/// Renders a quiet slim progress bar using block characters
fn render_progress_blocks(done: usize, total: usize, bar_width: usize) -> Vec<Span<'static>> {
    if total == 0 {
        return vec![Span::styled(
            "─".repeat(bar_width),
            Style::default().fg(THEME.dim),
        )];
    }
    let filled = ((done as f64 / total as f64) * bar_width as f64).round() as usize;
    let filled = filled.min(bar_width);
    let empty = bar_width.saturating_sub(filled);

    vec![
        Span::styled("█".repeat(filled), Style::default().fg(THEME.accent)),
        Span::styled("░".repeat(empty), Style::default().fg(THEME.dim)),
    ]
}
