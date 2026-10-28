use ratatui::style::Color;

use crate::config::Config;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    pub accent: Color,
    pub text: Color,
    pub dim: Color,
    pub subtext: Color,
    pub divider: Color,
    pub error: Color,
    pub warn: Color,
    pub is_mono: bool,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            accent: Color::Rgb(78, 186, 175), // Soft teal
            text: Color::Reset,               // Terminal default foreground
            dim: Color::DarkGray,             // Dimmed secondary text
            subtext: Color::Gray,             // Slightly brighter for key labels
            divider: Color::DarkGray,         // Light subtle rules
            error: Color::LightRed,           // Non-intrusive warning/error
            warn: Color::Rgb(228, 140, 75),   // Soft orange / warning for overdue
            is_mono: false,
        }
    }
}

impl Theme {
    pub fn from_config(config: &Config) -> Self {
        let mut theme = Self::default();
        if let Some(ref accent_str) = config.accent {
            if let Ok(color) = crate::config::parse_color(accent_str) {
                theme.accent = color;
            }
        }
        theme
    }
}
