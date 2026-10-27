use ratatui::style::Color;

// ==============================================================================
// THEME CONFIGURATION
// To swap the accent color, change the `accent` field below:
// - Soft Teal:   Color::Rgb(78, 186, 175)  (Default)
// - Soft Orange: Color::Rgb(228, 140, 75)
// - Soft Purple: Color::Rgb(180, 140, 220)
// ==============================================================================
pub struct Theme {
    pub accent: Color,
    pub text: Color,
    pub dim: Color,
    pub subtext: Color,
    pub divider: Color,
    pub error: Color,
    pub warn: Color,
}

pub const THEME: Theme = Theme {
    accent: Color::Rgb(78, 186, 175), // Soft teal
    text: Color::Reset,               // Terminal default foreground
    dim: Color::DarkGray,             // Dimmed secondary text
    subtext: Color::Gray,             // Slightly brighter for key labels
    divider: Color::DarkGray,         // Light subtle rules
    error: Color::LightRed,           // Non-intrusive warning/error
    warn: Color::Rgb(228, 140, 75),   // Soft orange / warning for overdue
};
