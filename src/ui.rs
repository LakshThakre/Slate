use chrono::{NaiveDate, Timelike};
use ratatui::layout::{Constraint, Direction, Layout, Position, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

use crate::app::App;
use crate::config::TimeFormat;
use crate::model::{HistoryPane, Todo, View};
use crate::theme::Theme;

pub const MIN_WIDTH: u16 = 40;
pub const MIN_HEIGHT: u16 = 8;

pub fn is_too_small(width: u16, height: u16) -> bool {
    width < MIN_WIDTH || height < MIN_HEIGHT
}

pub fn format_time(dt: &chrono::DateTime<chrono::Local>, fmt: TimeFormat) -> String {
    match fmt {
        TimeFormat::TwentyFourHour => dt.format("%H:%M").to_string(),
        TimeFormat::TwelveHour => {
            let hour = dt.format("%-I");
            let minute = dt.format("%M");
            let am_pm = if dt.time().hour() < 12 { "am" } else { "pm" };
            format!("{hour}:{minute} {am_pm}")
        }
    }
}

pub fn render(app: &mut App, frame: &mut Frame) {
    let terminal_area = frame.area();
    app.terminal_width = terminal_area.width;
    app.terminal_height = terminal_area.height;
    app.clamp_selections();

    if is_too_small(terminal_area.width, terminal_area.height) {
        if terminal_area.width > 0 && terminal_area.height > 0 {
            let msg = format!(
                "Terminal too small · need {MIN_WIDTH}×{MIN_HEIGHT}, have {}×{}",
                terminal_area.width, terminal_area.height
            );
            let needed_lines = if terminal_area.width >= 42 { 1 } else { 2 };
            let top_pad = terminal_area.height.saturating_sub(needed_lines) / 2;
            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(top_pad),
                    Constraint::Length(needed_lines),
                    Constraint::Min(0),
                ])
                .split(terminal_area);
            let p = Paragraph::new(msg)
                .alignment(ratatui::layout::Alignment::Center)
                .wrap(ratatui::widgets::Wrap { trim: true })
                .style(Style::default().fg(app.theme.dim));
            frame.render_widget(p, rows[1]);
        }
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
            .style(Style::default().fg(app.theme.divider)),
        header_divider_area,
    );

    // 2. Draw Main View (Today or History)
    app.visible_list_height = (main_area.height.saturating_sub(2) as usize).max(1);
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
                .style(Style::default().fg(app.theme.divider)),
            prompt_divider_area,
        );
        draw_input(app, frame, bottom_area);
    } else {
        draw_footer(app, frame, bottom_area);
    }

    // 4. Draw Help Overlay if toggled
    if app.show_help {
        draw_help_overlay(app, frame);
    }
}

/// Slim top header line: App name + plain text view tabs
fn draw_header(app: &App, frame: &mut Frame, area: Rect) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let today_tab = if app.current_view == View::Today {
        Span::styled(
            "Today",
            Style::default()
                .fg(app.theme.accent)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )
    } else {
        Span::styled("Today", Style::default().fg(app.theme.dim))
    };

    let history_tab = if app.current_view == View::History {
        Span::styled(
            "History",
            Style::default()
                .fg(app.theme.accent)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )
    } else {
        Span::styled("History", Style::default().fg(app.theme.dim))
    };

    let line = Line::from(vec![
        Span::styled(
            "Slate",
            Style::default()
                .fg(app.theme.accent)
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
    if area.width == 0 || area.height == 0 {
        return;
    }
    let today = app.today;
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
    let date_str = today.format(&app.config.date_format).to_string();
    let bar_width = if area.width > 70 { 10 } else { 6 };
    let mut title_spans = vec![
        Span::styled(
            date_str,
            Style::default()
                .fg(app.theme.text)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("    "),
        Span::styled(
            format!("{done} of {total} done"),
            Style::default().fg(app.theme.dim),
        ),
    ];

    if total > 0 {
        title_spans.push(Span::raw("  "));
        title_spans.extend(render_progress_blocks(done, total, bar_width, &app.theme));
    }

    let overdue_count = app
        .todos
        .iter()
        .filter(|t| t.created_date < today && !t.done)
        .count();
    if overdue_count > 0 {
        title_spans.push(Span::raw("    "));
        title_spans.push(Span::styled(
            format!("{overdue_count} overdue"),
            Style::default().fg(app.theme.warn),
        ));
    }

    frame.render_widget(Paragraph::new(Line::from(title_spans)), parts[0]);

    if let Some((prev_date, ref ids)) = uncarried {
        let count = ids.len();
        let date_label = if prev_date == today.pred_opt().unwrap_or(today) {
            "yesterday".to_string()
        } else {
            format_friendly_date(prev_date, today, &app.config.short_date_format)
        };
        let hint_line = Line::from(vec![
            Span::styled(
                format!("{count} unfinished from {date_label}  ·  "),
                Style::default().fg(app.theme.dim),
            ),
            Span::styled(
                "c",
                Style::default()
                    .fg(app.theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" to carry over", Style::default().fg(app.theme.dim)),
        ]);
        frame.render_widget(Paragraph::new(hint_line), parts[1]);
    }

    let list_area = if has_uncarried { parts[3] } else { parts[2] };

    // Empty state or task list
    if today_todos.is_empty() {
        let empty_text = Line::from(vec![
            Span::styled("No todos yet. Press ", Style::default().fg(app.theme.dim)),
            Span::styled(
                "a",
                Style::default()
                    .fg(app.theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " to add your first one.",
                Style::default().fg(app.theme.dim),
            ),
        ]);
        frame.render_widget(Paragraph::new(empty_text), list_area);
        return;
    }

    let items: Vec<ListItem> = today_todos
        .iter()
        .enumerate()
        .map(|(idx, todo)| {
            let is_selected = idx == app.today_selected_index;
            format_todo_row(todo, is_selected, list_area.width as usize, &app.theme)
        })
        .collect();

    let list = List::new(items);
    let mut list_state = ListState::default();
    list_state.select(Some(app.today_selected_index));
    frame.render_stateful_widget(list, list_area, &mut list_state);
}

/// History view: Narrow Date column + thin vertical rule + Todo list for selected date
fn draw_history(app: &App, frame: &mut Frame, area: Rect) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let dates = app.dates_list();
    let today = app.today;

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
            let is_overdue = date < today && done < total;

            let marker = if is_selected {
                if is_date_pane_active {
                    Span::styled(
                        "› ",
                        Style::default()
                            .fg(app.theme.accent)
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    Span::styled("› ", Style::default().fg(app.theme.dim))
                }
            } else {
                Span::raw("  ")
            };

            let friendly_date = format_friendly_date(date, today, &app.config.short_date_format);
            let count_str = format!("{done}/{total}");

            // Calculate spacing so count aligns to the right edge of the date column
            let text_space = (date_area.width as usize).saturating_sub(2 + count_str.len());
            let date_truncated = truncate_text(&friendly_date, text_space.saturating_sub(1));
            let pad_spaces = text_space.saturating_sub(date_truncated.chars().count());

            let date_style = if is_selected && is_date_pane_active {
                Style::default()
                    .fg(app.theme.text)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(app.theme.text)
            };

            let count_style = if is_overdue {
                Style::default().fg(app.theme.warn)
            } else {
                Style::default().fg(app.theme.dim)
            };

            let line = Line::from(vec![
                marker,
                Span::styled(date_truncated, date_style),
                Span::raw(" ".repeat(pad_spaces)),
                Span::styled(count_str, count_style),
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
        Paragraph::new(divider_line).style(Style::default().fg(app.theme.divider)),
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
        let full_date_str = date.format(&app.config.date_format).to_string();
        let heading_line = Line::from(vec![
            Span::styled(
                full_date_str,
                Style::default()
                    .fg(app.theme.text)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("   •   ", Style::default().fg(app.theme.dim)),
            Span::styled(
                format!("{done} of {total} done"),
                Style::default().fg(app.theme.dim),
            ),
        ]);
        frame.render_widget(Paragraph::new(heading_line), right_rows[0]);

        if date_todos.is_empty() {
            frame.render_widget(
                Paragraph::new(Span::styled(
                    "No todos recorded for this date.",
                    Style::default().fg(app.theme.dim),
                )),
                right_rows[2],
            );
        } else {
            let is_date_overdue = date < today;
            let items: Vec<ListItem> = date_todos
                .iter()
                .enumerate()
                .map(|(idx, todo)| {
                    let is_selected = idx == app.history_todo_index;
                    let active_marker = is_selected && is_todo_pane_active;
                    let is_todo_overdue = is_date_overdue && !todo.done;
                    let time_str = if todo.done {
                        match todo.done_at {
                            Some(dt) => {
                                Some(format!("done {}", format_time(&dt, app.config.time_format)))
                            }
                            None => Some("done".to_string()),
                        }
                    } else {
                        None
                    };

                    format_todo_row_history(
                        todo,
                        is_selected,
                        active_marker,
                        is_todo_overdue,
                        right_rows[2].width as usize,
                        time_str.as_deref(),
                        &app.theme,
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
    if area.width == 0 || area.height == 0 {
        return;
    }
    let prompt_label = match app.input_mode {
        crate::model::InputMode::Editing(_) => "› Edit task ",
        _ => "› ",
    };

    let prompt_symbol = Span::styled(
        prompt_label,
        Style::default()
            .fg(app.theme.accent)
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
        spans.push(Span::styled(
            placeholder,
            Style::default().fg(app.theme.dim),
        ));
        let placeholder_len = placeholder.chars().count();
        let pad = (area.width as usize).saturating_sub(prompt_len + placeholder_len + hints_len);
        spans.push(Span::raw(" ".repeat(pad)));
    } else {
        spans.push(Span::styled(
            &visible_input,
            Style::default().fg(app.theme.text),
        ));
        let pad = (area.width as usize)
            .saturating_sub(prompt_len + visible_input.chars().count() + hints_len);
        spans.push(Span::raw(" ".repeat(pad)));
    }

    spans.push(Span::styled(hints, Style::default().fg(app.theme.dim)));

    frame.render_widget(Paragraph::new(Line::from(spans)), area);

    // Position the cursor right after prompt / typed character
    let cursor_offset = (app.cursor_position.saturating_sub(visible_start)) as u16;
    let cursor_x = (area.x + prompt_len as u16 + cursor_offset).min(area.right().saturating_sub(1));
    let cursor_y = area.y;
    frame.set_cursor_position(Position::new(cursor_x, cursor_y));
}

/// Single dim line of key hints that is short and context-aware
fn draw_footer(app: &App, frame: &mut Frame, area: Rect) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    if let Some(ref err) = app.status_message {
        let error_line = Line::from(vec![
            Span::styled(
                "save error: ",
                Style::default()
                    .fg(app.theme.error)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(err, Style::default().fg(app.theme.error)),
        ]);
        frame.render_widget(Paragraph::new(error_line), area);
        return;
    }

    if let Some(ref msg) = app.transient_message {
        let msg_line = Line::from(vec![Span::styled(
            msg,
            Style::default().fg(app.theme.subtext),
        )]);
        frame.render_widget(Paragraph::new(msg_line), area);
        return;
    }

    let hints: Vec<(&str, &str)> = match app.current_view {
        View::Today => vec![
            ("a", "add"),
            ("space", "done"),
            ("tab", "history"),
            ("?", "help"),
            ("q", "quit"),
        ],
        View::History => match app.history_pane {
            HistoryPane::Dates => vec![
                ("enter/l", "tasks"),
                ("tab", "today"),
                ("?", "help"),
                ("q", "quit"),
            ],
            HistoryPane::Todos => vec![
                ("h", "dates"),
                ("space", "done"),
                ("tab", "today"),
                ("?", "help"),
                ("q", "quit"),
            ],
        },
    };

    let mut spans: Vec<Span> = Vec::new();
    for (i, (key, action)) in hints.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(
                "  ·  ",
                Style::default().fg(app.theme.divider),
            ));
        }
        spans.push(Span::styled(*key, Style::default().fg(app.theme.subtext)));
        spans.push(Span::raw(" "));
        spans.push(Span::styled(*action, Style::default().fg(app.theme.dim)));
    }

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn format_todo_row(
    todo: &Todo,
    is_selected: bool,
    max_width: usize,
    theme: &Theme,
) -> ListItem<'static> {
    let marker = if is_selected {
        Span::styled(
            "› ",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::raw("  ")
    };

    let checkbox = if todo.done {
        Span::styled("✓ ", Style::default().fg(theme.accent))
    } else {
        Span::styled("○ ", Style::default().fg(theme.dim))
    };

    // Available space for text = width - marker (2) - checkbox (2)
    let avail = max_width.saturating_sub(5);
    let text_truncated = truncate_text(&todo.text, avail);

    let text_span = if todo.done {
        Span::styled(
            text_truncated,
            Style::default()
                .fg(theme.dim)
                .add_modifier(Modifier::CROSSED_OUT),
        )
    } else {
        Span::styled(text_truncated, Style::default().fg(theme.text))
    };

    ListItem::new(Line::from(vec![marker, checkbox, text_span]))
}

fn format_todo_row_history(
    todo: &Todo,
    is_selected: bool,
    is_active: bool,
    is_overdue: bool,
    max_width: usize,
    time_str: Option<&str>,
    theme: &Theme,
) -> ListItem<'static> {
    let marker = if is_selected {
        if is_active {
            Span::styled(
                "› ",
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled("› ", Style::default().fg(theme.dim))
        }
    } else {
        Span::raw("  ")
    };

    let checkbox = if todo.done {
        Span::styled("✓ ", Style::default().fg(theme.accent))
    } else if is_overdue {
        Span::styled("! ", Style::default().fg(theme.warn))
    } else {
        Span::styled("○ ", Style::default().fg(theme.dim))
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
                .fg(theme.dim)
                .add_modifier(Modifier::CROSSED_OUT),
        )
    } else {
        Span::styled(text_truncated.clone(), Style::default().fg(theme.text))
    };

    let pad_len = max_width.saturating_sub(4 + text_truncated.chars().count() + suffix_len);

    let mut spans = vec![marker, checkbox, text_span, Span::raw(" ".repeat(pad_len))];

    if !suffix_str.is_empty() {
        spans.push(Span::styled(suffix_str, Style::default().fg(theme.dim)));
    }

    ListItem::new(Line::from(spans))
}

/// Formats dates friendly: "Today", "Yesterday", or "Mon 19 Oct"
fn format_friendly_date(date: NaiveDate, today: NaiveDate, short_date_format: &str) -> String {
    if date == today {
        "Today".to_string()
    } else if today.pred_opt() == Some(date) {
        "Yesterday".to_string()
    } else {
        date.format(short_date_format).to_string()
    }
}

/// Truncates string gracefully with ellipsis "…" if it exceeds max_len
fn truncate_text(text: &str, max_len: usize) -> String {
    if max_len == 0 {
        return String::new();
    }
    let char_count = text.chars().count();
    if char_count <= max_len {
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
fn render_progress_blocks(
    done: usize,
    total: usize,
    bar_width: usize,
    theme: &Theme,
) -> Vec<Span<'static>> {
    if bar_width == 0 {
        return Vec::new();
    }
    if total == 0 {
        return vec![Span::styled(
            "─".repeat(bar_width),
            Style::default().fg(theme.dim),
        )];
    }
    let filled = ((done as f64 / total as f64) * bar_width as f64).round() as usize;
    let filled = filled.min(bar_width);
    let empty = bar_width.saturating_sub(filled);

    vec![
        Span::styled("█".repeat(filled), Style::default().fg(theme.accent)),
        Span::styled("░".repeat(empty), Style::default().fg(theme.dim)),
    ]
}

fn draw_help_overlay(app: &App, frame: &mut Frame) {
    let area = frame.area();
    if area.width < 10 || area.height < 4 {
        return;
    }

    // Centered overlay area, clamped to area bounds
    let width = 68.min(area.width.saturating_sub(2)).max(1);
    let height = 22.min(area.height.saturating_sub(2)).max(1);
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let overlay_area = Rect::new(x, y, width, height);

    frame.render_widget(Clear, overlay_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Title line: "Help" ... "esc / q / ? close"
            Constraint::Length(1), // Thin divider rule
            Constraint::Min(0),    // Grouped keybindings
        ])
        .split(overlay_area);

    let title_line = Line::from(vec![
        Span::styled(
            "Help",
            Style::default()
                .fg(app.theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" ".repeat((chunks[0].width as usize).saturating_sub(25))),
        Span::styled("esc / q / ? to close", Style::default().fg(app.theme.dim)),
    ]);
    frame.render_widget(Paragraph::new(title_line), chunks[0]);

    frame.render_widget(
        Paragraph::new("─".repeat(chunks[1].width as usize))
            .style(Style::default().fg(app.theme.divider)),
        chunks[1],
    );

    let body_area = chunks[2];
    if body_area.height == 0 || body_area.width == 0 {
        return;
    }

    let is_wide = body_area.width >= 56;
    if is_wide {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50),
                Constraint::Length(2),
                Constraint::Percentage(50),
            ])
            .split(body_area);

        frame.render_widget(Paragraph::new(build_left_help_lines(&app.theme)), cols[0]);
        frame.render_widget(Paragraph::new(build_right_help_lines(&app.theme)), cols[2]);
    } else {
        frame.render_widget(Paragraph::new(build_all_help_lines(&app.theme)), body_area);
    }
}

fn help_entry(key: &str, desc: &str, pad_key: usize, theme: &Theme) -> Line<'static> {
    let key_pad = pad_key.saturating_sub(key.chars().count());
    Line::from(vec![
        Span::styled(
            format!("  {key}"),
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" ".repeat(key_pad + 2)),
        Span::styled(desc.to_string(), Style::default().fg(theme.dim)),
    ])
}

fn help_header(title: &str, theme: &Theme) -> Line<'static> {
    Line::from(vec![Span::styled(
        title.to_string(),
        Style::default()
            .fg(theme.text)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
    )])
}

fn build_left_help_lines(theme: &Theme) -> Vec<Line<'static>> {
    vec![
        help_header("Navigation", theme),
        help_entry("Tab", "switch views", 10, theme),
        help_entry("j / k", "navigate down / up", 10, theme),
        help_entry("g / G", "jump top / bottom", 10, theme),
        help_entry("Ctrl+d/u", "half page down / up", 10, theme),
        help_entry("?", "toggle help", 10, theme),
        help_entry("q", "quit", 10, theme),
        Line::raw(""),
        help_header("History", theme),
        help_entry("enter/l", "focus tasks column", 10, theme),
        help_entry("h", "focus dates column", 10, theme),
        Line::raw(""),
        help_header("Input", theme),
        help_entry("enter", "save", 10, theme),
        help_entry("esc", "cancel", 10, theme),
    ]
}

fn build_right_help_lines(theme: &Theme) -> Vec<Line<'static>> {
    vec![
        help_header("Tasks", theme),
        help_entry("a", "add new task", 10, theme),
        help_entry("e", "edit selected task", 10, theme),
        help_entry("space", "toggle complete", 10, theme),
        help_entry("J / K", "move task down / up", 10, theme),
        help_entry("m", "move task to Today", 10, theme),
        help_entry("d", "delete task", 10, theme),
        help_entry("D", "clear completed tasks", 10, theme),
        help_entry("c", "carry over unfinished", 10, theme),
        help_entry("u", "undo last action", 10, theme),
        help_entry("Ctrl+r", "redo undone action", 10, theme),
    ]
}

fn build_all_help_lines(theme: &Theme) -> Vec<Line<'static>> {
    let mut lines = build_left_help_lines(theme);
    lines.push(Line::raw(""));
    lines.extend(build_right_help_lines(theme));
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::InputMode;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    #[test]
    fn test_render_help_overlay() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new_for_test(Vec::new());
        app.show_help = true;

        terminal.draw(|f| render(&mut app, f)).unwrap();
        let buffer = terminal.backend().buffer();
        let content: String = buffer.content().iter().map(|c| c.symbol()).collect();
        assert!(content.contains("Help"));
        assert!(content.contains("Navigation"));
        assert!(content.contains("Tasks"));
    }

    #[test]
    fn test_render_small_terminal_no_panic() {
        for (w, h) in [(10, 4), (5, 2), (20, 5), (15, 6), (24, 6), (30, 8)] {
            let backend = TestBackend::new(w, h);
            let mut terminal = Terminal::new(backend).unwrap();
            let mut app = App::new_for_test(Vec::new());

            app.show_help = false;
            terminal.draw(|f| render(&mut app, f)).unwrap();

            app.show_help = true;
            terminal.draw(|f| render(&mut app, f)).unwrap();
        }
    }

    #[test]
    fn test_render_overdue_marker() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let today = NaiveDate::from_ymd_opt(2026, 10, 27).unwrap();
        let yesterday = today.pred_opt().unwrap();
        let mut app = App::new_for_test_with_date(
            vec![Todo {
                id: 1,
                text: "Overdue old task".to_string(),
                created_date: yesterday,
                done: false,
                done_at: None,
            }],
            today,
        );

        // In Today view, header should show "1 overdue"
        terminal.draw(|f| render(&mut app, f)).unwrap();
        let content: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(content.contains("1 overdue"));

        // Switch to History view and select yesterday
        app.current_view = View::History;
        app.history_pane = HistoryPane::Todos;
        let dates = app.dates_list();
        app.history_date_index = dates.iter().position(|&d| d == yesterday).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();

        let buffer = terminal.backend().buffer();
        let content_history: String = buffer.content().iter().map(|c| c.symbol()).collect();
        assert!(content_history.contains("Overdue old task"));
        // The overdue marker '!' should be rendered
        assert!(content_history.contains('!'));

        // Verify that the '!' cell has fg color app.theme.warn
        let has_warn_cell = buffer
            .content()
            .iter()
            .any(|c| c.symbol() == "!" && c.fg == app.theme.warn);
        assert!(has_warn_cell);
    }

    #[test]
    fn test_render_today_history_and_input_views() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let today = NaiveDate::from_ymd_opt(2026, 10, 27).unwrap();
        let mut app = App::new_for_test_with_date(
            vec![Todo {
                id: 1,
                text: "Characterization task 1".to_string(),
                created_date: today,
                done: false,
                done_at: None,
            }],
            today,
        );

        // 1. Today view
        app.current_view = View::Today;
        terminal.draw(|f| render(&mut app, f)).unwrap();
        let buf_today: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(buf_today.contains("Slate"));
        assert!(buf_today.contains("Today"));
        assert!(buf_today.contains("History"));
        assert!(buf_today.contains("Characterization task 1"));
        assert!(buf_today.contains("a add"));

        // 2. History view
        app.current_view = View::History;
        app.history_pane = HistoryPane::Dates;
        terminal.draw(|f| render(&mut app, f)).unwrap();
        let buf_history: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(buf_history.contains("Slate"));
        assert!(buf_history.contains("enter/l tasks"));

        // 3. Input prompt: Adding mode
        app.current_view = View::Today;
        app.input_mode = InputMode::Adding;
        app.input_buffer = "Typing new todo".to_string();
        app.cursor_position = app.input_buffer.len();
        terminal.draw(|f| render(&mut app, f)).unwrap();
        let buf_adding: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(buf_adding.contains("Typing new todo"));
        assert!(buf_adding.contains("enter save"));
        assert!(buf_adding.contains("esc cancel"));

        // 4. Input prompt: Editing mode
        app.input_mode = InputMode::Editing(1);
        app.input_buffer = "Editing todo".to_string();
        app.cursor_position = app.input_buffer.len();
        terminal.draw(|f| render(&mut app, f)).unwrap();
        let buf_editing: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(buf_editing.contains("Edit task"));
        assert!(buf_editing.contains("Editing todo"));
    }

    #[test]
    fn test_render_all_sizes_and_modes_no_panic() {
        let test_sizes = [
            (0, 0),
            (1, 1),
            (10, 3),
            (39, 24),
            (40, 7),
            (40, 8),
            (200, 60),
            (1, 24),
            (40, 1),
        ];

        let today = NaiveDate::from_ymd_opt(2026, 10, 27).unwrap();
        let sample_todos = vec![
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
                created_date: today.pred_opt().unwrap(),
                done: true,
                done_at: None,
            },
        ];

        for (w, h) in test_sizes {
            for view in [View::Today, View::History] {
                for mode in [InputMode::Normal, InputMode::Adding, InputMode::Editing(1)] {
                    for show_help in [false, true] {
                        let backend = TestBackend::new(w, h);
                        let mut terminal = Terminal::new(backend).unwrap();
                        let mut app = App::new_for_test_with_date(sample_todos.clone(), today);
                        app.current_view = view;
                        app.input_mode = mode;
                        app.show_help = show_help;
                        if mode != InputMode::Normal {
                            app.input_buffer = "Work in progress".to_string();
                            app.cursor_position = app.input_buffer.len();
                        }
                        // Rendering must never panic regardless of terminal dimensions
                        terminal.draw(|f| render(&mut app, f)).unwrap();
                    }
                }
            }
        }
    }

    #[test]
    fn test_too_small_message_below_minimum_and_not_at_or_above() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 27).unwrap();

        // 1. Below minimum: 39x24 (width too small)
        {
            let backend = TestBackend::new(39, 24);
            let mut terminal = Terminal::new(backend).unwrap();
            let mut app = App::new_for_test_with_date(Vec::new(), today);
            terminal.draw(|f| render(&mut app, f)).unwrap();
            let buf: String = terminal
                .backend()
                .buffer()
                .content()
                .iter()
                .map(|c| c.symbol())
                .collect();
            assert!(buf.contains("Terminal too small"));
            assert!(buf.contains("need 40×8"));
            assert!(buf.contains("39×24"));
        }

        // 2. Below minimum: 40x7 (height too small)
        {
            let backend = TestBackend::new(40, 7);
            let mut terminal = Terminal::new(backend).unwrap();
            let mut app = App::new_for_test_with_date(Vec::new(), today);
            terminal.draw(|f| render(&mut app, f)).unwrap();
            let buf: String = terminal
                .backend()
                .buffer()
                .content()
                .iter()
                .map(|c| c.symbol())
                .collect();
            assert!(buf.contains("Terminal too small"));
            assert!(buf.contains("need 40×8"));
            assert!(buf.contains("40×7"));
        }

        // 3. Below minimum: 10x3
        {
            let backend = TestBackend::new(10, 3);
            let mut terminal = Terminal::new(backend).unwrap();
            let mut app = App::new_for_test_with_date(Vec::new(), today);
            terminal.draw(|f| render(&mut app, f)).unwrap();
            let buf: String = terminal
                .backend()
                .buffer()
                .content()
                .iter()
                .map(|c| c.symbol())
                .collect();
            assert!(buf.contains("Terminal"));
        }

        // 4. Exactly at minimum: 40x8
        {
            let backend = TestBackend::new(40, 8);
            let mut terminal = Terminal::new(backend).unwrap();
            let mut app = App::new_for_test_with_date(Vec::new(), today);
            terminal.draw(|f| render(&mut app, f)).unwrap();
            let buf: String = terminal
                .backend()
                .buffer()
                .content()
                .iter()
                .map(|c| c.symbol())
                .collect();
            assert!(
                !buf.contains("Terminal too small"),
                "Did not expect small message at 40x8, got: {buf}"
            );
            assert!(buf.contains("Today"));
        }

        // 5. Well above minimum: 200x60
        {
            let backend = TestBackend::new(200, 60);
            let mut terminal = Terminal::new(backend).unwrap();
            let mut app = App::new_for_test_with_date(Vec::new(), today);
            terminal.draw(|f| render(&mut app, f)).unwrap();
            let buf: String = terminal
                .backend()
                .buffer()
                .content()
                .iter()
                .map(|c| c.symbol())
                .collect();
            assert!(!buf.contains("Terminal too small"));
            assert!(buf.contains("Today"));
        }
    }
}
