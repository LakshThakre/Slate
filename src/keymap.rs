use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::collections::HashMap;

use crate::config::{Config, KeyMode};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    Quit,
    Add,
    Edit,
    Toggle,
    Delete,
    ClearCompleted,
    Undo,
    Redo,
    CarryOver,
    MoveToToday,
    MoveSelectionUp,
    MoveSelectionDown,
    ReorderUp,
    ReorderDown,
    JumpTop,
    JumpBottom,
    HalfPageUp,
    HalfPageDown,
    Help,
    SwitchView,
    FocusTasks,
    FocusDates,
    Reschedule,
    CyclePriority,
    CycleSort,
    Filter,
    ToggleDetail,
    EditNote,
    AddSubtask,
    ToggleFocus,
    Recurrence,
    ToggleMark,
    ToggleMarkAll,
    Search,
    JumpToDate,
    ToggleSummaryRange,
    PrevPeriod,
    NextPeriod,
}

impl Action {
    pub const ALL: [Action; 38] = [
        Action::Quit,
        Action::Add,
        Action::AddSubtask,
        Action::Edit,
        Action::Toggle,
        Action::Delete,
        Action::ClearCompleted,
        Action::Undo,
        Action::Redo,
        Action::CarryOver,
        Action::MoveToToday,
        Action::MoveSelectionUp,
        Action::MoveSelectionDown,
        Action::ReorderUp,
        Action::ReorderDown,
        Action::JumpTop,
        Action::JumpBottom,
        Action::HalfPageUp,
        Action::HalfPageDown,
        Action::Help,
        Action::SwitchView,
        Action::FocusTasks,
        Action::FocusDates,
        Action::Reschedule,
        Action::CyclePriority,
        Action::CycleSort,
        Action::Filter,
        Action::ToggleDetail,
        Action::EditNote,
        Action::ToggleFocus,
        Action::Recurrence,
        Action::ToggleMark,
        Action::ToggleMarkAll,
        Action::Search,
        Action::JumpToDate,
        Action::ToggleSummaryRange,
        Action::PrevPeriod,
        Action::NextPeriod,
    ];

    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "quit" => Some(Action::Quit),
            "add" => Some(Action::Add),
            "add_subtask" | "subtask" => Some(Action::AddSubtask),
            "edit" => Some(Action::Edit),
            "toggle" => Some(Action::Toggle),
            "delete" => Some(Action::Delete),
            "clear_completed" => Some(Action::ClearCompleted),
            "undo" => Some(Action::Undo),
            "redo" => Some(Action::Redo),
            "carry_over" => Some(Action::CarryOver),
            "move_to_today" => Some(Action::MoveToToday),
            "move_selection_up" => Some(Action::MoveSelectionUp),
            "move_selection_down" => Some(Action::MoveSelectionDown),
            "reorder_up" => Some(Action::ReorderUp),
            "reorder_down" => Some(Action::ReorderDown),
            "jump_top" => Some(Action::JumpTop),
            "jump_bottom" => Some(Action::JumpBottom),
            "half_page_up" => Some(Action::HalfPageUp),
            "half_page_down" => Some(Action::HalfPageDown),
            "help" => Some(Action::Help),
            "switch_view" => Some(Action::SwitchView),
            "focus_tasks" => Some(Action::FocusTasks),
            "focus_dates" => Some(Action::FocusDates),
            "reschedule" => Some(Action::Reschedule),
            "cycle_priority" | "priority" => Some(Action::CyclePriority),
            "cycle_sort" | "sort" => Some(Action::CycleSort),
            "filter" => Some(Action::Filter),
            "toggle_detail" | "detail" => Some(Action::ToggleDetail),
            "edit_note" | "note" => Some(Action::EditNote),
            "toggle_focus" | "focus" | "pin" => Some(Action::ToggleFocus),
            "recurrence" | "recur" => Some(Action::Recurrence),
            "toggle_mark" | "mark" => Some(Action::ToggleMark),
            "toggle_mark_all" | "mark_all" => Some(Action::ToggleMarkAll),
            "search" => Some(Action::Search),
            "jump_to_date" | "jump" => Some(Action::JumpToDate),
            "toggle_summary_range" | "summary_range" => Some(Action::ToggleSummaryRange),
            "prev_period" => Some(Action::PrevPeriod),
            "next_period" => Some(Action::NextPeriod),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Action::Quit => "quit",
            Action::Add => "add",
            Action::Edit => "edit",
            Action::Toggle => "toggle",
            Action::Delete => "delete",
            Action::ClearCompleted => "clear_completed",
            Action::Undo => "undo",
            Action::Redo => "redo",
            Action::CarryOver => "carry_over",
            Action::MoveToToday => "move_to_today",
            Action::MoveSelectionUp => "move_selection_up",
            Action::MoveSelectionDown => "move_selection_down",
            Action::ReorderUp => "reorder_up",
            Action::ReorderDown => "reorder_down",
            Action::JumpTop => "jump_top",
            Action::JumpBottom => "jump_bottom",
            Action::HalfPageUp => "half_page_up",
            Action::HalfPageDown => "half_page_down",
            Action::Help => "help",
            Action::SwitchView => "switch_view",
            Action::FocusTasks => "focus_tasks",
            Action::FocusDates => "focus_dates",
            Action::Reschedule => "reschedule",
            Action::CyclePriority => "cycle_priority",
            Action::CycleSort => "cycle_sort",
            Action::Filter => "filter",
            Action::ToggleDetail => "toggle_detail",
            Action::EditNote => "edit_note",
            Action::AddSubtask => "add_subtask",
            Action::ToggleFocus => "toggle_focus",
            Action::Recurrence => "recurrence",
            Action::ToggleMark => "toggle_mark",
            Action::ToggleMarkAll => "toggle_mark_all",
            Action::Search => "search",
            Action::JumpToDate => "jump_to_date",
            Action::ToggleSummaryRange => "toggle_summary_range",
            Action::PrevPeriod => "prev_period",
            Action::NextPeriod => "next_period",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyChord {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl KeyChord {
    pub fn parse(s: &str) -> Result<Self, String> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err("empty key string".to_string());
        }

        let parts: Vec<&str> = trimmed.split('+').collect();
        let mut modifiers = KeyModifiers::NONE;

        if parts.len() == 1 {
            return parse_base_key(parts[0], modifiers);
        }

        for (i, &part) in parts.iter().enumerate() {
            if i == parts.len() - 1 {
                return parse_base_key(part, modifiers);
            }
            match part.to_ascii_lowercase().as_str() {
                "ctrl" | "control" => modifiers.insert(KeyModifiers::CONTROL),
                "alt" => modifiers.insert(KeyModifiers::ALT),
                "shift" => modifiers.insert(KeyModifiers::SHIFT),
                other => return Err(format!("unknown modifier '{other}' in '{trimmed}'")),
            }
        }

        Err(format!("invalid key string '{trimmed}'"))
    }

    pub fn matches(&self, event: &KeyEvent) -> bool {
        let event_ctrl = event.modifiers.contains(KeyModifiers::CONTROL);
        let pat_ctrl = self.modifiers.contains(KeyModifiers::CONTROL);
        if event_ctrl != pat_ctrl {
            return false;
        }

        let event_alt = event.modifiers.contains(KeyModifiers::ALT);
        let pat_alt = self.modifiers.contains(KeyModifiers::ALT);
        if event_alt != pat_alt {
            return false;
        }

        let event_shift = event.modifiers.contains(KeyModifiers::SHIFT);
        let pat_shift = self.modifiers.contains(KeyModifiers::SHIFT);

        match (self.code, event.code) {
            (KeyCode::Char(pc), KeyCode::Char(ec)) => {
                if pc.is_ascii_alphabetic() {
                    if !pc.eq_ignore_ascii_case(&ec) {
                        return false;
                    }
                    if pat_shift {
                        event_shift || ec.is_ascii_uppercase()
                    } else {
                        !event_shift && ec.is_ascii_lowercase()
                    }
                } else {
                    if pc != ec {
                        return false;
                    }
                    if pat_shift {
                        event_shift
                    } else {
                        true
                    }
                }
            }
            (KeyCode::Tab, KeyCode::BackTab) | (KeyCode::BackTab, KeyCode::Tab) => true,
            (KeyCode::BackTab, KeyCode::BackTab) => true,
            (KeyCode::Tab, KeyCode::Tab) => {
                if pat_shift {
                    event_shift
                } else {
                    !event_shift
                }
            }
            (c1, c2) => c1 == c2 && pat_shift == event_shift,
        }
    }

    pub fn format_hint(&self) -> String {
        let mut prefix = String::new();
        if self.modifiers.contains(KeyModifiers::CONTROL) {
            prefix.push_str("ctrl+");
        }
        if self.modifiers.contains(KeyModifiers::ALT) {
            prefix.push_str("alt+");
        }

        let base_shift = self.modifiers.contains(KeyModifiers::SHIFT);

        let base = match self.code {
            KeyCode::Char(' ') => "space".to_string(),
            KeyCode::Char(c) => {
                if base_shift && c.is_ascii_alphabetic() {
                    return format!("{prefix}{}", c.to_ascii_uppercase());
                } else {
                    c.to_string()
                }
            }
            KeyCode::Enter => "enter".to_string(),
            KeyCode::Esc => "esc".to_string(),
            KeyCode::Tab => "tab".to_string(),
            KeyCode::BackTab => "backtab".to_string(),
            KeyCode::Up => "up".to_string(),
            KeyCode::Down => "down".to_string(),
            KeyCode::Left => "left".to_string(),
            KeyCode::Right => "right".to_string(),
            KeyCode::Home => "home".to_string(),
            KeyCode::End => "end".to_string(),
            KeyCode::PageUp => "pageup".to_string(),
            KeyCode::PageDown => "pagedown".to_string(),
            KeyCode::Delete => "delete".to_string(),
            KeyCode::Backspace => "backspace".to_string(),
            _ => "?".to_string(),
        };

        if base_shift && !self.code_is_upper_char() {
            format!("{prefix}shift+{base}")
        } else {
            format!("{prefix}{base}")
        }
    }

    fn code_is_upper_char(&self) -> bool {
        match self.code {
            KeyCode::Char(c) => c.is_ascii_alphabetic(),
            _ => false,
        }
    }
}

fn parse_base_key(s: &str, mut modifiers: KeyModifiers) -> Result<KeyChord, String> {
    match s.to_ascii_lowercase().as_str() {
        "enter" | "return" => Ok(KeyChord {
            code: KeyCode::Enter,
            modifiers,
        }),
        "esc" | "escape" => Ok(KeyChord {
            code: KeyCode::Esc,
            modifiers,
        }),
        "space" => Ok(KeyChord {
            code: KeyCode::Char(' '),
            modifiers,
        }),
        "tab" => Ok(KeyChord {
            code: KeyCode::Tab,
            modifiers,
        }),
        "backtab" => Ok(KeyChord {
            code: KeyCode::BackTab,
            modifiers: modifiers | KeyModifiers::SHIFT,
        }),
        "up" => Ok(KeyChord {
            code: KeyCode::Up,
            modifiers,
        }),
        "down" => Ok(KeyChord {
            code: KeyCode::Down,
            modifiers,
        }),
        "left" => Ok(KeyChord {
            code: KeyCode::Left,
            modifiers,
        }),
        "right" => Ok(KeyChord {
            code: KeyCode::Right,
            modifiers,
        }),
        "home" => Ok(KeyChord {
            code: KeyCode::Home,
            modifiers,
        }),
        "end" => Ok(KeyChord {
            code: KeyCode::End,
            modifiers,
        }),
        "pageup" | "page_up" | "pgup" => Ok(KeyChord {
            code: KeyCode::PageUp,
            modifiers,
        }),
        "pagedown" | "page_down" | "pgdn" => Ok(KeyChord {
            code: KeyCode::PageDown,
            modifiers,
        }),
        "delete" | "del" => Ok(KeyChord {
            code: KeyCode::Delete,
            modifiers,
        }),
        "backspace" => Ok(KeyChord {
            code: KeyCode::Backspace,
            modifiers,
        }),
        _ => {
            let chars: Vec<char> = s.chars().collect();
            if chars.len() == 1 {
                let ch = chars[0];
                if ch.is_ascii_uppercase() {
                    modifiers.insert(KeyModifiers::SHIFT);
                    Ok(KeyChord {
                        code: KeyCode::Char(ch.to_ascii_lowercase()),
                        modifiers,
                    })
                } else {
                    Ok(KeyChord {
                        code: KeyCode::Char(ch),
                        modifiers,
                    })
                }
            } else {
                Err(format!("unknown key name '{s}'"))
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Keymap {
    bindings: Vec<(Action, Vec<KeyChord>)>,
    mode: KeyMode,
}

impl Default for Keymap {
    fn default() -> Self {
        Self::new(KeyMode::Default)
    }
}

impl Keymap {
    pub fn new(mode: KeyMode) -> Self {
        let bindings = Self::default_bindings(mode);
        Self { bindings, mode }
    }

    pub fn mode(&self) -> KeyMode {
        self.mode
    }

    fn default_bindings(mode: KeyMode) -> Vec<(Action, Vec<KeyChord>)> {
        match mode {
            KeyMode::Default => vec![
                (
                    Action::Quit,
                    vec![KeyChord {
                        code: KeyCode::Char('q'),
                        modifiers: KeyModifiers::NONE,
                    }],
                ),
                (
                    Action::Add,
                    vec![KeyChord {
                        code: KeyCode::Char('a'),
                        modifiers: KeyModifiers::NONE,
                    }],
                ),
                (
                    Action::AddSubtask,
                    vec![KeyChord {
                        code: KeyCode::Char('a'),
                        modifiers: KeyModifiers::SHIFT,
                    }],
                ),
                (
                    Action::Edit,
                    vec![KeyChord {
                        code: KeyCode::Char('e'),
                        modifiers: KeyModifiers::NONE,
                    }],
                ),
                (
                    Action::Toggle,
                    vec![
                        KeyChord {
                            code: KeyCode::Char(' '),
                            modifiers: KeyModifiers::NONE,
                        },
                        KeyChord {
                            code: KeyCode::Enter,
                            modifiers: KeyModifiers::NONE,
                        },
                    ],
                ),
                (
                    Action::Delete,
                    vec![
                        KeyChord {
                            code: KeyCode::Char('d'),
                            modifiers: KeyModifiers::NONE,
                        },
                        KeyChord {
                            code: KeyCode::Delete,
                            modifiers: KeyModifiers::NONE,
                        },
                    ],
                ),
                (
                    Action::ClearCompleted,
                    vec![KeyChord {
                        code: KeyCode::Char('d'),
                        modifiers: KeyModifiers::SHIFT,
                    }],
                ),
                (
                    Action::Undo,
                    vec![KeyChord {
                        code: KeyCode::Char('u'),
                        modifiers: KeyModifiers::NONE,
                    }],
                ),
                (
                    Action::Redo,
                    vec![KeyChord {
                        code: KeyCode::Char('r'),
                        modifiers: KeyModifiers::CONTROL,
                    }],
                ),
                (
                    Action::CarryOver,
                    vec![KeyChord {
                        code: KeyCode::Char('c'),
                        modifiers: KeyModifiers::NONE,
                    }],
                ),
                (
                    Action::MoveToToday,
                    vec![KeyChord {
                        code: KeyCode::Char('m'),
                        modifiers: KeyModifiers::NONE,
                    }],
                ),
                (
                    Action::MoveSelectionUp,
                    vec![
                        KeyChord {
                            code: KeyCode::Char('k'),
                            modifiers: KeyModifiers::NONE,
                        },
                        KeyChord {
                            code: KeyCode::Up,
                            modifiers: KeyModifiers::NONE,
                        },
                    ],
                ),
                (
                    Action::MoveSelectionDown,
                    vec![
                        KeyChord {
                            code: KeyCode::Char('j'),
                            modifiers: KeyModifiers::NONE,
                        },
                        KeyChord {
                            code: KeyCode::Down,
                            modifiers: KeyModifiers::NONE,
                        },
                    ],
                ),
                (
                    Action::ReorderUp,
                    vec![KeyChord {
                        code: KeyCode::Char('k'),
                        modifiers: KeyModifiers::SHIFT,
                    }],
                ),
                (
                    Action::ReorderDown,
                    vec![KeyChord {
                        code: KeyCode::Char('j'),
                        modifiers: KeyModifiers::SHIFT,
                    }],
                ),
                (
                    Action::JumpTop,
                    vec![
                        KeyChord {
                            code: KeyCode::Char('g'),
                            modifiers: KeyModifiers::NONE,
                        },
                        KeyChord {
                            code: KeyCode::Home,
                            modifiers: KeyModifiers::NONE,
                        },
                    ],
                ),
                (
                    Action::JumpBottom,
                    vec![
                        KeyChord {
                            code: KeyCode::Char('g'),
                            modifiers: KeyModifiers::SHIFT,
                        },
                        KeyChord {
                            code: KeyCode::End,
                            modifiers: KeyModifiers::NONE,
                        },
                    ],
                ),
                (
                    Action::HalfPageUp,
                    vec![
                        KeyChord {
                            code: KeyCode::Char('u'),
                            modifiers: KeyModifiers::CONTROL,
                        },
                        KeyChord {
                            code: KeyCode::PageUp,
                            modifiers: KeyModifiers::NONE,
                        },
                    ],
                ),
                (
                    Action::HalfPageDown,
                    vec![
                        KeyChord {
                            code: KeyCode::Char('d'),
                            modifiers: KeyModifiers::CONTROL,
                        },
                        KeyChord {
                            code: KeyCode::PageDown,
                            modifiers: KeyModifiers::NONE,
                        },
                    ],
                ),
                (
                    Action::Help,
                    vec![KeyChord {
                        code: KeyCode::Char('?'),
                        modifiers: KeyModifiers::NONE,
                    }],
                ),
                (
                    Action::SwitchView,
                    vec![
                        KeyChord {
                            code: KeyCode::Tab,
                            modifiers: KeyModifiers::NONE,
                        },
                        KeyChord {
                            code: KeyCode::BackTab,
                            modifiers: KeyModifiers::SHIFT,
                        },
                    ],
                ),
                (
                    Action::FocusTasks,
                    vec![
                        KeyChord {
                            code: KeyCode::Char('l'),
                            modifiers: KeyModifiers::NONE,
                        },
                        KeyChord {
                            code: KeyCode::Right,
                            modifiers: KeyModifiers::NONE,
                        },
                    ],
                ),
                (
                    Action::FocusDates,
                    vec![
                        KeyChord {
                            code: KeyCode::Char('h'),
                            modifiers: KeyModifiers::NONE,
                        },
                        KeyChord {
                            code: KeyCode::Left,
                            modifiers: KeyModifiers::NONE,
                        },
                    ],
                ),
                (
                    Action::Reschedule,
                    vec![KeyChord {
                        code: KeyCode::Char('t'),
                        modifiers: KeyModifiers::NONE,
                    }],
                ),
                (
                    Action::CyclePriority,
                    vec![KeyChord {
                        code: KeyCode::Char('p'),
                        modifiers: KeyModifiers::NONE,
                    }],
                ),
                (
                    Action::CycleSort,
                    vec![KeyChord {
                        code: KeyCode::Char('s'),
                        modifiers: KeyModifiers::NONE,
                    }],
                ),
                (
                    Action::Filter,
                    vec![KeyChord {
                        code: KeyCode::Char('f'),
                        modifiers: KeyModifiers::NONE,
                    }],
                ),
                (
                    Action::ToggleDetail,
                    vec![KeyChord {
                        code: KeyCode::Char('i'),
                        modifiers: KeyModifiers::NONE,
                    }],
                ),
                (
                    Action::EditNote,
                    vec![KeyChord {
                        code: KeyCode::Char('n'),
                        modifiers: KeyModifiers::NONE,
                    }],
                ),
                (
                    Action::ToggleFocus,
                    vec![KeyChord {
                        code: KeyCode::Char('p'),
                        modifiers: KeyModifiers::SHIFT,
                    }],
                ),
                (
                    Action::Recurrence,
                    vec![KeyChord {
                        code: KeyCode::Char('r'),
                        modifiers: KeyModifiers::NONE,
                    }],
                ),
                (
                    Action::ToggleMark,
                    vec![KeyChord {
                        code: KeyCode::Char('v'),
                        modifiers: KeyModifiers::NONE,
                    }],
                ),
                (
                    Action::ToggleMarkAll,
                    vec![KeyChord {
                        code: KeyCode::Char('v'),
                        modifiers: KeyModifiers::SHIFT,
                    }],
                ),
                (
                    Action::Search,
                    vec![KeyChord {
                        code: KeyCode::Char('/'),
                        modifiers: KeyModifiers::NONE,
                    }],
                ),
                (
                    Action::JumpToDate,
                    vec![KeyChord {
                        code: KeyCode::Char(':'),
                        modifiers: KeyModifiers::NONE,
                    }],
                ),
                (
                    Action::ToggleSummaryRange,
                    vec![KeyChord {
                        code: KeyCode::Char('w'),
                        modifiers: KeyModifiers::NONE,
                    }],
                ),
                (
                    Action::PrevPeriod,
                    vec![KeyChord {
                        code: KeyCode::Char('['),
                        modifiers: KeyModifiers::NONE,
                    }],
                ),
                (
                    Action::NextPeriod,
                    vec![KeyChord {
                        code: KeyCode::Char(']'),
                        modifiers: KeyModifiers::NONE,
                    }],
                ),
            ],
            KeyMode::Arrows => vec![
                (
                    Action::Quit,
                    vec![KeyChord {
                        code: KeyCode::Char('q'),
                        modifiers: KeyModifiers::NONE,
                    }],
                ),
                (
                    Action::Add,
                    vec![KeyChord {
                        code: KeyCode::Char('a'),
                        modifiers: KeyModifiers::NONE,
                    }],
                ),
                (
                    Action::AddSubtask,
                    vec![KeyChord {
                        code: KeyCode::Char('a'),
                        modifiers: KeyModifiers::SHIFT,
                    }],
                ),
                (
                    Action::Edit,
                    vec![KeyChord {
                        code: KeyCode::Char('e'),
                        modifiers: KeyModifiers::NONE,
                    }],
                ),
                (
                    Action::Toggle,
                    vec![
                        KeyChord {
                            code: KeyCode::Char(' '),
                            modifiers: KeyModifiers::NONE,
                        },
                        KeyChord {
                            code: KeyCode::Enter,
                            modifiers: KeyModifiers::NONE,
                        },
                    ],
                ),
                (
                    Action::Delete,
                    vec![
                        KeyChord {
                            code: KeyCode::Char('d'),
                            modifiers: KeyModifiers::NONE,
                        },
                        KeyChord {
                            code: KeyCode::Delete,
                            modifiers: KeyModifiers::NONE,
                        },
                    ],
                ),
                (
                    Action::ClearCompleted,
                    vec![KeyChord {
                        code: KeyCode::Char('d'),
                        modifiers: KeyModifiers::SHIFT,
                    }],
                ),
                (
                    Action::Undo,
                    vec![KeyChord {
                        code: KeyCode::Char('u'),
                        modifiers: KeyModifiers::NONE,
                    }],
                ),
                (
                    Action::Redo,
                    vec![KeyChord {
                        code: KeyCode::Char('r'),
                        modifiers: KeyModifiers::CONTROL,
                    }],
                ),
                (
                    Action::CarryOver,
                    vec![KeyChord {
                        code: KeyCode::Char('c'),
                        modifiers: KeyModifiers::NONE,
                    }],
                ),
                (
                    Action::MoveToToday,
                    vec![KeyChord {
                        code: KeyCode::Char('m'),
                        modifiers: KeyModifiers::NONE,
                    }],
                ),
                (
                    Action::MoveSelectionUp,
                    vec![KeyChord {
                        code: KeyCode::Up,
                        modifiers: KeyModifiers::NONE,
                    }],
                ),
                (
                    Action::MoveSelectionDown,
                    vec![KeyChord {
                        code: KeyCode::Down,
                        modifiers: KeyModifiers::NONE,
                    }],
                ),
                (
                    Action::ReorderUp,
                    vec![KeyChord {
                        code: KeyCode::Up,
                        modifiers: KeyModifiers::SHIFT,
                    }],
                ),
                (
                    Action::ReorderDown,
                    vec![KeyChord {
                        code: KeyCode::Down,
                        modifiers: KeyModifiers::SHIFT,
                    }],
                ),
                (
                    Action::JumpTop,
                    vec![KeyChord {
                        code: KeyCode::Home,
                        modifiers: KeyModifiers::NONE,
                    }],
                ),
                (
                    Action::JumpBottom,
                    vec![KeyChord {
                        code: KeyCode::End,
                        modifiers: KeyModifiers::NONE,
                    }],
                ),
                (
                    Action::HalfPageUp,
                    vec![KeyChord {
                        code: KeyCode::PageUp,
                        modifiers: KeyModifiers::NONE,
                    }],
                ),
                (
                    Action::HalfPageDown,
                    vec![KeyChord {
                        code: KeyCode::PageDown,
                        modifiers: KeyModifiers::NONE,
                    }],
                ),
                (
                    Action::Help,
                    vec![KeyChord {
                        code: KeyCode::Char('?'),
                        modifiers: KeyModifiers::NONE,
                    }],
                ),
                (
                    Action::SwitchView,
                    vec![
                        KeyChord {
                            code: KeyCode::Tab,
                            modifiers: KeyModifiers::NONE,
                        },
                        KeyChord {
                            code: KeyCode::BackTab,
                            modifiers: KeyModifiers::SHIFT,
                        },
                    ],
                ),
                (
                    Action::FocusTasks,
                    vec![KeyChord {
                        code: KeyCode::Right,
                        modifiers: KeyModifiers::NONE,
                    }],
                ),
                (
                    Action::FocusDates,
                    vec![KeyChord {
                        code: KeyCode::Left,
                        modifiers: KeyModifiers::NONE,
                    }],
                ),
                (
                    Action::Reschedule,
                    vec![KeyChord {
                        code: KeyCode::Char('t'),
                        modifiers: KeyModifiers::NONE,
                    }],
                ),
                (
                    Action::CyclePriority,
                    vec![KeyChord {
                        code: KeyCode::Char('p'),
                        modifiers: KeyModifiers::NONE,
                    }],
                ),
                (
                    Action::CycleSort,
                    vec![KeyChord {
                        code: KeyCode::Char('s'),
                        modifiers: KeyModifiers::NONE,
                    }],
                ),
                (
                    Action::Filter,
                    vec![KeyChord {
                        code: KeyCode::Char('f'),
                        modifiers: KeyModifiers::NONE,
                    }],
                ),
                (
                    Action::ToggleDetail,
                    vec![KeyChord {
                        code: KeyCode::Char('i'),
                        modifiers: KeyModifiers::NONE,
                    }],
                ),
                (
                    Action::EditNote,
                    vec![KeyChord {
                        code: KeyCode::Char('n'),
                        modifiers: KeyModifiers::NONE,
                    }],
                ),
                (
                    Action::ToggleFocus,
                    vec![KeyChord {
                        code: KeyCode::Char('p'),
                        modifiers: KeyModifiers::SHIFT,
                    }],
                ),
                (
                    Action::Recurrence,
                    vec![KeyChord {
                        code: KeyCode::Char('r'),
                        modifiers: KeyModifiers::NONE,
                    }],
                ),
                (
                    Action::ToggleMark,
                    vec![KeyChord {
                        code: KeyCode::Char('v'),
                        modifiers: KeyModifiers::NONE,
                    }],
                ),
                (
                    Action::ToggleMarkAll,
                    vec![KeyChord {
                        code: KeyCode::Char('v'),
                        modifiers: KeyModifiers::SHIFT,
                    }],
                ),
                (
                    Action::Search,
                    vec![KeyChord {
                        code: KeyCode::Char('/'),
                        modifiers: KeyModifiers::NONE,
                    }],
                ),
                (
                    Action::JumpToDate,
                    vec![KeyChord {
                        code: KeyCode::Char(':'),
                        modifiers: KeyModifiers::NONE,
                    }],
                ),
                (
                    Action::ToggleSummaryRange,
                    vec![KeyChord {
                        code: KeyCode::Char('w'),
                        modifiers: KeyModifiers::NONE,
                    }],
                ),
                (
                    Action::PrevPeriod,
                    vec![KeyChord {
                        code: KeyCode::Char('['),
                        modifiers: KeyModifiers::NONE,
                    }],
                ),
                (
                    Action::NextPeriod,
                    vec![KeyChord {
                        code: KeyCode::Char(']'),
                        modifiers: KeyModifiers::NONE,
                    }],
                ),
            ],
        }
    }

    pub fn from_config(config: &Config) -> (Self, Vec<String>) {
        let mut warnings = Vec::new();
        let mode = config.keys.mode;
        let mut map: HashMap<Action, Vec<KeyChord>> =
            Self::default_bindings(mode).into_iter().collect();

        // Apply overrides
        for (action_key, val) in &config.keys.overrides {
            let Some(action) = Action::from_name(action_key) else {
                warnings.push(format!("Unknown key action '{action_key}'"));
                continue;
            };

            let strings: Vec<String> = match val {
                toml::Value::Array(arr) => {
                    let mut s_vec = Vec::new();
                    let mut valid = true;
                    for item in arr {
                        if let Some(s) = item.as_str() {
                            s_vec.push(s.to_string());
                        } else {
                            warnings.push(format!(
                                "Invalid key binding for action '{action_key}': array item must be string · keeping default"
                            ));
                            valid = false;
                            break;
                        }
                    }
                    if !valid {
                        continue;
                    }
                    s_vec
                }
                toml::Value::String(s) => vec![s.clone()],
                _ => {
                    warnings.push(format!(
                        "Invalid key binding for action '{action_key}': must be a list of strings · keeping default"
                    ));
                    continue;
                }
            };

            let mut chords = Vec::new();
            let mut parse_ok = true;
            for s in strings {
                match KeyChord::parse(&s) {
                    Ok(chord) => {
                        // Ctrl+c protection
                        if chord.code == KeyCode::Char('c')
                            && chord.modifiers.contains(KeyModifiers::CONTROL)
                            && action != Action::Quit
                        {
                            warnings.push(format!(
                                "Ctrl+c is reserved for Quit and cannot be bound to '{}' · keeping default",
                                action.name()
                            ));
                            parse_ok = false;
                            break;
                        }
                        chords.push(chord);
                    }
                    Err(e) => {
                        warnings.push(format!(
                            "Invalid key string '{s}' for action '{action_key}': {e} · keeping default"
                        ));
                        parse_ok = false;
                        break;
                    }
                }
            }

            if parse_ok {
                // An override replaces that action's default bindings
                map.insert(action, chords);
            }
        }

        // Conflict detection across actions
        let mut seen_keys: HashMap<KeyChord, Action> = HashMap::new();
        let mut clean_bindings: Vec<(Action, Vec<KeyChord>)> = Vec::new();

        for action in Action::ALL {
            let chords = map.remove(&action).unwrap_or_default();
            let mut non_conflicting_chords = Vec::new();

            for chord in chords {
                if let Some(earlier_action) = seen_keys.get(&chord) {
                    warnings.push(format!(
                        "Key conflict: '{}' is already bound to '{}', ignoring binding for '{}'",
                        chord.format_hint(),
                        earlier_action.name(),
                        action.name()
                    ));
                } else {
                    seen_keys.insert(chord, action);
                    non_conflicting_chords.push(chord);
                }
            }

            clean_bindings.push((action, non_conflicting_chords));
        }

        (
            Self {
                bindings: clean_bindings,
                mode,
            },
            warnings,
        )
    }

    pub fn resolve(&self, event: &KeyEvent) -> Option<Action> {
        // Safety: Ctrl+c always quits and cannot be unbound
        if event.modifiers.contains(KeyModifiers::CONTROL) && event.code == KeyCode::Char('c') {
            return Some(Action::Quit);
        }

        for (action, chords) in &self.bindings {
            for chord in chords {
                if chord.matches(event) {
                    return Some(*action);
                }
            }
        }

        None
    }

    pub fn hint(&self, action: Action) -> String {
        for (act, chords) in &self.bindings {
            if *act == action {
                if action == Action::FocusTasks
                    && self.mode == KeyMode::Default
                    && chords.len() == 2
                    && chords[0].code == KeyCode::Char('l')
                    && chords[1].code == KeyCode::Right
                {
                    return "enter/l".to_string();
                }
                if let Some(first) = chords.first() {
                    return first.format_hint();
                }
                break;
            }
        }

        match action {
            Action::Quit => "q".to_string(),
            Action::Add => "a".to_string(),
            Action::Edit => "e".to_string(),
            Action::Toggle => "space".to_string(),
            Action::Delete => "d".to_string(),
            Action::ClearCompleted => "D".to_string(),
            Action::Undo => "u".to_string(),
            Action::Redo => "ctrl+r".to_string(),
            Action::CarryOver => "c".to_string(),
            Action::MoveToToday => "m".to_string(),
            Action::MoveSelectionUp => "k".to_string(),
            Action::MoveSelectionDown => "j".to_string(),
            Action::ReorderUp => "K".to_string(),
            Action::ReorderDown => "J".to_string(),
            Action::JumpTop => "g".to_string(),
            Action::JumpBottom => "G".to_string(),
            Action::HalfPageUp => "ctrl+u".to_string(),
            Action::HalfPageDown => "ctrl+d".to_string(),
            Action::Help => "?".to_string(),
            Action::SwitchView => "tab".to_string(),
            Action::FocusTasks => "enter/l".to_string(),
            Action::FocusDates => "h".to_string(),
            Action::Reschedule => "t".to_string(),
            Action::CyclePriority => "p".to_string(),
            Action::CycleSort => "s".to_string(),
            Action::Filter => "f".to_string(),
            Action::ToggleDetail => "i".to_string(),
            Action::EditNote => "n".to_string(),
            Action::AddSubtask => "A".to_string(),
            Action::ToggleFocus => "P".to_string(),
            Action::Recurrence => "r".to_string(),
            Action::ToggleMark => "v".to_string(),
            Action::ToggleMarkAll => "V".to_string(),
            Action::Search => "/".to_string(),
            Action::JumpToDate => ":".to_string(),
            Action::ToggleSummaryRange => "w".to_string(),
            Action::PrevPeriod => "[".to_string(),
            Action::NextPeriod => "]".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_chord_parsing() {
        assert_eq!(
            KeyChord::parse("a").unwrap(),
            KeyChord {
                code: KeyCode::Char('a'),
                modifiers: KeyModifiers::NONE,
            }
        );
        assert_eq!(
            KeyChord::parse("J").unwrap(),
            KeyChord {
                code: KeyCode::Char('j'),
                modifiers: KeyModifiers::SHIFT,
            }
        );
        assert_eq!(
            KeyChord::parse("?").unwrap(),
            KeyChord {
                code: KeyCode::Char('?'),
                modifiers: KeyModifiers::NONE,
            }
        );
        assert_eq!(
            KeyChord::parse("enter").unwrap(),
            KeyChord {
                code: KeyCode::Enter,
                modifiers: KeyModifiers::NONE,
            }
        );
        assert_eq!(
            KeyChord::parse("ctrl+c").unwrap(),
            KeyChord {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::CONTROL,
            }
        );
        assert_eq!(
            KeyChord::parse("ctrl+u").unwrap(),
            KeyChord {
                code: KeyCode::Char('u'),
                modifiers: KeyModifiers::CONTROL,
            }
        );
        assert_eq!(
            KeyChord::parse("shift+up").unwrap(),
            KeyChord {
                code: KeyCode::Up,
                modifiers: KeyModifiers::SHIFT,
            }
        );
        assert_eq!(
            KeyChord::parse("alt+x").unwrap(),
            KeyChord {
                code: KeyCode::Char('x'),
                modifiers: KeyModifiers::ALT,
            }
        );
        assert_eq!(
            KeyChord::parse("space").unwrap(),
            KeyChord {
                code: KeyCode::Char(' '),
                modifiers: KeyModifiers::NONE,
            }
        );
        assert_eq!(
            KeyChord::parse("backtab").unwrap(),
            KeyChord {
                code: KeyCode::BackTab,
                modifiers: KeyModifiers::SHIFT,
            }
        );
        assert!(KeyChord::parse("unknown_key_name").is_err());
        assert!(KeyChord::parse("").is_err());
    }

    #[test]
    fn test_ctrl_c_cannot_be_unbound_and_always_quits() {
        // Even if config tries to bind ctrl+c to Add
        let mut config = Config::default();
        config.keys.overrides.insert(
            "add".to_string(),
            toml::Value::Array(vec![toml::Value::String("ctrl+c".to_string())]),
        );

        let (keymap, warnings) = Keymap::from_config(&config);
        assert!(!warnings.is_empty());
        assert!(warnings[0].contains("Ctrl+c is reserved"));

        // When resolving Ctrl+c, it resolves to Quit
        let event = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(keymap.resolve(&event), Some(Action::Quit));
    }

    #[test]
    fn test_conflict_detection_warns_and_keeps_first() {
        let mut config = Config::default();
        // Override add with 'x', then edit with 'x'
        config.keys.overrides.insert(
            "add".to_string(),
            toml::Value::Array(vec![toml::Value::String("x".to_string())]),
        );
        config.keys.overrides.insert(
            "edit".to_string(),
            toml::Value::Array(vec![toml::Value::String("x".to_string())]),
        );

        let (keymap, warnings) = Keymap::from_config(&config);
        assert!(warnings.iter().any(|w| w.contains("Key conflict: 'x'")));

        // 'x' should trigger Add, not Edit
        let event = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE);
        assert_eq!(keymap.resolve(&event), Some(Action::Add));
    }

    #[test]
    fn test_override_replaces_default_bindings() {
        let mut config = Config::default();
        // Override add with 'n'
        config.keys.overrides.insert(
            "add".to_string(),
            toml::Value::Array(vec![toml::Value::String("n".to_string())]),
        );

        let (keymap, _) = Keymap::from_config(&config);
        assert_eq!(keymap.hint(Action::Add), "n");

        // 'a' should no longer trigger Add
        let event_a = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        assert_ne!(keymap.resolve(&event_a), Some(Action::Add));

        // 'n' should trigger Add
        let event_n = KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE);
        assert_eq!(keymap.resolve(&event_n), Some(Action::Add));
    }

    #[test]
    fn test_arrows_mode() {
        let config = Config {
            keys: crate::config::KeysConfig {
                mode: KeyMode::Arrows,
                overrides: HashMap::new(),
            },
            ..Default::default()
        };

        let (keymap, _) = Keymap::from_config(&config);
        assert_eq!(keymap.mode(), KeyMode::Arrows);

        // 'j' and 'k' must NOT trigger MoveSelectionDown or MoveSelectionUp
        let event_j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        let event_k = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
        assert_ne!(keymap.resolve(&event_j), Some(Action::MoveSelectionDown));
        assert_ne!(keymap.resolve(&event_k), Some(Action::MoveSelectionUp));

        // Down and Up arrows DO trigger MoveSelectionDown and MoveSelectionUp
        let event_down = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        let event_up = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        assert_eq!(keymap.resolve(&event_down), Some(Action::MoveSelectionDown));
        assert_eq!(keymap.resolve(&event_up), Some(Action::MoveSelectionUp));

        // Reorder is Shift+Up / Shift+Down
        let event_shift_up = KeyEvent::new(KeyCode::Up, KeyModifiers::SHIFT);
        let event_shift_down = KeyEvent::new(KeyCode::Down, KeyModifiers::SHIFT);
        assert_eq!(keymap.resolve(&event_shift_up), Some(Action::ReorderUp));
        assert_eq!(keymap.resolve(&event_shift_down), Some(Action::ReorderDown));

        // Action keys like 'a', 'e', 'd', 'u' keep their letters
        let event_a = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        let event_e = KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE);
        let event_d = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE);
        let event_u = KeyEvent::new(KeyCode::Char('u'), KeyModifiers::NONE);
        assert_eq!(keymap.resolve(&event_a), Some(Action::Add));
        assert_eq!(keymap.resolve(&event_e), Some(Action::Edit));
        assert_eq!(keymap.resolve(&event_d), Some(Action::Delete));
        assert_eq!(keymap.resolve(&event_u), Some(Action::Undo));
    }

    #[test]
    fn test_default_keymap_parity() {
        let keymap = Keymap::default();

        // Modifiers checked: Ctrl+d never triggers 'd'
        let event_ctrl_d = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL);
        assert_eq!(keymap.resolve(&event_ctrl_d), Some(Action::HalfPageDown));

        let event_d = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE);
        assert_eq!(keymap.resolve(&event_d), Some(Action::Delete));

        let event_shift_d = KeyEvent::new(KeyCode::Char('D'), KeyModifiers::NONE);
        assert_eq!(keymap.resolve(&event_shift_d), Some(Action::ClearCompleted));

        let event_space = KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE);
        assert_eq!(keymap.resolve(&event_space), Some(Action::Toggle));

        let event_enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(keymap.resolve(&event_enter), Some(Action::Toggle));

        let event_tab = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
        assert_eq!(keymap.resolve(&event_tab), Some(Action::SwitchView));

        let event_backtab = KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT);
        assert_eq!(keymap.resolve(&event_backtab), Some(Action::SwitchView));

        let event_shift_k = KeyEvent::new(KeyCode::Char('K'), KeyModifiers::NONE);
        assert_eq!(keymap.resolve(&event_shift_k), Some(Action::ReorderUp));

        let event_shift_j = KeyEvent::new(KeyCode::Char('J'), KeyModifiers::NONE);
        assert_eq!(keymap.resolve(&event_shift_j), Some(Action::ReorderDown));
    }
}
