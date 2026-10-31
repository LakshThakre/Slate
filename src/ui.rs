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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Symbols {
    pub check_done: &'static str,
    pub check_todo: &'static str,
    pub check_overdue: &'static str,
    pub marker: &'static str,
    pub mark: &'static str,
    pub prompt_default: &'static str,
    pub prompt_subtask: &'static str,
    pub prompt_edit: &'static str,
    pub prompt_reschedule: &'static str,
    pub prompt_filter: &'static str,
    pub prompt_recurrence: &'static str,
    pub prompt_search: &'static str,
    pub prompt_jump: &'static str,
    pub note: &'static str,
    pub repeat: &'static str,
    pub subtask_connector: &'static str,
    pub focus_marker: &'static str,
    pub focus_prefix: &'static str,
    pub v_divider: &'static str,
    pub h_rule: &'static str,
    pub bullet: &'static str,
    pub dot: &'static str,
    pub ellipsis: &'static str,
    pub multiply: &'static str,
    pub progress_filled: &'static str,
    pub progress_empty: &'static str,
    pub heat_empty: &'static str,
    pub heat_1: &'static str,
    pub heat_2: &'static str,
    pub heat_3: &'static str,
    pub heat_4: &'static str,
    pub is_ascii: bool,
}

impl Default for Symbols {
    fn default() -> Self {
        Self::unicode()
    }
}

impl Symbols {
    pub const fn unicode() -> Self {
        Self {
            check_done: "✓ ",
            check_todo: "○ ",
            check_overdue: "! ",
            marker: "› ",
            mark: "◆ ",
            prompt_default: "› ",
            prompt_subtask: "› Add subtask ",
            prompt_edit: "› Edit task ",
            prompt_reschedule: "› Reschedule: ",
            prompt_filter: "› filter ",
            prompt_recurrence: "› recurrence: ",
            prompt_search: "› search ",
            prompt_jump: "› go to date ",
            note: "≡",
            repeat: "↻ ",
            subtask_connector: "└ ",
            focus_marker: "▸ ",
            focus_prefix: "▸ Focus: ",
            v_divider: "│",
            h_rule: "─",
            bullet: "•",
            dot: "·",
            ellipsis: "…",
            multiply: "×",
            progress_filled: "█",
            progress_empty: "░",
            heat_empty: "·",
            heat_1: "░",
            heat_2: "▒",
            heat_3: "▓",
            heat_4: "█",
            is_ascii: false,
        }
    }

    pub const fn ascii() -> Self {
        Self {
            check_done: "[x] ",
            check_todo: "[ ] ",
            check_overdue: "[!] ",
            marker: "> ",
            mark: "* ",
            prompt_default: "> ",
            prompt_subtask: "> Add subtask ",
            prompt_edit: "> Edit task ",
            prompt_reschedule: "> Reschedule: ",
            prompt_filter: "> filter ",
            prompt_recurrence: "> recurrence: ",
            prompt_search: "> search ",
            prompt_jump: "> go to date ",
            note: "=",
            repeat: "~ ",
            subtask_connector: "` ",
            focus_marker: "> ",
            focus_prefix: "> Focus: ",
            v_divider: "|",
            h_rule: "-",
            bullet: "*",
            dot: "*",
            ellipsis: "...",
            multiply: "x",
            progress_filled: "#",
            progress_empty: "-",
            heat_empty: ".",
            heat_1: "-",
            heat_2: "+",
            heat_3: "*",
            heat_4: "#",
            is_ascii: true,
        }
    }

    pub fn heat(&self, level: usize) -> &'static str {
        match level {
            1 => self.heat_1,
            2 => self.heat_2,
            3 => self.heat_3,
            4 => self.heat_4,
            _ => self.heat_empty,
        }
    }

    pub fn from_config(config: &crate::config::Config) -> Self {
        Self::from_config_with_env(config, &crate::config::SystemEnv)
    }

    pub fn from_config_with_env(
        config: &crate::config::Config,
        env: &impl crate::config::EnvLookup,
    ) -> Self {
        let env_ascii = env
            .get("SLATE_ASCII")
            .map(|s| {
                let t = s.trim();
                t == "1" || t.eq_ignore_ascii_case("true")
            })
            .unwrap_or(false);

        if env_ascii || config.ui.symbols == crate::config::SymbolsMode::Ascii {
            Self::ascii()
        } else {
            Self::unicode()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitTarget {
    TabToday,
    TabUpcoming,
    TabHistory,
    TabSummary,
    TodayTask { index: usize },
    TodayCheckbox { index: usize },
    HistoryDate { index: usize },
    HistoryTask { index: usize },
    HistoryCheckbox { index: usize },
    UpcomingDate { index: usize },
    UpcomingTask { index: usize },
    UpcomingCheckbox { index: usize },
    DateArea,
    TodoArea,
    SearchResult { index: usize },
    SummaryDay { date: NaiveDate },
    SummaryWeek { index: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HitArea {
    pub rect: Rect,
    pub target: HitTarget,
}

impl HitArea {
    pub fn contains(&self, col: u16, row: u16) -> bool {
        col >= self.rect.x
            && col < self.rect.x.saturating_add(self.rect.width)
            && row >= self.rect.y
            && row < self.rect.y.saturating_add(self.rect.height)
    }
}

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
    app.hit_areas.clear();
    let terminal_area = frame.area();
    app.terminal_width = terminal_area.width;
    app.terminal_height = terminal_area.height;
    app.clamp_selections();

    if is_too_small(terminal_area.width, terminal_area.height) {
        if terminal_area.width > 0 && terminal_area.height > 0 {
            let msg = format!(
                "Terminal too small {} need {MIN_WIDTH}{}{MIN_HEIGHT}, have {}{}{}",
                app.symbols.dot,
                app.symbols.multiply,
                terminal_area.width,
                app.symbols.multiply,
                terminal_area.height
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
                .style(app.theme.dim_style());
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
        Paragraph::new(
            app.symbols
                .h_rule
                .repeat(header_divider_area.width as usize),
        )
        .style(app.theme.divider_style()),
        header_divider_area,
    );

    // 2. Draw Main View (Today, Upcoming, or History, or Search)
    app.visible_list_height = (main_area.height.saturating_sub(2) as usize).max(1);
    if app.input_mode == crate::model::InputMode::Searching {
        draw_search_results(app, frame, main_area);
    } else {
        match app.current_view {
            View::Today => draw_today(app, frame, main_area),
            View::Upcoming => draw_upcoming(app, frame, main_area),
            View::History => draw_history(app, frame, main_area),
            View::Summary => draw_summary(app, frame, main_area),
        }
    }

    // 3. Draw Bottom Area (Inline prompt when adding, editing, or rescheduling, otherwise subtle key hints)
    if app.input_mode != crate::model::InputMode::Normal {
        frame.render_widget(
            Paragraph::new(
                app.symbols
                    .h_rule
                    .repeat(prompt_divider_area.width as usize),
            )
            .style(app.theme.divider_style()),
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
fn draw_header(app: &mut App, frame: &mut Frame, area: Rect) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let mut header_spans = vec![
        Span::styled(
            "Slate",
            app.theme.accent_style().add_modifier(Modifier::BOLD),
        ),
        Span::raw("    "),
    ];
    let mut cur_x = area.x + 9;

    let today_tab = if app.current_view == View::Today {
        Span::styled(
            "Today",
            app.theme
                .accent_style()
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )
    } else {
        Span::styled("Today", app.theme.dim_style())
    };
    header_spans.push(today_tab);
    app.hit_areas.push(HitArea {
        rect: Rect::new(cur_x, area.y, 5, 1),
        target: HitTarget::TabToday,
    });
    cur_x += 5;

    if app.config.ui.show_upcoming {
        header_spans.push(Span::raw("   "));
        cur_x += 3;
        let upcoming_tab = if app.current_view == View::Upcoming {
            Span::styled(
                "Upcoming",
                app.theme
                    .accent_style()
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            )
        } else {
            Span::styled("Upcoming", app.theme.dim_style())
        };
        header_spans.push(upcoming_tab);
        app.hit_areas.push(HitArea {
            rect: Rect::new(cur_x, area.y, 8, 1),
            target: HitTarget::TabUpcoming,
        });
        cur_x += 8;
    }

    if app.config.ui.show_history {
        header_spans.push(Span::raw("   "));
        cur_x += 3;
        let history_tab = if app.current_view == View::History {
            Span::styled(
                "History",
                app.theme
                    .accent_style()
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            )
        } else {
            Span::styled("History", app.theme.dim_style())
        };
        header_spans.push(history_tab);
        app.hit_areas.push(HitArea {
            rect: Rect::new(cur_x, area.y, 7, 1),
            target: HitTarget::TabHistory,
        });
        cur_x += 7;
    }

    if app.config.ui.show_history && app.config.ui.show_summary {
        header_spans.push(Span::raw("   "));
        cur_x += 3;
        let summary_tab = if app.current_view == View::Summary {
            Span::styled(
                "Summary",
                app.theme
                    .accent_style()
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            )
        } else {
            Span::styled("Summary", app.theme.dim_style())
        };
        header_spans.push(summary_tab);
        app.hit_areas.push(HitArea {
            rect: Rect::new(cur_x, area.y, 7, 1),
            target: HitTarget::TabSummary,
        });
    }

    if app.sort_mode != crate::model::SortMode::Manual {
        header_spans.push(Span::raw("    "));
        header_spans.push(Span::styled(
            format!("sort: {}", app.sort_mode),
            app.theme.dim_style(),
        ));
    }

    let cur_filter = app.current_filter();
    if !cur_filter.is_empty() {
        header_spans.push(Span::raw("    "));
        header_spans.push(Span::styled(
            cur_filter.format_header(),
            app.theme.dim_style(),
        ));
    }

    let line = Line::from(header_spans);

    frame.render_widget(Paragraph::new(line), area);
}
/// Quiet Today view: Title row with date, count, slim block progress bar, and todo list
fn draw_today(app: &mut App, frame: &mut Frame, area: Rect) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let today = app.today;
    let today_todos: Vec<&Todo> = app
        .todos
        .iter()
        .filter(|t| t.created_date == today)
        .collect();
    // Phase 5: Header progress bar and count use top-level tasks only
    let today_top: Vec<&Todo> = today_todos
        .iter()
        .copied()
        .filter(|t| t.parent.is_none())
        .collect();
    let total = today_top.len();
    let done = today_top.iter().filter(|t| t.done).count();

    let uncarried = app.uncarried_previous_tasks();
    let has_uncarried = uncarried.is_some();

    let focus_task = app
        .focus
        .and_then(|fid| app.todos.iter().find(|t| t.id == fid && !t.done));
    let has_focus = focus_task.is_some();

    let mut constraints = vec![Constraint::Length(1)]; // Title row
    if has_uncarried {
        constraints.push(Constraint::Length(1)); // Carry over hint line
    }
    if has_focus {
        constraints.push(Constraint::Length(1)); // Focus line
    }
    constraints.push(Constraint::Length(1)); // Blank line
    constraints.push(Constraint::Min(0)); // Todos list

    let parts = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    // Friendly title row: "Friday, 23 October   3 of 5 done  ████░░░░░░"
    let date_str = today.format(&app.config.date_format).to_string();
    let mut title_spans = vec![Span::styled(
        date_str,
        app.theme.text_style().add_modifier(Modifier::BOLD),
    )];

    if app.config.progress.show_count {
        title_spans.push(Span::raw("    "));
        title_spans.push(Span::styled(
            format!("{done} of {total} done"),
            app.theme.dim_style(),
        ));
    }

    let progress_style = if app.symbols.is_ascii
        && app.config.progress.style != crate::config::ProgressStyle::None
    {
        crate::config::ProgressStyle::Ascii
    } else {
        app.config.progress.style
    };

    if progress_style != crate::config::ProgressStyle::None {
        let bar_spans = render_progress_bar(
            done,
            total,
            app.config.progress.width as usize,
            progress_style,
            &app.theme,
            &app.symbols,
        );
        if !bar_spans.is_empty() {
            title_spans.push(Span::raw("  "));
            title_spans.extend(bar_spans);
        }
    }

    if app.config.ui.show_history {
        let overdue_count = app
            .todos
            .iter()
            .filter(|t| t.created_date < today && !t.done)
            .count();
        if overdue_count > 0 {
            title_spans.push(Span::raw("    "));
            title_spans.push(Span::styled(
                format!("{overdue_count} overdue"),
                app.theme.warn_style(),
            ));
        }
    }

    let upcoming_count = app
        .todos
        .iter()
        .filter(|t| t.created_date > today && t.parent.is_none() && !t.done)
        .count();
    if upcoming_count > 0 {
        title_spans.push(Span::raw("    "));
        title_spans.push(Span::styled(
            format!("+{upcoming_count} upcoming"),
            app.theme.dim_style(),
        ));
    }

    frame.render_widget(Paragraph::new(Line::from(title_spans)), parts[0]);
    let mut cur_part = 1;

    if let Some((prev_date, ref ids)) = uncarried {
        let count = ids
            .iter()
            .filter(|&&id| app.todos.iter().any(|t| t.id == id && t.parent.is_none()))
            .count();
        let date_label = if prev_date == today.pred_opt().unwrap_or(today) {
            "yesterday".to_string()
        } else {
            format_friendly_date(prev_date, today, &app.config.short_date_format)
        };
        let hint_line = Line::from(vec![
            Span::styled(
                format!(
                    "{count} unfinished from {date_label}  {}  ",
                    app.symbols.dot
                ),
                app.theme.dim_style(),
            ),
            Span::styled(
                app.keymap.hint(crate::keymap::Action::CarryOver),
                app.theme.accent_style().add_modifier(Modifier::BOLD),
            ),
            Span::styled(" to carry over", app.theme.dim_style()),
        ]);
        frame.render_widget(Paragraph::new(hint_line), parts[cur_part]);
        cur_part += 1;
    }

    if let Some(focus) = focus_task {
        let focus_area = parts[cur_part];
        cur_part += 1;
        let prefix = app.symbols.focus_prefix;
        let prefix_len = prefix.chars().count();
        let max_text_width = (focus_area.width as usize).saturating_sub(prefix_len);
        let text_truncated = truncate_text(&focus.text, max_text_width, app.symbols.ellipsis);
        let focus_line = Line::from(vec![Span::styled(
            format!("{prefix}{text_truncated}"),
            app.theme.dim_style(),
        )]);
        frame.render_widget(Paragraph::new(focus_line), focus_area);
    }

    cur_part += 1; // Skip blank line
    let raw_list_area = parts[cur_part];

    let show_detail = (app.show_detail_pane
        || matches!(app.input_mode, crate::model::InputMode::EditingNote(_)))
        && app.terminal_height >= 20;

    let (list_area, divider_area, detail_area) = if show_detail && raw_list_area.height >= 8 {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(4),
                Constraint::Length(1),
                Constraint::Length(6.min(raw_list_area.height.saturating_sub(5))),
            ])
            .split(raw_list_area);
        (chunks[0], Some(chunks[1]), Some(chunks[2]))
    } else {
        (raw_list_area, None, None)
    };

    // Empty state or task list
    if today_todos.is_empty() {
        let empty_text = Line::from(vec![
            Span::styled("No todos yet. Press ", app.theme.dim_style()),
            Span::styled(
                app.keymap.hint(crate::keymap::Action::Add),
                app.theme.accent_style().add_modifier(Modifier::BOLD),
            ),
            Span::styled(" to add your first one.", app.theme.dim_style()),
        ]);
        frame.render_widget(Paragraph::new(empty_text), list_area);
        return;
    }

    let rows = app.visible_rows_today();
    let items: Vec<ListItem> = rows
        .iter()
        .enumerate()
        .filter_map(|(idx, row)| {
            let todo = app.todos.iter().find(|t| t.id == row.id)?;
            let is_selected = idx == app.today_selected_index;
            let subtask_progress = if row.depth == 0 {
                let subs: Vec<&Todo> = app
                    .todos
                    .iter()
                    .filter(|t| t.parent == Some(todo.id))
                    .collect();
                if !subs.is_empty() {
                    let done_subs = subs.iter().filter(|t| t.done).count();
                    Some((done_subs, subs.len()))
                } else {
                    None
                }
            } else {
                None
            };
            let is_pinned = app.focus == Some(todo.id);
            let is_marked = app.marks.contains(&todo.id);
            Some(format_todo_row(
                todo,
                row.depth,
                subtask_progress,
                is_selected,
                is_marked,
                is_pinned,
                list_area.width as usize,
                app.terminal_width,
                app.config.tasks.show_tags,
                &app.theme,
                &app.symbols,
            ))
        })
        .collect();

    let list = List::new(items);
    let mut list_state = ListState::default();
    list_state.select(Some(app.today_selected_index));
    frame.render_stateful_widget(list, list_area, &mut list_state);

    let offset = list_state.offset();
    let visible_count = rows
        .len()
        .saturating_sub(offset)
        .min(list_area.height as usize);
    let marker_len = app.symbols.marker.chars().count() as u16;
    let checkbox_len = app.symbols.check_done.chars().count() as u16;
    for i in 0..visible_count {
        let item_idx = offset + i;
        let row_y = list_area.y + i as u16;
        let connector_len = if rows[item_idx].depth > 0 {
            app.symbols.subtask_connector.chars().count() as u16
        } else {
            0
        };
        app.hit_areas.push(HitArea {
            rect: Rect::new(
                list_area.x + marker_len + connector_len,
                row_y,
                checkbox_len,
                1,
            ),
            target: HitTarget::TodayCheckbox { index: item_idx },
        });
        app.hit_areas.push(HitArea {
            rect: Rect::new(list_area.x, row_y, list_area.width, 1),
            target: HitTarget::TodayTask { index: item_idx },
        });
    }

    if let (Some(div), Some(det)) = (divider_area, detail_area) {
        draw_detail_pane(app, frame, det, div, app.today_selected_id);
    }
}

/// History view: Narrow Date column + thin vertical rule + Todo list for selected date
fn draw_history(app: &mut App, frame: &mut Frame, area: Rect) {
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

    app.hit_areas.push(HitArea {
        rect: date_area,
        target: HitTarget::DateArea,
    });
    app.hit_areas.push(HitArea {
        rect: todo_area,
        target: HitTarget::TodoArea,
    });

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
            // Phase 5: Progress count and overdue markers use top-level tasks only
            let date_top: Vec<&Todo> = date_todos
                .iter()
                .copied()
                .filter(|t| t.parent.is_none())
                .collect();
            let total = date_top.len();
            let done = date_top.iter().filter(|t| t.done).count();
            let is_overdue = date < today && done < total;

            let marker = if is_selected {
                if is_date_pane_active {
                    Span::styled(
                        app.symbols.marker,
                        app.theme.accent_style().add_modifier(Modifier::BOLD),
                    )
                } else {
                    Span::styled(app.symbols.marker, app.theme.dim_style())
                }
            } else {
                Span::raw(" ".repeat(app.symbols.marker.chars().count()))
            };

            let friendly_date = format_friendly_date(date, today, &app.config.short_date_format);
            let count_str = format!("{done}/{total}");

            // Calculate spacing so count aligns to the right edge of the date column
            let marker_width = app.symbols.marker.chars().count();
            let text_space =
                (date_area.width as usize).saturating_sub(marker_width + count_str.len());
            let date_truncated = truncate_text(
                &friendly_date,
                text_space.saturating_sub(1),
                app.symbols.ellipsis,
            );
            let pad_spaces = text_space.saturating_sub(date_truncated.chars().count());

            let date_style = if is_selected && is_date_pane_active {
                app.theme.text_style().add_modifier(Modifier::BOLD)
            } else {
                app.theme.text_style()
            };

            let count_style = if is_overdue {
                app.theme.warn_style()
            } else {
                app.theme.dim_style()
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

    let date_offset = date_state.offset();
    let visible_dates = dates
        .len()
        .saturating_sub(date_offset)
        .min(date_area.height as usize);
    for i in 0..visible_dates {
        let date_idx = date_offset + i;
        let row_y = date_area.y + i as u16;
        app.hit_areas.push(HitArea {
            rect: Rect::new(date_area.x, row_y, date_area.width, 1),
            target: HitTarget::HistoryDate { index: date_idx },
        });
    }

    // 2. Draw Thin Vertical Divider
    let divider_line = format!("{}\n", app.symbols.v_divider).repeat(area.height as usize);
    frame.render_widget(
        Paragraph::new(divider_line).style(app.theme.divider_style()),
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
        // Phase 5: Header progress count uses top-level tasks only
        let date_top: Vec<&Todo> = date_todos
            .iter()
            .copied()
            .filter(|t| t.parent.is_none())
            .collect();
        let total = date_top.len();
        let done = date_top.iter().filter(|t| t.done).count();

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
        let mut heading_spans = vec![Span::styled(
            full_date_str,
            app.theme.text_style().add_modifier(Modifier::BOLD),
        )];
        if app.config.progress.show_count {
            heading_spans.push(Span::styled(
                format!("   {}   ", app.symbols.bullet),
                app.theme.dim_style(),
            ));
            heading_spans.push(Span::styled(
                format!("{done} of {total} done"),
                app.theme.dim_style(),
            ));
        }
        let heading_line = Line::from(heading_spans);
        frame.render_widget(Paragraph::new(heading_line), right_rows[0]);

        if date_todos.is_empty() {
            frame.render_widget(
                Paragraph::new(Span::styled(
                    "No todos recorded for this date.",
                    app.theme.dim_style(),
                )),
                right_rows[2],
            );
        } else {
            let is_date_overdue = date < today;
            let rows = app.visible_rows_history();

            let show_detail = (app.show_detail_pane
                || matches!(app.input_mode, crate::model::InputMode::EditingNote(_)))
                && app.terminal_height >= 20;

            let (tasks_list_area, divider_area, detail_area) =
                if show_detail && right_rows[2].height >= 8 {
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Min(4),
                            Constraint::Length(1),
                            Constraint::Length(6.min(right_rows[2].height.saturating_sub(5))),
                        ])
                        .split(right_rows[2]);
                    (chunks[0], Some(chunks[1]), Some(chunks[2]))
                } else {
                    (right_rows[2], None, None)
                };

            let items: Vec<ListItem> = rows
                .iter()
                .enumerate()
                .filter_map(|(idx, row)| {
                    let todo = app.todos.iter().find(|t| t.id == row.id)?;
                    let is_selected = idx == app.history_todo_index;
                    let active_marker = is_selected && is_todo_pane_active;
                    // Phase 5: Overdue markers use top-level unfinished tasks only
                    let is_todo_overdue = is_date_overdue && !todo.done && todo.parent.is_none();
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

                    let subtask_progress = if row.depth == 0 {
                        let subs: Vec<&Todo> = app
                            .todos
                            .iter()
                            .filter(|t| t.parent == Some(todo.id))
                            .collect();
                        if !subs.is_empty() {
                            let done_subs = subs.iter().filter(|t| t.done).count();
                            Some((done_subs, subs.len()))
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    let is_pinned = app.focus == Some(todo.id);
                    let is_marked = app.marks.contains(&todo.id);
                    Some(format_todo_row_history(
                        todo,
                        row.depth,
                        subtask_progress,
                        is_selected,
                        active_marker,
                        is_marked,
                        is_todo_overdue,
                        is_pinned,
                        tasks_list_area.width as usize,
                        app.terminal_width,
                        app.config.tasks.show_tags,
                        time_str.as_deref(),
                        &app.theme,
                        &app.symbols,
                    ))
                })
                .collect();

            let list = List::new(items);
            let mut todo_state = ListState::default();
            todo_state.select(Some(app.history_todo_index));
            frame.render_stateful_widget(list, tasks_list_area, &mut todo_state);

            let todo_offset = todo_state.offset();
            let visible_todos = rows
                .len()
                .saturating_sub(todo_offset)
                .min(tasks_list_area.height as usize);
            let marker_len = app.symbols.marker.chars().count() as u16;
            let checkbox_len = app.symbols.check_done.chars().count() as u16;
            for i in 0..visible_todos {
                let todo_idx = todo_offset + i;
                let row_y = tasks_list_area.y + i as u16;
                let connector_len = if rows[todo_idx].depth > 0 {
                    app.symbols.subtask_connector.chars().count() as u16
                } else {
                    0
                };
                app.hit_areas.push(HitArea {
                    rect: Rect::new(
                        tasks_list_area.x + marker_len + connector_len,
                        row_y,
                        checkbox_len,
                        1,
                    ),
                    target: HitTarget::HistoryCheckbox { index: todo_idx },
                });
                app.hit_areas.push(HitArea {
                    rect: Rect::new(tasks_list_area.x, row_y, tasks_list_area.width, 1),
                    target: HitTarget::HistoryTask { index: todo_idx },
                });
            }

            if let (Some(div), Some(det)) = (divider_area, detail_area) {
                draw_detail_pane(app, frame, det, div, app.history_todo_id);
            }
        }
    }
}

/// Upcoming view: Future Dates column + thin vertical rule + Todo list for selected date
fn draw_upcoming(app: &mut App, frame: &mut Frame, area: Rect) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let dates = app.upcoming_dates_list();
    let today = app.today;

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

    app.hit_areas.push(HitArea {
        rect: date_area,
        target: HitTarget::DateArea,
    });
    app.hit_areas.push(HitArea {
        rect: todo_area,
        target: HitTarget::TodoArea,
    });

    let is_date_pane_active = app.upcoming_pane == HistoryPane::Dates;
    let is_todo_pane_active = app.upcoming_pane == HistoryPane::Todos;

    if dates.is_empty() {
        frame.render_widget(
            Paragraph::new(Span::styled("No upcoming tasks", app.theme.dim_style())),
            date_area,
        );
        let divider_line = format!("{}\n", app.symbols.v_divider).repeat(area.height as usize);
        frame.render_widget(
            Paragraph::new(divider_line).style(app.theme.divider_style()),
            divider_area,
        );
        return;
    }

    // 1. Draw Left Date Column
    let date_items: Vec<ListItem> = dates
        .iter()
        .enumerate()
        .map(|(idx, &date)| {
            let is_selected = idx == app.upcoming_date_index;
            let date_todos: Vec<&Todo> = app
                .todos
                .iter()
                .filter(|t| t.created_date == date)
                .collect();
            // Phase 5: Progress count uses top-level tasks only
            let date_top: Vec<&Todo> = date_todos
                .iter()
                .copied()
                .filter(|t| t.parent.is_none())
                .collect();
            let total = date_top.len();
            let done = date_top.iter().filter(|t| t.done).count();

            let marker = if is_selected {
                if is_date_pane_active {
                    Span::styled(
                        app.symbols.marker,
                        app.theme.accent_style().add_modifier(Modifier::BOLD),
                    )
                } else {
                    Span::styled(app.symbols.marker, app.theme.dim_style())
                }
            } else {
                Span::raw(" ".repeat(app.symbols.marker.chars().count()))
            };

            let friendly_date = format_friendly_date(date, today, &app.config.short_date_format);
            let count_str = format!("{done}/{total}");

            let marker_width = app.symbols.marker.chars().count();
            let text_space =
                (date_area.width as usize).saturating_sub(marker_width + count_str.len());
            let date_truncated = truncate_text(
                &friendly_date,
                text_space.saturating_sub(1),
                app.symbols.ellipsis,
            );
            let pad_spaces = text_space.saturating_sub(date_truncated.chars().count());

            let date_style = if is_selected && is_date_pane_active {
                app.theme.text_style().add_modifier(Modifier::BOLD)
            } else {
                app.theme.text_style()
            };

            let count_style = app.theme.dim_style();

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
        date_state.select(Some(app.upcoming_date_index));
    }
    frame.render_stateful_widget(date_list, date_area, &mut date_state);

    let date_offset = date_state.offset();
    let visible_dates = dates
        .len()
        .saturating_sub(date_offset)
        .min(date_area.height as usize);
    for i in 0..visible_dates {
        let date_idx = date_offset + i;
        let row_y = date_area.y + i as u16;
        app.hit_areas.push(HitArea {
            rect: Rect::new(date_area.x, row_y, date_area.width, 1),
            target: HitTarget::UpcomingDate { index: date_idx },
        });
    }

    // 2. Draw Thin Vertical Divider
    let divider_line = format!("{}\n", app.symbols.v_divider).repeat(area.height as usize);
    frame.render_widget(
        Paragraph::new(divider_line).style(app.theme.divider_style()),
        divider_area,
    );

    // 3. Draw Right Todo Pane
    let selected_date = dates.get(app.upcoming_date_index).copied();
    if let Some(date) = selected_date {
        let date_todos: Vec<&Todo> = app
            .todos
            .iter()
            .filter(|t| t.created_date == date)
            .collect();
        // Phase 5: Header progress count uses top-level tasks only
        let date_top: Vec<&Todo> = date_todos
            .iter()
            .copied()
            .filter(|t| t.parent.is_none())
            .collect();
        let total = date_top.len();
        let done = date_top.iter().filter(|t| t.done).count();

        let right_rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Heading row
                Constraint::Length(1), // Blank line spacing
                Constraint::Min(0),    // Todos list
            ])
            .split(todo_area);

        let full_date_str = date.format(&app.config.date_format).to_string();
        let mut heading_spans = vec![Span::styled(
            full_date_str,
            app.theme.text_style().add_modifier(Modifier::BOLD),
        )];
        if app.config.progress.show_count {
            heading_spans.push(Span::styled(
                format!("   {}   ", app.symbols.bullet),
                app.theme.dim_style(),
            ));
            heading_spans.push(Span::styled(
                format!("{done} of {total} done"),
                app.theme.dim_style(),
            ));
        }
        let heading_line = Line::from(heading_spans);
        frame.render_widget(Paragraph::new(heading_line), right_rows[0]);

        if date_todos.is_empty() {
            frame.render_widget(
                Paragraph::new(Span::styled(
                    "No todos scheduled for this date.",
                    app.theme.dim_style(),
                )),
                right_rows[2],
            );
        } else {
            let rows = app.visible_rows_upcoming();

            let show_detail = (app.show_detail_pane
                || matches!(app.input_mode, crate::model::InputMode::EditingNote(_)))
                && app.terminal_height >= 20;

            let (tasks_list_area, divider_area, detail_area) =
                if show_detail && right_rows[2].height >= 8 {
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Min(4),
                            Constraint::Length(1),
                            Constraint::Length(6.min(right_rows[2].height.saturating_sub(5))),
                        ])
                        .split(right_rows[2]);
                    (chunks[0], Some(chunks[1]), Some(chunks[2]))
                } else {
                    (right_rows[2], None, None)
                };

            let items: Vec<ListItem> = rows
                .iter()
                .enumerate()
                .filter_map(|(idx, row)| {
                    let todo = app.todos.iter().find(|t| t.id == row.id)?;
                    let is_selected = idx == app.upcoming_todo_index;
                    let active_marker = is_selected && is_todo_pane_active;
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

                    let subtask_progress = if row.depth == 0 {
                        let subs: Vec<&Todo> = app
                            .todos
                            .iter()
                            .filter(|t| t.parent == Some(todo.id))
                            .collect();
                        if !subs.is_empty() {
                            let done_subs = subs.iter().filter(|t| t.done).count();
                            Some((done_subs, subs.len()))
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    let is_pinned = app.focus == Some(todo.id);
                    let is_marked = app.marks.contains(&todo.id);
                    Some(format_todo_row_history(
                        todo,
                        row.depth,
                        subtask_progress,
                        is_selected,
                        active_marker,
                        is_marked,
                        false,
                        is_pinned,
                        tasks_list_area.width as usize,
                        app.terminal_width,
                        app.config.tasks.show_tags,
                        time_str.as_deref(),
                        &app.theme,
                        &app.symbols,
                    ))
                })
                .collect();

            let list = List::new(items);
            let mut todo_state = ListState::default();
            todo_state.select(Some(app.upcoming_todo_index));
            frame.render_stateful_widget(list, tasks_list_area, &mut todo_state);

            let todo_offset = todo_state.offset();
            let visible_todos = rows
                .len()
                .saturating_sub(todo_offset)
                .min(tasks_list_area.height as usize);
            let marker_len = app.symbols.marker.chars().count() as u16;
            let checkbox_len = app.symbols.check_done.chars().count() as u16;
            for i in 0..visible_todos {
                let todo_idx = todo_offset + i;
                let row_y = tasks_list_area.y + i as u16;
                let connector_len = if rows[todo_idx].depth > 0 {
                    app.symbols.subtask_connector.chars().count() as u16
                } else {
                    0
                };
                app.hit_areas.push(HitArea {
                    rect: Rect::new(
                        tasks_list_area.x + marker_len + connector_len,
                        row_y,
                        checkbox_len,
                        1,
                    ),
                    target: HitTarget::UpcomingCheckbox { index: todo_idx },
                });
                app.hit_areas.push(HitArea {
                    rect: Rect::new(tasks_list_area.x, row_y, tasks_list_area.width, 1),
                    target: HitTarget::UpcomingTask { index: todo_idx },
                });
            }

            if let (Some(div), Some(det)) = (divider_area, detail_area) {
                draw_detail_pane(app, frame, det, div, app.upcoming_todo_id);
            }
        }
    }
}

fn draw_summary(app: &mut App, frame: &mut Frame, area: Rect) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    match app.summary_range {
        crate::config::SummaryRange::Week => draw_summary_week(app, frame, area),
        crate::config::SummaryRange::Month => draw_summary_month(app, frame, area),
    }
}

fn draw_summary_week(app: &mut App, frame: &mut Frame, area: Rect) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let week_start = crate::dates::week_start_of(app.summary_date, app.config.week_start);
    let week_days = crate::dates::week_days(week_start);
    let start_str = week_days[0].format(&app.config.short_date_format);
    let end_str = week_days[6].format(&app.config.short_date_format);
    let dash = if app.symbols.is_ascii { "-" } else { "–" };
    let header_text = format!("Week of {start_str} {dash} {end_str}");

    let mut lines = Vec::new();

    // 1. Header line
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(
            header_text,
            app.theme.subtext_style().add_modifier(Modifier::BOLD),
        ),
    ]));

    // 2. Divider line
    let divider_width = (area.width as usize).saturating_sub(4);
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(
            app.symbols.h_rule.repeat(divider_width),
            app.theme.divider_style(),
        ),
    ]));

    // 3. 7 day rows
    let day_sums = crate::stats::day_summaries(&app.todos, week_days);

    for (i, &d) in week_days.iter().enumerate() {
        let is_cursor = d == app.summary_date;
        let is_today = d == app.today;

        let marker = if is_cursor {
            Span::styled(
                app.symbols.marker,
                app.theme.accent_style().add_modifier(Modifier::BOLD),
            )
        } else {
            Span::raw("  ")
        };

        let date_str = d.format(&app.config.short_date_format).to_string();
        let date_padded = format!("{:<11}", date_str);
        let date_span = Span::styled(
            date_padded,
            if is_today {
                app.theme.today_style()
            } else {
                app.theme.dim_style()
            },
        );

        let sum = day_sums.iter().find(|s| s.date == d);
        let total = sum.map(|s| s.total).unwrap_or(0);
        let done = sum.map(|s| s.done).unwrap_or(0);

        let mut row_spans = vec![marker, date_span, Span::raw("   ")];

        if total == 0 {
            let empty_dash = if app.symbols.is_ascii { "-" } else { "–" };
            row_spans.push(Span::styled(empty_dash, app.theme.dim_style()));
        } else if d > app.today {
            row_spans.push(Span::styled(
                format!("{total} scheduled"),
                app.theme.dim_style(),
            ));
        } else {
            let bar_w = (app.config.progress.width as usize)
                .min((area.width as usize).saturating_sub(30))
                .max(1);
            let bar_style = if app.symbols.is_ascii {
                crate::config::ProgressStyle::Ascii
            } else {
                app.config.progress.style
            };
            let bar_spans =
                render_progress_bar(done, total, bar_w, bar_style, &app.theme, &app.symbols);
            row_spans.extend(bar_spans);
            row_spans.push(Span::styled(
                format!("   {done}/{total}"),
                app.theme.dim_style(),
            ));
        }

        lines.push(Line::from(row_spans));

        let row_y = area.y + 2 + i as u16;
        if row_y < area.bottom() {
            app.hit_areas.push(HitArea {
                rect: Rect::new(area.x, row_y, area.width, 1),
                target: HitTarget::SummaryDay { date: d },
            });
        }
    }

    // 4. Total line
    let p_sum = crate::stats::period_summary(&day_sums);
    let total_text = format!(
        "Week  {} of {} done {} {}%",
        p_sum.done, p_sum.total, app.symbols.dot, p_sum.percent
    );
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(total_text, app.theme.dim_style()),
    ]));

    frame.render_widget(Paragraph::new(lines), area);
}

fn draw_summary_month(app: &mut App, frame: &mut Frame, area: Rect) {
    draw_summary_week(app, frame, area);
}

pub fn build_highlighted_text_spans(
    display_text: &str,
    highlight_ranges: &[(usize, usize)],
    is_done: bool,
    avail_width: usize,
    ellipsis: &str,
    theme: &Theme,
) -> Vec<Span<'static>> {
    if avail_width == 0 {
        return Vec::new();
    }
    let chars: Vec<char> = display_text.chars().collect();
    let ellipsis_chars: Vec<char> = ellipsis.chars().collect();
    let ellipsis_len = ellipsis_chars.len();

    let (visible_chars, show_ellipsis) = if chars.len() > avail_width {
        let keep_len = avail_width.saturating_sub(ellipsis_len);
        (&chars[..keep_len], true)
    } else {
        (&chars[..], false)
    };

    let base_style = if is_done {
        theme.dim_style().add_modifier(Modifier::CROSSED_OUT)
    } else {
        theme.text_style()
    };
    let match_style = if is_done {
        theme
            .search_match_style()
            .add_modifier(Modifier::CROSSED_OUT)
    } else {
        theme.search_match_style()
    };

    let mut spans = Vec::new();
    let mut current_segment = String::new();
    let mut current_is_match: Option<bool> = None;

    for (i, &ch) in visible_chars.iter().enumerate() {
        let is_match = highlight_ranges
            .iter()
            .any(|&(start, end)| i >= start && i < end);

        match current_is_match {
            Some(cur) if cur == is_match => {
                current_segment.push(ch);
            }
            Some(cur) => {
                if !current_segment.is_empty() {
                    spans.push(Span::styled(
                        current_segment,
                        if cur { match_style } else { base_style },
                    ));
                    current_segment = String::new();
                }
                current_segment.push(ch);
                current_is_match = Some(is_match);
            }
            None => {
                current_segment.push(ch);
                current_is_match = Some(is_match);
            }
        }
    }

    if !current_segment.is_empty() {
        let is_match = current_is_match.unwrap_or(false);
        spans.push(Span::styled(
            current_segment,
            if is_match { match_style } else { base_style },
        ));
    }

    if show_ellipsis && ellipsis_len > 0 {
        spans.push(Span::styled(ellipsis.to_string(), base_style));
    }

    spans
}

fn draw_search_results(app: &mut App, frame: &mut Frame, area: Rect) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    if app.input_buffer.is_empty() {
        let (task_count, day_count) =
            crate::search::search_stats(&app.todos, app.today, app.config.ui.show_history);
        let hint = if app.config.ui.show_history {
            format!(
                "type to search {} {task_count} tasks across {day_count} days",
                app.symbols.dot
            )
        } else {
            format!(
                "type to search {} {task_count} tasks across {day_count} days (history hidden)",
                app.symbols.dot
            )
        };
        let p = Paragraph::new(Line::from(vec![
            Span::raw("  "),
            Span::styled(hint, app.theme.dim_style()),
        ]));
        frame.render_widget(p, area);
        return;
    }

    if app.search_results.hits.is_empty() {
        let p = Paragraph::new(Line::from(vec![
            Span::raw("  "),
            Span::styled("No matching tasks", app.theme.dim_style()),
        ]));
        frame.render_widget(p, area);
        return;
    }

    let hits = &app.search_results.hits;
    let total_hits = app.search_results.total_hits;
    let marker_len = app.symbols.marker.chars().count();
    let max_width = area.width as usize;

    let mut items: Vec<ListItem> = Vec::with_capacity(hits.len() + 1);

    for (idx, hit) in hits.iter().enumerate() {
        let is_selected = idx == app.search_selected_index;
        let marker = if is_selected {
            Span::styled(
                app.symbols.marker,
                app.theme.accent_style().add_modifier(Modifier::BOLD),
            )
        } else {
            Span::raw(" ".repeat(marker_len))
        };

        let date_str = format_friendly_date(hit.date, app.today, &app.config.short_date_format);
        let date_col_width = 11;
        let date_padded = format!("{:<width$}", date_str, width = date_col_width);
        let date_span = Span::styled(format!("{date_padded} "), app.theme.dim_style());
        let date_total_len = date_col_width + 1;

        let checkbox = if hit.done {
            Span::styled(
                app.symbols.check_done,
                Style::default().fg(app.theme.accent),
            )
        } else {
            Span::styled(app.symbols.check_todo, app.theme.dim_style())
        };
        let check_space = Span::raw(" ");
        let checkbox_total_len = app.symbols.check_done.chars().count() + 1;

        let note_span = if hit.match_in_note_only {
            Some(Span::styled(
                format!(" {}", app.symbols.note),
                app.theme.dim_style(),
            ))
        } else {
            None
        };
        let note_len = if note_span.is_some() {
            app.symbols.note.chars().count() + 1
        } else {
            0
        };

        let can_show_tags =
            app.config.tasks.show_tags && app.terminal_width >= 60 && !hit.tags.is_empty();
        let (tags_str, tags_len) = if can_show_tags {
            let s = format!(
                " {}",
                hit.tags
                    .iter()
                    .map(|t| format!("#{t}"))
                    .collect::<Vec<_>>()
                    .join(" ")
            );
            let len = s.chars().count();
            (s, len)
        } else {
            (String::new(), 0)
        };

        let fixed_len = marker_len + date_total_len + checkbox_total_len + note_len;
        let avail_total = max_width.saturating_sub(fixed_len);

        let (avail_text, show_tags) = if tags_len > 0 && avail_total > tags_len + 4 {
            (avail_total.saturating_sub(tags_len), true)
        } else {
            (avail_total, false)
        };

        let mut spans = vec![marker, date_span, checkbox, check_space];
        let text_spans = build_highlighted_text_spans(
            &hit.display_text,
            &hit.highlight_ranges,
            hit.done,
            avail_text,
            app.symbols.ellipsis,
            &app.theme,
        );
        spans.extend(text_spans);

        if let Some(note_s) = note_span {
            spans.push(note_s);
        }

        if show_tags && !tags_str.is_empty() {
            spans.push(Span::styled(tags_str, app.theme.dim_style()));
        }

        items.push(ListItem::new(Line::from(spans)));
    }

    if total_hits > hits.len() {
        let more_count = total_hits - hits.len();
        let indent = marker_len + 12;
        let more_spans = vec![
            Span::raw(" ".repeat(indent)),
            Span::styled(format!("+{more_count} more"), app.theme.dim_style()),
        ];
        items.push(ListItem::new(Line::from(more_spans)));
    }

    let list = List::new(items);
    let mut list_state = ListState::default();
    list_state.select(Some(app.search_selected_index));
    frame.render_stateful_widget(list, area, &mut list_state);

    let offset = list_state.offset();
    let visible_count = hits.len().saturating_sub(offset).min(area.height as usize);
    for i in 0..visible_count {
        let item_idx = offset + i;
        if item_idx < hits.len() {
            app.hit_areas.push(crate::ui::HitArea {
                target: crate::ui::HitTarget::SearchResult { index: item_idx },
                rect: Rect::new(area.x, area.y + i as u16, area.width, 1),
            });
        }
    }
}

/// Single line modern prompt at the bottom: "› Text typed...       enter save · esc cancel"
fn draw_input(app: &App, frame: &mut Frame, area: Rect) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    if let crate::model::InputMode::ConfirmDeleteRecurring(_) = app.input_mode {
        let prompt_text = "Delete (t)his instance or (a)ll (template + future)?";
        let right_text = "esc cancel";
        let prompt_len = prompt_text.chars().count();
        let right_len = right_text.chars().count();
        let pad = (area.width as usize).saturating_sub(prompt_len + right_len);
        let spans = vec![
            Span::styled(
                prompt_text,
                app.theme.warn_style().add_modifier(Modifier::BOLD),
            ),
            Span::raw(" ".repeat(pad)),
            Span::styled(right_text, app.theme.dim_style()),
        ];
        frame.render_widget(Paragraph::new(Line::from(spans)), area);
        return;
    }

    let prompt_label = match app.input_mode {
        crate::model::InputMode::Editing(_) => app.symbols.prompt_edit,
        crate::model::InputMode::AddingSubtask(_) => app.symbols.prompt_subtask,
        crate::model::InputMode::Rescheduling(_) | crate::model::InputMode::ReschedulingBulk => {
            app.symbols.prompt_reschedule
        }
        crate::model::InputMode::Filtering => app.symbols.prompt_filter,
        crate::model::InputMode::SettingRecurrence(_) => app.symbols.prompt_recurrence,
        crate::model::InputMode::Searching => app.symbols.prompt_search,
        crate::model::InputMode::JumpToDate => app.symbols.prompt_jump,
        _ => app.symbols.prompt_default,
    };

    let prompt_symbol = Span::styled(
        prompt_label,
        app.theme.accent_style().add_modifier(Modifier::BOLD),
    );

    let (right_text, right_style) = if let Some(ref err) = app.prompt_error {
        (err.clone(), app.theme.dim_style())
    } else if app.input_mode == crate::model::InputMode::Searching {
        (
            format!("enter open  {}  esc cancel", app.symbols.dot),
            app.theme.dim_style(),
        )
    } else if app.input_mode == crate::model::InputMode::JumpToDate {
        (
            format!("enter jump  {}  esc cancel", app.symbols.dot),
            app.theme.dim_style(),
        )
    } else {
        (
            format!("enter save  {}  esc cancel", app.symbols.dot),
            app.theme.dim_style(),
        )
    };

    let hints_len = right_text.chars().count();
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
            crate::model::InputMode::Rescheduling(_)
            | crate::model::InputMode::ReschedulingBulk => "tomorrow, fri, +3d, YYYY-MM-DD",
            crate::model::InputMode::Filtering => "#tag, ! (empty clears)",
            crate::model::InputMode::SettingRecurrence(_) => "daily, weekdays, weekly, off",
            crate::model::InputMode::Searching => "",
            crate::model::InputMode::JumpToDate => "today, +3d, fri, YYYY-MM-DD",
            _ => "What needs doing today?",
        };
        spans.push(Span::styled(placeholder, app.theme.dim_style()));
        let placeholder_len = placeholder.chars().count();
        let pad = (area.width as usize).saturating_sub(prompt_len + placeholder_len + hints_len);
        spans.push(Span::raw(" ".repeat(pad)));
    } else {
        spans.push(Span::styled(&visible_input, app.theme.text_style()));
        let pad = (area.width as usize)
            .saturating_sub(prompt_len + visible_input.chars().count() + hints_len);
        spans.push(Span::raw(" ".repeat(pad)));
    }

    spans.push(Span::styled(right_text, right_style));

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
        let sanitized = if app.symbols.is_ascii {
            err.replace('·', "*")
        } else {
            err.clone()
        };
        let error_line = Line::from(vec![
            Span::styled(
                "save error: ",
                app.theme.error_style().add_modifier(Modifier::BOLD),
            ),
            Span::styled(sanitized, app.theme.error_style()),
        ]);
        frame.render_widget(Paragraph::new(error_line), area);
        return;
    }

    if let Some(ref msg) = app.transient_message {
        let sanitized = if app.symbols.is_ascii {
            msg.replace('·', "*")
        } else {
            msg.clone()
        };
        let msg_line = Line::from(vec![Span::styled(sanitized, app.theme.subtext_style())]);
        frame.render_widget(Paragraph::new(msg_line), area);
        return;
    }

    if !app.marks.is_empty() {
        let count = app.marks.len();
        let toggle_hint = app.keymap.hint(crate::keymap::Action::Toggle);
        let del_hint = app.keymap.hint(crate::keymap::Action::Delete);
        let move_hint = app.keymap.hint(crate::keymap::Action::MoveToToday);
        let resched_hint = app.keymap.hint(crate::keymap::Action::Reschedule);
        let hints = [
            (format!("{count} marked"), ""),
            (toggle_hint, "toggle"),
            (del_hint, "delete"),
            (move_hint, "today"),
            (resched_hint, "reschedule"),
            ("esc".to_string(), "unmark"),
        ];
        let mut spans: Vec<Span> = Vec::new();
        for (i, (key, action)) in hints.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled(
                    format!("  {}  ", app.symbols.dot),
                    app.theme.divider_style(),
                ));
            }
            if action.is_empty() {
                spans.push(Span::styled(
                    key.clone(),
                    app.theme.accent_style().add_modifier(Modifier::BOLD),
                ));
            } else {
                spans.push(Span::styled(key.clone(), app.theme.subtext_style()));
                spans.push(Span::raw(" "));
                spans.push(Span::styled(*action, app.theme.dim_style()));
            }
        }
        frame.render_widget(Paragraph::new(Line::from(spans)), area);
        return;
    }

    let tab_hint = app.keymap.hint(crate::keymap::Action::SwitchView);
    let help_hint = app.keymap.hint(crate::keymap::Action::Help);
    let quit_hint = app.keymap.hint(crate::keymap::Action::Quit);
    let done_hint = app.keymap.hint(crate::keymap::Action::Toggle);

    let hints: Vec<(String, &str)> = match app.current_view {
        View::Today => {
            let mut list = vec![
                (app.keymap.hint(crate::keymap::Action::Add), "add"),
                (done_hint, "done"),
            ];
            list.push((
                app.keymap.hint(crate::keymap::Action::Reschedule),
                "reschedule",
            ));
            if app.config.ui.show_upcoming {
                list.push((tab_hint, "upcoming"));
            } else if app.config.ui.show_history {
                list.push((tab_hint, "history"));
            }
            list.push((help_hint, "help"));
            list.push((quit_hint, "quit"));
            list
        }
        View::Upcoming => match app.upcoming_pane {
            HistoryPane::Dates => vec![
                (app.keymap.hint(crate::keymap::Action::FocusTasks), "tasks"),
                (tab_hint, "history"),
                (help_hint, "help"),
                (quit_hint, "quit"),
            ],
            HistoryPane::Todos => vec![
                (app.keymap.hint(crate::keymap::Action::FocusDates), "dates"),
                (done_hint, "done"),
                (
                    app.keymap.hint(crate::keymap::Action::Reschedule),
                    "reschedule",
                ),
                (tab_hint, "history"),
                (help_hint, "help"),
                (quit_hint, "quit"),
            ],
        },
        View::History => match app.history_pane {
            HistoryPane::Dates => vec![
                (app.keymap.hint(crate::keymap::Action::FocusTasks), "tasks"),
                (tab_hint, "today"),
                (help_hint, "help"),
                (quit_hint, "quit"),
            ],
            HistoryPane::Todos => vec![
                (app.keymap.hint(crate::keymap::Action::FocusDates), "dates"),
                (done_hint, "done"),
                (
                    app.keymap.hint(crate::keymap::Action::Reschedule),
                    "reschedule",
                ),
                (tab_hint, "today"),
                (help_hint, "help"),
                (quit_hint, "quit"),
            ],
        },
        View::Summary => vec![
            (
                app.keymap.hint(crate::keymap::Action::ToggleSummaryRange),
                "week/month",
            ),
            ("enter".to_string(), "open"),
            (app.keymap.hint(crate::keymap::Action::Add), "add today"),
            (tab_hint, "today"),
            (help_hint, "help"),
            (quit_hint, "quit"),
        ],
    };

    let mut spans: Vec<Span> = Vec::new();
    for (i, (key, action)) in hints.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(
                format!("  {}  ", app.symbols.dot),
                app.theme.divider_style(),
            ));
        }
        spans.push(Span::styled(key.clone(), app.theme.subtext_style()));
        spans.push(Span::raw(" "));
        spans.push(Span::styled(*action, app.theme.dim_style()));
    }

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

#[allow(clippy::too_many_arguments)]
fn format_todo_row(
    todo: &Todo,
    depth: usize,
    subtask_progress: Option<(usize, usize)>,
    is_selected: bool,
    is_marked: bool,
    is_pinned: bool,
    max_width: usize,
    terminal_width: u16,
    show_tags: bool,
    theme: &Theme,
    symbols: &Symbols,
) -> ListItem<'static> {
    let marker_len = symbols.marker.chars().count();
    let marker = if is_marked {
        let style = if is_selected {
            theme
                .accent_style()
                .add_modifier(Modifier::BOLD | Modifier::REVERSED)
        } else {
            theme.accent_style()
        };
        Span::styled(symbols.mark, style)
    } else if is_selected {
        Span::styled(
            symbols.marker,
            theme.accent_style().add_modifier(Modifier::BOLD),
        )
    } else {
        Span::raw(" ".repeat(marker_len))
    };

    let connector_len = if depth > 0 {
        symbols.subtask_connector.chars().count()
    } else {
        0
    };

    let checkbox = if todo.done {
        Span::styled(symbols.check_done, Style::default().fg(theme.accent))
    } else {
        Span::styled(symbols.check_todo, theme.dim_style())
    };

    let (prio_str, prio_len, prio_style) = match todo.priority {
        1 => {
            if todo.done {
                ("! ", 2, theme.dim_style())
            } else {
                ("! ", 2, theme.priority_1_style())
            }
        }
        2 => {
            if todo.done {
                ("!! ", 3, theme.dim_style())
            } else {
                ("!! ", 3, theme.priority_2_style())
            }
        }
        _ => ("", 0, Style::default()),
    };

    let pin_len = if is_pinned {
        symbols.focus_marker.chars().count()
    } else {
        0
    };

    let is_recurring = todo.recurring_id.is_some();
    let repeat_len = if is_recurring {
        symbols.repeat.chars().count()
    } else {
        0
    };

    let checkbox_len = symbols.check_done.chars().count();
    let has_note = todo.note.as_ref().is_some_and(|n| !n.trim().is_empty());
    let note_len = if has_note { 2 } else { 0 };

    let (prog_str, prog_len) = if let Some((done, total)) = subtask_progress {
        let s = format!(" {done}/{total}");
        let len = s.chars().count();
        (s, len)
    } else {
        (String::new(), 0)
    };

    let avail_total = max_width.saturating_sub(
        marker_len
            + connector_len
            + checkbox_len
            + pin_len
            + repeat_len
            + prio_len
            + prog_len
            + note_len
            + 1,
    );

    let can_show_tags = show_tags && terminal_width >= 60 && !todo.tags.is_empty();
    let (tags_str, tags_len) = if can_show_tags {
        let s = format!(
            " {}",
            todo.tags
                .iter()
                .map(|t| format!("#{t}"))
                .collect::<Vec<_>>()
                .join(" ")
        );
        let len = s.chars().count();
        (s, len)
    } else {
        (String::new(), 0)
    };

    let (text_truncated, show_rendered_tags) = if tags_len > 0 && avail_total > tags_len + 2 {
        let avail_text = avail_total.saturating_sub(tags_len);
        (
            truncate_text(&todo.text, avail_text, symbols.ellipsis),
            true,
        )
    } else {
        (
            truncate_text(&todo.text, avail_total, symbols.ellipsis),
            false,
        )
    };

    let text_span = if todo.done {
        Span::styled(
            text_truncated,
            theme.dim_style().add_modifier(Modifier::CROSSED_OUT),
        )
    } else {
        Span::styled(text_truncated, theme.text_style())
    };

    let mut spans = vec![marker];
    if depth > 0 {
        spans.push(Span::styled(symbols.subtask_connector, theme.dim_style()));
    }
    spans.push(checkbox);
    if is_pinned {
        spans.push(Span::styled(symbols.focus_marker, theme.dim_style()));
    }
    if is_recurring {
        spans.push(Span::styled(symbols.repeat, theme.dim_style()));
    }
    if prio_len > 0 {
        spans.push(Span::styled(prio_str, prio_style));
    }
    spans.push(text_span);
    if prog_len > 0 {
        spans.push(Span::styled(prog_str, theme.dim_style()));
    }
    if show_rendered_tags {
        spans.push(Span::styled(tags_str, theme.dim_style()));
    }
    if has_note {
        spans.push(Span::styled(
            format!(" {}", symbols.note),
            theme.dim_style(),
        ));
    }

    ListItem::new(Line::from(spans))
}

#[allow(clippy::too_many_arguments)]
fn format_todo_row_history(
    todo: &Todo,
    depth: usize,
    subtask_progress: Option<(usize, usize)>,
    is_selected: bool,
    is_active: bool,
    is_marked: bool,
    is_overdue: bool,
    is_pinned: bool,
    max_width: usize,
    terminal_width: u16,
    show_tags: bool,
    time_str: Option<&str>,
    theme: &Theme,
    symbols: &Symbols,
) -> ListItem<'static> {
    let marker_len = symbols.marker.chars().count();
    let marker = if is_marked {
        let style = if is_selected {
            theme
                .accent_style()
                .add_modifier(Modifier::BOLD | Modifier::REVERSED)
        } else {
            theme.accent_style()
        };
        Span::styled(symbols.mark, style)
    } else if is_selected {
        if is_active {
            Span::styled(
                symbols.marker,
                theme.accent_style().add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled(symbols.marker, theme.dim_style())
        }
    } else {
        Span::raw(" ".repeat(marker_len))
    };

    let connector_len = if depth > 0 {
        symbols.subtask_connector.chars().count()
    } else {
        0
    };

    let checkbox = if todo.done {
        Span::styled(symbols.check_done, Style::default().fg(theme.accent))
    } else if is_overdue {
        Span::styled(symbols.check_overdue, theme.warn_style())
    } else {
        Span::styled(symbols.check_todo, theme.dim_style())
    };

    let (prio_str, prio_len, prio_style) = match todo.priority {
        1 => {
            if todo.done {
                ("! ", 2, theme.dim_style())
            } else {
                ("! ", 2, theme.priority_1_style())
            }
        }
        2 => {
            if todo.done {
                ("!! ", 3, theme.dim_style())
            } else {
                ("!! ", 3, theme.priority_2_style())
            }
        }
        _ => ("", 0, Style::default()),
    };

    let pin_len = if is_pinned {
        symbols.focus_marker.chars().count()
    } else {
        0
    };

    let is_recurring = todo.recurring_id.is_some();
    let repeat_len = if is_recurring {
        symbols.repeat.chars().count()
    } else {
        0
    };

    let checkbox_len = symbols.check_done.chars().count();
    let suffix_str = time_str.unwrap_or("").to_string();
    let suffix_len = if suffix_str.is_empty() {
        0
    } else {
        suffix_str.chars().count() + 2
    };

    let has_note = todo.note.as_ref().is_some_and(|n| !n.trim().is_empty());
    let note_len = if has_note { 2 } else { 0 };

    let (prog_str, prog_len) = if let Some((done, total)) = subtask_progress {
        let s = format!(" {done}/{total}");
        let len = s.chars().count();
        (s, len)
    } else {
        (String::new(), 0)
    };

    let avail_total = max_width.saturating_sub(
        marker_len
            + connector_len
            + checkbox_len
            + pin_len
            + repeat_len
            + prio_len
            + prog_len
            + suffix_len
            + note_len
            + 1,
    );

    let can_show_tags = show_tags && terminal_width >= 60 && !todo.tags.is_empty();
    let (tags_str, tags_len) = if can_show_tags {
        let s = format!(
            " {}",
            todo.tags
                .iter()
                .map(|t| format!("#{t}"))
                .collect::<Vec<_>>()
                .join(" ")
        );
        let len = s.chars().count();
        (s, len)
    } else {
        (String::new(), 0)
    };

    let (text_truncated, show_rendered_tags, actual_tags_len) =
        if tags_len > 0 && avail_total > tags_len + 2 {
            let avail_text = avail_total.saturating_sub(tags_len);
            (
                truncate_text(&todo.text, avail_text, symbols.ellipsis),
                true,
                tags_len,
            )
        } else {
            (
                truncate_text(&todo.text, avail_total, symbols.ellipsis),
                false,
                0,
            )
        };

    let text_span = if todo.done {
        Span::styled(
            text_truncated.clone(),
            theme.dim_style().add_modifier(Modifier::CROSSED_OUT),
        )
    } else {
        Span::styled(text_truncated.clone(), theme.text_style())
    };

    let pad_len = max_width.saturating_sub(
        marker_len
            + connector_len
            + checkbox_len
            + pin_len
            + repeat_len
            + prio_len
            + text_truncated.chars().count()
            + prog_len
            + actual_tags_len
            + note_len
            + suffix_len,
    );

    let mut spans = vec![marker];
    if depth > 0 {
        spans.push(Span::styled(symbols.subtask_connector, theme.dim_style()));
    }
    spans.push(checkbox);
    if is_pinned {
        spans.push(Span::styled(symbols.focus_marker, theme.dim_style()));
    }
    if is_recurring {
        spans.push(Span::styled(symbols.repeat, theme.dim_style()));
    }
    if prio_len > 0 {
        spans.push(Span::styled(prio_str, prio_style));
    }
    spans.push(text_span);
    if prog_len > 0 {
        spans.push(Span::styled(prog_str, theme.dim_style()));
    }
    if show_rendered_tags {
        spans.push(Span::styled(tags_str, theme.dim_style()));
    }
    if has_note {
        spans.push(Span::styled(
            format!(" {}", symbols.note),
            theme.dim_style(),
        ));
    }
    spans.push(Span::raw(" ".repeat(pad_len)));

    if !suffix_str.is_empty() {
        spans.push(Span::styled(suffix_str, theme.dim_style()));
    }

    ListItem::new(Line::from(spans))
}

fn draw_detail_pane(
    app: &App,
    frame: &mut Frame,
    area: Rect,
    divider_area: Rect,
    task_id: Option<u64>,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    // 1. Thin divider rule
    if divider_area.width > 0 && divider_area.height > 0 {
        frame.render_widget(
            Paragraph::new(app.symbols.h_rule.repeat(divider_area.width as usize))
                .style(app.theme.divider_style()),
            divider_area,
        );
    }

    // 2. Note editing mode
    if let crate::model::InputMode::EditingNote(_) = app.input_mode {
        let title_line = Line::from(vec![
            Span::styled(
                "Note ",
                app.theme.accent_style().add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(
                    "(Enter save {} Alt+Enter/Ctrl+j newline {} Esc cancel)",
                    app.symbols.dot, app.symbols.dot
                ),
                app.theme.dim_style(),
            ),
        ]);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Title line
                Constraint::Min(0),    // Note editor lines
            ])
            .split(area);

        frame.render_widget(Paragraph::new(title_line), chunks[0]);

        let body_area = chunks[1];
        if body_area.width > 0 && body_area.height > 0 {
            let chars: Vec<char> = app.note_buffer.chars().collect();
            let mut line_num = 0;
            let mut col_num = 0;
            for (idx, &c) in chars.iter().enumerate() {
                if idx == app.note_cursor {
                    break;
                }
                if c == '\n' {
                    line_num += 1;
                    col_num = 0;
                } else {
                    col_num += 1;
                }
            }

            let lines: Vec<Line> = app
                .note_buffer
                .split('\n')
                .map(|line| Line::from(Span::styled(line.to_string(), app.theme.text_style())))
                .collect();

            frame.render_widget(Paragraph::new(lines), body_area);

            let cursor_x = (body_area.x + col_num as u16).min(body_area.right().saturating_sub(1));
            let cursor_y =
                (body_area.y + line_num as u16).min(body_area.bottom().saturating_sub(1));
            frame.set_cursor_position(ratatui::layout::Position::new(cursor_x, cursor_y));
        }
        return;
    }

    // 3. Normal detail display
    let Some(id) = task_id else {
        return;
    };
    let Some(todo) = app.todos.iter().find(|t| t.id == id) else {
        return;
    };

    let mut lines = Vec::new();

    // Full wrapped text
    lines.push(Line::from(vec![
        Span::styled("Task: ", app.theme.dim_style()),
        Span::styled(
            &todo.text,
            app.theme.text_style().add_modifier(Modifier::BOLD),
        ),
    ]));

    // Metadata line: Date, Priority, Tags, Recurrence, Subtasks
    let mut meta_spans = vec![
        Span::styled("Date: ", app.theme.dim_style()),
        Span::styled(
            todo.created_date
                .format(&app.config.date_format)
                .to_string(),
            app.theme.text_style(),
        ),
    ];

    if todo.priority > 0 {
        meta_spans.push(Span::raw("    "));
        meta_spans.push(Span::styled("Priority: ", app.theme.dim_style()));
        let prio_style = if todo.priority == 2 {
            app.theme.priority_2_style()
        } else {
            app.theme.priority_1_style()
        };
        let prio_str = if todo.priority == 2 { "!!" } else { "!" };
        meta_spans.push(Span::styled(prio_str, prio_style));
    }

    if !todo.tags.is_empty() {
        meta_spans.push(Span::raw("    "));
        meta_spans.push(Span::styled("Tags: ", app.theme.dim_style()));
        meta_spans.push(Span::styled(
            todo.tags
                .iter()
                .map(|t| format!("#{t}"))
                .collect::<Vec<_>>()
                .join(" "),
            app.theme.dim_style(),
        ));
    }

    if let Some(rule) = todo
        .recurring_id
        .and_then(|rid| app.recurring.iter().find(|r| r.id == rid).map(|r| r.rule))
    {
        meta_spans.push(Span::raw("    "));
        meta_spans.push(Span::styled("Repeats: ", app.theme.dim_style()));
        let rule_str = match rule {
            crate::model::RecurrenceRule::Daily => "daily",
            crate::model::RecurrenceRule::Weekdays => "weekdays",
            crate::model::RecurrenceRule::Weekly => "weekly",
        };
        meta_spans.push(Span::styled(rule_str, app.theme.text_style()));
    }

    let subtasks: Vec<&Todo> = app
        .todos
        .iter()
        .filter(|t| t.parent == Some(todo.id))
        .collect();
    if !subtasks.is_empty() {
        let done_subs = subtasks.iter().filter(|t| t.done).count();
        meta_spans.push(Span::raw("    "));
        meta_spans.push(Span::styled("Subtasks: ", app.theme.dim_style()));
        meta_spans.push(Span::styled(
            format!("{done_subs}/{} done", subtasks.len()),
            app.theme.dim_style(),
        ));
    }

    lines.push(Line::from(meta_spans));

    // Note section
    if let Some(ref note) = todo.note {
        lines.push(Line::from(vec![Span::styled(
            "Note:",
            app.theme.dim_style(),
        )]));
        for line in note.lines() {
            lines.push(Line::from(Span::styled(
                format!("  {line}"),
                app.theme.text_style(),
            )));
        }
    } else {
        lines.push(Line::from(Span::styled(
            "Note: (none · press 'n' to add)",
            app.theme.dim_style(),
        )));
    }

    frame.render_widget(
        Paragraph::new(lines).wrap(ratatui::widgets::Wrap { trim: false }),
        area,
    );
}

/// Formats dates friendly: "Today", "Yesterday", "Tomorrow", or "Mon 19 Oct"
fn format_friendly_date(date: NaiveDate, today: NaiveDate, short_date_format: &str) -> String {
    if date == today {
        "Today".to_string()
    } else if today.pred_opt() == Some(date) {
        "Yesterday".to_string()
    } else if today.succ_opt() == Some(date) {
        "Tomorrow".to_string()
    } else {
        date.format(short_date_format).to_string()
    }
}

/// Truncates string gracefully with ellipsis if it exceeds max_len
fn truncate_text(text: &str, max_len: usize, ellipsis: &str) -> String {
    if max_len == 0 {
        return String::new();
    }
    let char_count = text.chars().count();
    if char_count <= max_len {
        return text.to_string();
    }
    let el_len = ellipsis.chars().count();
    if max_len <= el_len {
        return ellipsis.chars().take(max_len).collect();
    }
    let take_count = max_len - el_len;
    let truncated: String = text.chars().take(take_count).collect();
    format!("{truncated}{ellipsis}")
}

/// Renders a quiet slim progress bar using the configured style
pub fn render_progress_bar(
    done: usize,
    total: usize,
    bar_width: usize,
    style: crate::config::ProgressStyle,
    theme: &Theme,
    symbols: &Symbols,
) -> Vec<Span<'static>> {
    if bar_width == 0 || style == crate::config::ProgressStyle::None {
        return Vec::new();
    }

    let effective_style = if symbols.is_ascii {
        crate::config::ProgressStyle::Ascii
    } else {
        style
    };

    let (filled_char, empty_char) = match effective_style {
        crate::config::ProgressStyle::Block => ("█", "░"),
        crate::config::ProgressStyle::Line => ("━", "─"),
        crate::config::ProgressStyle::Dots => ("●", "○"),
        crate::config::ProgressStyle::Ascii => ("#", "-"),
        crate::config::ProgressStyle::None => return Vec::new(),
    };

    let filled = (done * bar_width + total / 2)
        .checked_div(total)
        .unwrap_or(0)
        .min(bar_width);
    let empty = bar_width.saturating_sub(filled);

    let mut spans = Vec::new();
    if filled > 0 {
        spans.push(Span::styled(
            filled_char.repeat(filled),
            theme.accent_style(),
        ));
    }
    if empty > 0 {
        spans.push(Span::styled(empty_char.repeat(empty), theme.dim_style()));
    }

    spans
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
            app.theme.accent_style().add_modifier(Modifier::BOLD),
        ),
        Span::raw(" ".repeat((chunks[0].width as usize).saturating_sub(25))),
        Span::styled("esc / q / ? to close", app.theme.dim_style()),
    ]);
    frame.render_widget(Paragraph::new(title_line), chunks[0]);

    frame.render_widget(
        Paragraph::new(app.symbols.h_rule.repeat(chunks[1].width as usize))
            .style(app.theme.divider_style()),
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

        frame.render_widget(
            Paragraph::new(build_left_help_lines(
                &app.theme,
                &app.keymap,
                app.config.ui.show_history,
            )),
            cols[0],
        );
        frame.render_widget(
            Paragraph::new(build_right_help_lines(
                &app.theme,
                &app.keymap,
                app.config.ui.show_history,
            )),
            cols[2],
        );
    } else {
        frame.render_widget(
            Paragraph::new(build_all_help_lines(
                &app.theme,
                &app.keymap,
                app.config.ui.show_history,
            )),
            body_area,
        );
    }
}

fn help_entry(key: &str, desc: &str, pad_key: usize, theme: &Theme) -> Line<'static> {
    let key_pad = pad_key.saturating_sub(key.chars().count());
    Line::from(vec![
        Span::styled(
            format!("  {key}"),
            theme.accent_style().add_modifier(Modifier::BOLD),
        ),
        Span::raw(" ".repeat(key_pad + 2)),
        Span::styled(desc.to_string(), theme.dim_style()),
    ])
}

fn help_header(title: &str, theme: &Theme) -> Line<'static> {
    Line::from(vec![Span::styled(
        title.to_string(),
        theme
            .text_style()
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
    )])
}

fn build_left_help_lines(
    theme: &Theme,
    keymap: &crate::keymap::Keymap,
    show_history: bool,
) -> Vec<Line<'static>> {
    let mut lines = vec![help_header("Navigation", theme)];
    if show_history {
        lines.push(help_entry(
            &keymap.hint(crate::keymap::Action::SwitchView),
            "switch views",
            12,
            theme,
        ));
    }
    let nav_down = keymap.hint(crate::keymap::Action::MoveSelectionDown);
    let nav_up = keymap.hint(crate::keymap::Action::MoveSelectionUp);
    lines.push(help_entry(
        &format!("{nav_down} / {nav_up}"),
        "navigate down / up",
        12,
        theme,
    ));

    let top = keymap.hint(crate::keymap::Action::JumpTop);
    let bot = keymap.hint(crate::keymap::Action::JumpBottom);
    lines.push(help_entry(
        &format!("{top} / {bot}"),
        "jump top / bottom",
        12,
        theme,
    ));

    let pg_down = keymap.hint(crate::keymap::Action::HalfPageDown);
    let pg_up = keymap.hint(crate::keymap::Action::HalfPageUp);
    lines.push(help_entry(
        &format!("{pg_down}/{pg_up}"),
        "half page down / up",
        12,
        theme,
    ));

    lines.push(help_entry(
        &keymap.hint(crate::keymap::Action::Help),
        "toggle help",
        12,
        theme,
    ));
    lines.push(help_entry(
        &keymap.hint(crate::keymap::Action::Quit),
        "quit",
        12,
        theme,
    ));

    if show_history {
        lines.push(Line::raw(""));
        lines.push(help_header("History", theme));
        lines.push(help_entry(
            &keymap.hint(crate::keymap::Action::FocusTasks),
            "focus tasks column",
            12,
            theme,
        ));
        lines.push(help_entry(
            &keymap.hint(crate::keymap::Action::FocusDates),
            "focus dates column",
            12,
            theme,
        ));
    }

    lines.push(Line::raw(""));
    lines.push(help_header("Input", theme));
    lines.push(help_entry("enter", "save", 12, theme));
    lines.push(help_entry("esc", "cancel", 12, theme));

    lines
}

fn build_right_help_lines(
    theme: &Theme,
    keymap: &crate::keymap::Keymap,
    show_history: bool,
) -> Vec<Line<'static>> {
    let reorder_down = keymap.hint(crate::keymap::Action::ReorderDown);
    let reorder_up = keymap.hint(crate::keymap::Action::ReorderUp);

    let mut lines = vec![
        help_header("Tasks", theme),
        help_entry(
            &keymap.hint(crate::keymap::Action::Add),
            "add new task",
            12,
            theme,
        ),
        help_entry(
            &keymap.hint(crate::keymap::Action::AddSubtask),
            "add subtask",
            12,
            theme,
        ),
        help_entry(
            &keymap.hint(crate::keymap::Action::Edit),
            "edit selected task",
            12,
            theme,
        ),
        help_entry(
            &keymap.hint(crate::keymap::Action::Toggle),
            "toggle complete",
            12,
            theme,
        ),
        help_entry(
            &keymap.hint(crate::keymap::Action::CyclePriority),
            "cycle priority",
            12,
            theme,
        ),
        help_entry(
            &keymap.hint(crate::keymap::Action::CycleSort),
            "cycle sort mode",
            12,
            theme,
        ),
        help_entry(
            &keymap.hint(crate::keymap::Action::Reschedule),
            "reschedule task",
            12,
            theme,
        ),
        help_entry(
            &keymap.hint(crate::keymap::Action::Filter),
            "filter tasks",
            12,
            theme,
        ),
        help_entry(
            &keymap.hint(crate::keymap::Action::ToggleDetail),
            "toggle detail pane",
            12,
            theme,
        ),
        help_entry(
            &keymap.hint(crate::keymap::Action::EditNote),
            "edit task note",
            12,
            theme,
        ),
        help_entry(
            &keymap.hint(crate::keymap::Action::ToggleFocus),
            "toggle focus task",
            12,
            theme,
        ),
        help_entry(
            &keymap.hint(crate::keymap::Action::Recurrence),
            "set recurrence",
            12,
            theme,
        ),
        help_entry(
            &keymap.hint(crate::keymap::Action::ToggleMark),
            "mark task",
            12,
            theme,
        ),
        help_entry(
            &keymap.hint(crate::keymap::Action::ToggleMarkAll),
            "mark all visible",
            12,
            theme,
        ),
        help_entry(
            &format!("{reorder_down} / {reorder_up}"),
            "move task down / up",
            12,
            theme,
        ),
    ];
    if show_history {
        lines.push(help_entry(
            &keymap.hint(crate::keymap::Action::MoveToToday),
            "move task to Today",
            12,
            theme,
        ));
    }
    lines.extend(vec![
        help_entry(
            &keymap.hint(crate::keymap::Action::Delete),
            "delete task",
            12,
            theme,
        ),
        help_entry(
            &keymap.hint(crate::keymap::Action::ClearCompleted),
            "clear completed tasks",
            12,
            theme,
        ),
        help_entry(
            &keymap.hint(crate::keymap::Action::CarryOver),
            "carry over unfinished",
            12,
            theme,
        ),
        help_entry(
            &keymap.hint(crate::keymap::Action::Undo),
            "undo last action",
            12,
            theme,
        ),
        help_entry(
            &keymap.hint(crate::keymap::Action::Redo),
            "redo undone action",
            12,
            theme,
        ),
    ]);
    lines
}

fn build_all_help_lines(
    theme: &Theme,
    keymap: &crate::keymap::Keymap,
    show_history: bool,
) -> Vec<Line<'static>> {
    let mut lines = build_left_help_lines(theme, keymap, show_history);
    lines.push(Line::raw(""));
    lines.extend(build_right_help_lines(theme, keymap, show_history));
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
                ..Default::default()
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
                ..Default::default()
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
                ..Default::default()
            },
            Todo {
                id: 2,
                text: "Task 2".to_string(),
                created_date: today.pred_opt().unwrap(),
                done: true,
                done_at: None,
                ..Default::default()
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

    #[test]
    fn test_render_mono_no_fg_colors_other_than_reset() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let today = NaiveDate::from_ymd_opt(2026, 10, 27).unwrap();
        let mut app = App::new_for_test_with_date(
            vec![
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
                    done: true,
                    done_at: None,
                    ..Default::default()
                },
            ],
            today,
        );
        app.theme = Theme::preset_mono();

        terminal.draw(|f| render(&mut app, f)).unwrap();
        for cell in terminal.backend().buffer().content() {
            assert_eq!(
                cell.fg,
                ratatui::style::Color::Reset,
                "Expected cell with symbol '{}' to have fg Reset in mono, but got {:?}",
                cell.symbol(),
                cell.fg
            );
        }

        // Test with help overlay
        app.show_help = true;
        terminal.draw(|f| render(&mut app, f)).unwrap();
        for cell in terminal.backend().buffer().content() {
            assert_eq!(
                cell.fg,
                ratatui::style::Color::Reset,
                "Expected cell with symbol '{}' to have fg Reset in mono (help open), but got {:?}",
                cell.symbol(),
                cell.fg
            );
        }
    }

    #[test]
    fn test_symbols_ascii_mode_is_pure_ascii() {
        let sym = Symbols::ascii();
        assert!(sym.is_ascii);
        let strings = [
            sym.check_done,
            sym.check_todo,
            sym.check_overdue,
            sym.marker,
            sym.prompt_default,
            sym.prompt_edit,
            sym.prompt_reschedule,
            sym.prompt_filter,
            sym.note,
            sym.v_divider,
            sym.h_rule,
            sym.bullet,
            sym.dot,
            sym.ellipsis,
            sym.multiply,
            sym.progress_filled,
            sym.progress_empty,
        ];
        for s in strings {
            assert!(s.is_ascii(), "Symbol '{s}' is not pure ASCII");
        }
    }

    #[test]
    fn test_render_ascii_mode_all_cells_ascii() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 27).unwrap();
        let past = NaiveDate::from_ymd_opt(2026, 10, 25).unwrap();

        let todos = vec![
            Todo {
                id: 1,
                text: "Task 1 today unfinished".to_string(),
                created_date: today,
                done: false,
                done_at: None,
                ..Default::default()
            },
            Todo {
                id: 2,
                text: "Task 2 today done".to_string(),
                created_date: today,
                done: true,
                done_at: None,
                ..Default::default()
            },
            Todo {
                id: 3,
                text: "Task 3 past overdue".to_string(),
                created_date: past,
                done: false,
                done_at: None,
                ..Default::default()
            },
        ];

        let mut app = App::new_for_test_with_date(todos, today);
        app.symbols = Symbols::ascii();

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let assert_all_ascii = |term: &Terminal<TestBackend>, view_name: &str| {
            for cell in term.backend().buffer().content() {
                assert!(
                    cell.symbol().is_ascii(),
                    "In view {view_name}, cell symbol '{}' contains non-ASCII char",
                    cell.symbol()
                );
            }
        };

        // 1. Today view
        terminal.draw(|f| render(&mut app, f)).unwrap();
        assert_all_ascii(&terminal, "Today");

        // 2. History view
        app.current_view = View::History;
        terminal.draw(|f| render(&mut app, f)).unwrap();
        assert_all_ascii(&terminal, "History");

        // 3. Input mode adding
        app.input_mode = InputMode::Adding;
        app.input_buffer = "New task text".to_string();
        terminal.draw(|f| render(&mut app, f)).unwrap();
        assert_all_ascii(&terminal, "Input Adding");

        // 4. Input mode editing
        app.input_mode = InputMode::Editing(1);
        terminal.draw(|f| render(&mut app, f)).unwrap();
        assert_all_ascii(&terminal, "Input Editing");

        // 5. Help overlay
        app.input_mode = InputMode::Normal;
        app.show_help = true;
        terminal.draw(|f| render(&mut app, f)).unwrap();
        assert_all_ascii(&terminal, "Help Overlay");

        // 6. Too-small screen
        app.show_help = false;
        let small_backend = TestBackend::new(30, 6);
        let mut small_terminal = Terminal::new(small_backend).unwrap();
        small_terminal.draw(|f| render(&mut app, f)).unwrap();
        assert_all_ascii(&small_terminal, "Too Small Screen");
    }

    #[test]
    fn test_progress_bar_styles_widths_and_edge_cases() {
        let theme = Theme::default();
        let unicode_sym = Symbols::unicode();
        let ascii_sym = Symbols::ascii();

        // 1. Widths 4, 10, 40 with Block style
        for width in [4, 10, 40] {
            // 0 of 0 tasks (safe integer math, no divide by zero)
            let spans = render_progress_bar(
                0,
                0,
                width,
                crate::config::ProgressStyle::Block,
                &theme,
                &unicode_sym,
            );
            let text: String = spans.iter().map(|s| s.content.as_ref()).collect();
            assert_eq!(text.chars().count(), width);
            assert_eq!(text, "░".repeat(width));

            // All done (5 of 5)
            let spans_done = render_progress_bar(
                5,
                5,
                width,
                crate::config::ProgressStyle::Block,
                &theme,
                &unicode_sym,
            );
            let text_done: String = spans_done.iter().map(|s| s.content.as_ref()).collect();
            assert_eq!(text_done.chars().count(), width);
            assert_eq!(text_done, "█".repeat(width));
        }

        // 2. Each style: Line, Dots, Ascii, None
        let width = 10;
        // Line
        let line_spans = render_progress_bar(
            5,
            10,
            width,
            crate::config::ProgressStyle::Line,
            &theme,
            &unicode_sym,
        );
        let line_text: String = line_spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(line_text, "━━━━━─────");

        // Dots
        let dots_spans = render_progress_bar(
            5,
            10,
            width,
            crate::config::ProgressStyle::Dots,
            &theme,
            &unicode_sym,
        );
        let dots_text: String = dots_spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(dots_text, "●●●●●○○○○○");

        // Ascii
        let ascii_spans = render_progress_bar(
            5,
            10,
            width,
            crate::config::ProgressStyle::Ascii,
            &theme,
            &unicode_sym,
        );
        let ascii_text: String = ascii_spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(ascii_text, "#####-----");

        // None
        let none_spans = render_progress_bar(
            5,
            10,
            width,
            crate::config::ProgressStyle::None,
            &theme,
            &unicode_sym,
        );
        assert!(none_spans.is_empty());

        // 3. ASCII symbols mode forces Ascii progress bar style even if config is Block
        let forced_ascii = render_progress_bar(
            5,
            10,
            width,
            crate::config::ProgressStyle::Block,
            &theme,
            &ascii_sym,
        );
        let forced_text: String = forced_ascii.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(forced_text, "#####-----");
    }

    #[test]
    fn test_render_show_history_false_hides_history_tab_and_hints() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 27).unwrap();
        let past = NaiveDate::from_ymd_opt(2026, 10, 25).unwrap();

        let todos = vec![
            Todo {
                id: 1,
                text: "Task 1 today".to_string(),
                created_date: today,
                done: false,
                done_at: None,
                ..Default::default()
            },
            Todo {
                id: 2,
                text: "Task 2 overdue in past".to_string(),
                created_date: past,
                done: false,
                done_at: None,
                ..Default::default()
            },
        ];

        let mut app = App::new_for_test_with_date(todos, today);
        app.config.ui.show_history = false;

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        // 1. In normal view: History tab must NOT be rendered, and overdue marker must NOT be rendered
        terminal.draw(|f| render(&mut app, f)).unwrap();
        let content: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();

        assert!(content.contains("Slate"));
        assert!(content.contains("Today"));
        assert!(
            !content.contains("History"),
            "Expected 'History' tab not to appear in header when show_history=false, but found it"
        );
        assert!(
            !content.contains("overdue"),
            "Expected overdue count not to appear in header when show_history=false"
        );
        assert!(
            !content.contains("tab  history"),
            "Expected 'tab history' hint not to appear in footer"
        );

        // 2. In help overlay: History section, move-to-today, and Tab switch views must NOT be rendered
        app.show_help = true;
        terminal.draw(|f| render(&mut app, f)).unwrap();
        let help_content: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();

        assert!(help_content.contains("Help"));
        assert!(help_content.contains("Navigation"));
        assert!(help_content.contains("Tasks"));
        assert!(
            !help_content.contains("History"),
            "Expected 'History' section not to appear in help overlay when show_history=false"
        );
        assert!(
            !help_content.contains("switch views"),
            "Expected 'switch views' not to appear in help overlay when show_history=false"
        );
        assert!(
            !help_content.contains("move task to Today"),
            "Expected 'move task to Today' not to appear in help overlay when show_history=false"
        );
    }

    #[test]
    fn test_render_remapped_keys_appear_in_footer_and_help() {
        let mut config = crate::config::Config::default();
        config.keys.overrides.insert(
            "add".to_string(),
            toml::Value::Array(vec![toml::Value::String("n".to_string())]),
        );
        config.keys.overrides.insert(
            "toggle".to_string(),
            toml::Value::Array(vec![toml::Value::String("x".to_string())]),
        );

        let (keymap, _) = crate::keymap::Keymap::from_config(&config);
        let mut app = App::new_for_test(Vec::new());
        app.keymap = keymap;

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        // 1. Footer should show remapped keys "n add" and "x done"
        terminal.draw(|f| render(&mut app, f)).unwrap();
        let footer_content: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();

        assert!(
            footer_content.contains("n add"),
            "Expected footer to show 'n add', got: {footer_content}"
        );
        assert!(
            footer_content.contains("x done"),
            "Expected footer to show 'x done', got: {footer_content}"
        );

        // 2. Help overlay should show remapped keys for add and toggle
        app.show_help = true;
        terminal.draw(|f| render(&mut app, f)).unwrap();
        let help_content: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();

        assert!(
            help_content.contains("n") && help_content.contains("add new task"),
            "Expected help overlay to contain remapped 'n' for add new task"
        );
        assert!(
            help_content.contains("x") && help_content.contains("toggle complete"),
            "Expected help overlay to contain remapped 'x' for toggle complete"
        );
    }

    #[test]
    fn test_render_small_terminal_all_options_on_no_panic() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 27).unwrap();
        let past = NaiveDate::from_ymd_opt(2026, 10, 25).unwrap();

        let todos = vec![
            Todo {
                id: 1,
                text: "Task 1 today".to_string(),
                created_date: today,
                done: false,
                done_at: None,
                ..Default::default()
            },
            Todo {
                id: 2,
                text: "Task 2 overdue in past".to_string(),
                created_date: past,
                done: false,
                done_at: None,
                ..Default::default()
            },
        ];

        for (w, h) in [(0, 0), (1, 1), (5, 2), (10, 4), (20, 5), (39, 7), (40, 8)] {
            let backend = TestBackend::new(w, h);
            let mut terminal = Terminal::new(backend).unwrap();

            let mut app = App::new_for_test_with_date(todos.clone(), today);
            app.config.ui.mouse = true;
            app.config.ui.show_history = true;
            app.config.progress.show_count = true;
            app.config.progress.width = 40;
            app.config.progress.style = crate::config::ProgressStyle::Block;

            // Render normal view
            terminal.draw(|f| render(&mut app, f)).unwrap();

            // Render with help open
            app.show_help = true;
            terminal.draw(|f| render(&mut app, f)).unwrap();
            app.show_help = false;

            // Render with adding prompt
            app.input_mode = crate::model::InputMode::Adding;
            terminal.draw(|f| render(&mut app, f)).unwrap();
            app.input_mode = crate::model::InputMode::Normal;

            // Render in history view
            app.current_view = crate::model::View::History;
            terminal.draw(|f| render(&mut app, f)).unwrap();
        }
    }

    #[test]
    fn test_render_priorities_and_header_sort() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let today = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        let mut app = App::new_for_test_with_date(
            vec![
                Todo {
                    id: 1,
                    text: "Important task".to_string(),
                    created_date: today,
                    priority: 1,
                    ..Default::default()
                },
                Todo {
                    id: 2,
                    text: "Urgent task".to_string(),
                    created_date: today,
                    priority: 2,
                    ..Default::default()
                },
            ],
            today,
        );

        app.sort_mode = crate::model::SortMode::Priority;
        terminal.draw(|f| render(&mut app, f)).unwrap();
        let content: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();

        assert!(content.contains("sort: priority"));
        assert!(content.contains("! Important task"));
        assert!(content.contains("!! Urgent task"));
    }

    #[test]
    fn test_render_tags_and_filter_header() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        let make_app = || {
            App::new_for_test_with_date(
                vec![Todo {
                    id: 1,
                    text: "Project work".to_string(),
                    created_date: today,
                    tags: vec!["work".to_string(), "urgent".to_string()],
                    ..Default::default()
                }],
                today,
            )
        };

        // 1. Normal width (80 cols) with show_tags=true: tags render
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = make_app();
        app.terminal_width = 80;
        app.today_filter = crate::parser::parse_filter("#work");
        terminal.draw(|f| render(&mut app, f)).unwrap();
        let content: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();

        assert!(content.contains("filter: #work"));
        assert!(content.contains("#work #urgent"));

        // 2. Narrow width (55 cols < 60): tags are hidden
        let backend_narrow = TestBackend::new(55, 24);
        let mut terminal_narrow = Terminal::new(backend_narrow).unwrap();
        let mut app_narrow = make_app();
        app_narrow.terminal_width = 55;
        terminal_narrow
            .draw(|f| render(&mut app_narrow, f))
            .unwrap();
        let content_narrow: String = terminal_narrow
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();

        assert!(!content_narrow.contains("#work #urgent"));
        assert!(content_narrow.contains("Project work"));

        // 3. Normal width with show_tags=false: tags are hidden
        let backend_notags = TestBackend::new(80, 24);
        let mut terminal_notags = Terminal::new(backend_notags).unwrap();
        let mut app_notags = make_app();
        app_notags.terminal_width = 80;
        app_notags.config.tasks.show_tags = false;
        terminal_notags
            .draw(|f| render(&mut app_notags, f))
            .unwrap();
        let content_notags: String = terminal_notags
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();

        assert!(!content_notags.contains("#work #urgent"));
        assert!(content_notags.contains("Project work"));
    }

    #[test]
    fn test_render_note_glyph_and_detail_pane() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        let mut app = App::new_for_test_with_date(
            vec![Todo {
                id: 1,
                text: "Refactor storage layer".to_string(),
                created_date: today,
                note: Some("Make sure backups prune correctly".to_string()),
                ..Default::default()
            }],
            today,
        );

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        // 1. Task row displays note glyph ≡
        terminal.draw(|f| render(&mut app, f)).unwrap();
        let content: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();

        assert!(content.contains("≡"));

        // 2. Open detail pane
        app.show_detail_pane = true;
        terminal.draw(|f| render(&mut app, f)).unwrap();
        let content_detail: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();

        assert!(content_detail.contains("Task: Refactor storage layer"));
        assert!(content_detail.contains("Note:"));
        assert!(content_detail.contains("Make sure backups prune correctly"));

        // 3. Auto-hide when terminal height is below 20
        let backend_small = TestBackend::new(80, 18);
        let mut terminal_small = Terminal::new(backend_small).unwrap();
        app.terminal_height = 18;
        terminal_small.draw(|f| render(&mut app, f)).unwrap();
        let content_small: String = terminal_small
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();

        assert!(!content_small.contains("Make sure backups prune correctly"));
    }

    #[test]
    fn test_render_subtask_connector_and_progress() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let today = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        let mut app = App::new_for_test_with_date(
            vec![
                Todo {
                    id: 1,
                    text: "Parent task".to_string(),
                    created_date: today,
                    done: false,
                    ..Default::default()
                },
                Todo {
                    id: 2,
                    text: "Subtask 1".to_string(),
                    created_date: today,
                    done: true,
                    parent: Some(1),
                    ..Default::default()
                },
                Todo {
                    id: 3,
                    text: "Subtask 2".to_string(),
                    created_date: today,
                    done: false,
                    parent: Some(1),
                    ..Default::default()
                },
            ],
            today,
        );

        // Render Unicode
        terminal.draw(|f| render(&mut app, f)).unwrap();
        let content_unicode: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();

        // Header shows top-level tasks only: "0 of 1 done"
        assert!(content_unicode.contains("0 of 1 done"));
        // Parent row shows progress "1/2"
        assert!(content_unicode.contains("1/2"));
        // Subtask connector "└"
        assert!(content_unicode.contains('└'));

        // Switch to ASCII mode
        let mut ascii_config = crate::config::Config::default();
        ascii_config.ui.symbols = crate::config::SymbolsMode::Ascii;
        app.symbols = Symbols::ascii();
        terminal.draw(|f| render(&mut app, f)).unwrap();
        let content_ascii: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();

        assert!(content_ascii.contains("0 of 1 done"));
        assert!(content_ascii.contains("1/2"));
        assert!(content_ascii.contains('`'));
    }

    #[test]
    fn test_render_focus_task_header_and_row_marker() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let today = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        let tomorrow = NaiveDate::from_ymd_opt(2026, 10, 31).unwrap();
        let mut app = App::new_for_test_with_date(
            vec![
                Todo {
                    id: 1,
                    text: "Today task".to_string(),
                    created_date: today,
                    done: false,
                    ..Default::default()
                },
                Todo {
                    id: 2,
                    text: "Tomorrow focus task".to_string(),
                    created_date: tomorrow,
                    done: false,
                    ..Default::default()
                },
            ],
            today,
        );

        // 1. Focus a future task -> Today view shows it above the list
        app.focus = Some(2);
        terminal.draw(|f| render(&mut app, f)).unwrap();
        let content: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();

        assert!(content.contains("▸ Focus: Tomorrow focus task"));
        // Today task (id 1) is not pinned -> row doesn't have focus marker
        // (content only has '▸' in the Focus header line)
        assert_eq!(content.matches('▸').count(), 1);

        // 2. Focus Today task -> row shows marker as well
        app.focus = Some(1);
        terminal.draw(|f| render(&mut app, f)).unwrap();
        let content_today_focus: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();

        assert!(content_today_focus.contains("▸ Focus: Today task"));
        // Both header line and row have '▸'
        assert_eq!(content_today_focus.matches('▸').count(), 2);

        // 3. ASCII mode
        app.symbols = Symbols::ascii();
        terminal.draw(|f| render(&mut app, f)).unwrap();
        let content_ascii: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();

        assert!(content_ascii.contains("> Focus: Today task"));
    }

    #[test]
    fn test_render_recurring_glyph_and_delete_prompt() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let today = NaiveDate::from_ymd_opt(2026, 10, 28).unwrap();
        let mut app = App::new_for_test_with_date(Vec::new(), today);

        app.todos.push(Todo {
            id: 1,
            text: "Daily standup".to_string(),
            created_date: today,
            done: false,
            recurring_id: Some(100),
            ..Default::default()
        });
        app.recurring.push(crate::model::Recurring {
            id: 100,
            text: "Daily standup".to_string(),
            tags: vec![],
            priority: 0,
            rule: crate::model::RecurrenceRule::Daily,
            anchor: today,
            last_materialized: Some(today),
        });

        // 1. Unicode mode renders '↻'
        terminal.draw(|f| render(&mut app, f)).unwrap();
        let content: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(content.contains('↻'));

        // 2. ASCII mode renders '~'
        app.symbols = Symbols::ascii();
        terminal.draw(|f| render(&mut app, f)).unwrap();
        let content_ascii: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(content_ascii.contains('~'));

        // 3. ConfirmDeleteRecurring prompt
        app.input_mode = InputMode::ConfirmDeleteRecurring(1);
        terminal.draw(|f| render(&mut app, f)).unwrap();
        let content_prompt: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(content_prompt.contains("Delete (t)his instance or (a)ll (template + future)?"));
        assert!(content_prompt.contains("esc cancel"));
    }

    #[test]
    fn test_render_marked_rows_marker_unicode_and_ascii() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        let today = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        let mut app = App::new_for_test_with_date(
            vec![
                Todo {
                    id: 1,
                    text: "Marked task".to_string(),
                    created_date: today,
                    done: false,
                    ..Default::default()
                },
                Todo {
                    id: 2,
                    text: "Unmarked task".to_string(),
                    created_date: today,
                    done: false,
                    ..Default::default()
                },
            ],
            today,
        );

        app.marks.insert(1);

        // 1. Unicode mode renders '◆' for marked task and '›' for unmarked selected task
        app.today_selected_index = 1;
        app.today_selected_id = Some(2);
        terminal.draw(|f| render(&mut app, f)).unwrap();
        let content: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();

        assert!(content.contains('◆'));
        assert!(content.contains('›'));

        // 2. ASCII mode renders '*' for marked task and '>' for unmarked selected task
        app.symbols = Symbols::ascii();
        terminal.draw(|f| render(&mut app, f)).unwrap();
        let content_ascii: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();

        assert!(content_ascii.contains('*'));
        assert!(content_ascii.contains('>'));
    }

    #[test]
    fn test_render_search_screen_empty_results_and_narrow() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        let mut app = App::new_for_test_with_date(
            vec![
                Todo {
                    id: 1,
                    text: "Important report".to_string(),
                    created_date: today,
                    done: false,
                    tags: vec!["work".to_string()],
                    ..Default::default()
                },
                Todo {
                    id: 2,
                    text: "Groceries".to_string(),
                    created_date: today,
                    done: true,
                    ..Default::default()
                },
            ],
            today,
        );

        // 1. Search screen with empty query shows hint
        app.start_search();
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();
        let content: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();

        assert!(content.contains("type to search"));
        assert!(content.contains("2 tasks across 1 days"));

        // 2. Search screen with match shows highlighted results
        for c in "report".chars() {
            app.input_buffer.push(c);
            app.cursor_position += 1;
        }
        app.update_search();
        terminal.draw(|f| render(&mut app, f)).unwrap();
        let content_res: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();

        assert!(content_res.contains("Important report"));
        assert!(content_res.contains("#work"));

        // 3. Render at narrow width (40x8) and ASCII mode without panic
        app.symbols = Symbols::ascii();
        let backend_narrow = TestBackend::new(40, 8);
        let mut terminal_narrow = Terminal::new(backend_narrow).unwrap();
        terminal_narrow.draw(|f| render(&mut app, f)).unwrap();
    }

    #[test]
    fn test_render_summary_week() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 28).unwrap(); // Wed
        let yesterday = NaiveDate::from_ymd_opt(2026, 10, 27).unwrap();
        let mut app = App::new_for_test_with_date(
            vec![
                Todo {
                    id: 1,
                    text: "Yesterday task 1".to_string(),
                    created_date: yesterday,
                    done: true,
                    ..Default::default()
                },
                Todo {
                    id: 2,
                    text: "Yesterday task 2".to_string(),
                    created_date: yesterday,
                    done: false,
                    ..Default::default()
                },
                Todo {
                    id: 3,
                    text: "Today task".to_string(),
                    created_date: today,
                    done: true,
                    ..Default::default()
                },
            ],
            today,
        );
        app.current_view = View::Summary;
        app.summary_date = today;

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| render(&mut app, f)).unwrap();

        let content: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();

        assert!(content.contains("Week of"));
        assert!(content.contains("done"));
        assert!(content.contains("Summary"));

        // Hit areas should include SummaryDay targets
        let has_day_hits = app
            .hit_areas
            .iter()
            .any(|h| matches!(h.target, HitTarget::SummaryDay { .. }));
        assert!(has_day_hits);

        // Render in ASCII mode and Mono mode
        app.symbols = Symbols::ascii();
        app.theme = Theme::preset_mono();
        terminal.draw(|f| render(&mut app, f)).unwrap();

        // Narrow size (40x8) and 0x0
        let narrow = TestBackend::new(40, 8);
        let mut t_narrow = Terminal::new(narrow).unwrap();
        t_narrow.draw(|f| render(&mut app, f)).unwrap();

        let zero = TestBackend::new(0, 0);
        let mut t_zero = Terminal::new(zero).unwrap();
        t_zero.draw(|f| render(&mut app, f)).unwrap();
    }
}
