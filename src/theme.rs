use ratatui::style::{Color, Modifier, Style};

use crate::config::{Config, EnvLookup, SystemEnv};

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
        Self::preset_teal()
    }
}

impl Theme {
    pub fn preset_teal() -> Self {
        Self {
            accent: Color::Rgb(78, 186, 175), // Soft teal
            text: Color::Reset,
            dim: Color::DarkGray,
            subtext: Color::Gray,
            divider: Color::DarkGray,
            error: Color::LightRed,
            warn: Color::Rgb(228, 140, 75),
            is_mono: false,
        }
    }

    pub fn preset_orange() -> Self {
        Self {
            accent: Color::Rgb(228, 140, 75), // Soft orange
            text: Color::Reset,
            dim: Color::DarkGray,
            subtext: Color::Gray,
            divider: Color::DarkGray,
            error: Color::LightRed,
            warn: Color::Rgb(228, 140, 75),
            is_mono: false,
        }
    }

    pub fn preset_lavender() -> Self {
        Self {
            accent: Color::Rgb(180, 140, 220), // Soft lavender
            text: Color::Reset,
            dim: Color::DarkGray,
            subtext: Color::Gray,
            divider: Color::DarkGray,
            error: Color::LightRed,
            warn: Color::Rgb(228, 140, 75),
            is_mono: false,
        }
    }

    pub fn preset_mono() -> Self {
        Self {
            accent: Color::Reset,
            text: Color::Reset,
            dim: Color::Reset,
            subtext: Color::Reset,
            divider: Color::Reset,
            error: Color::Reset,
            warn: Color::Reset,
            is_mono: true,
        }
    }

    pub fn from_config(config: &Config) -> Self {
        Self::from_config_with_env(config, &SystemEnv)
    }

    pub fn from_config_with_env(config: &Config, env: &impl EnvLookup) -> Self {
        let no_color = env
            .get("NO_COLOR")
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);

        if no_color {
            return Self::preset_mono();
        }

        let mut theme = match config.theme.to_ascii_lowercase().as_str() {
            "orange" => Self::preset_orange(),
            "lavender" => Self::preset_lavender(),
            "mono" => Self::preset_mono(),
            _ => Self::preset_teal(),
        };

        if let Some(ref accent_str) = config.accent {
            if let Ok(color) = crate::config::parse_color(accent_str) {
                theme.accent = color;
            }
        }

        let truecolor = supports_truecolor(env);
        theme.accent = map_color_to_truecolor(theme.accent, truecolor);
        theme.warn = map_color_to_truecolor(theme.warn, truecolor);

        theme
    }

    pub fn text_style(&self) -> Style {
        if self.is_mono {
            Style::default().fg(Color::Reset)
        } else {
            Style::default().fg(self.text)
        }
    }

    pub fn accent_style(&self) -> Style {
        if self.is_mono {
            Style::default()
                .fg(Color::Reset)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(self.accent)
        }
    }

    pub fn dim_style(&self) -> Style {
        if self.is_mono {
            Style::default()
                .fg(Color::Reset)
                .add_modifier(Modifier::DIM)
        } else {
            Style::default().fg(self.dim)
        }
    }

    pub fn subtext_style(&self) -> Style {
        if self.is_mono {
            Style::default().fg(Color::Reset)
        } else {
            Style::default().fg(self.subtext)
        }
    }

    pub fn divider_style(&self) -> Style {
        if self.is_mono {
            Style::default()
                .fg(Color::Reset)
                .add_modifier(Modifier::DIM)
        } else {
            Style::default().fg(self.divider)
        }
    }

    pub fn error_style(&self) -> Style {
        if self.is_mono {
            Style::default()
                .fg(Color::Reset)
                .add_modifier(Modifier::BOLD | Modifier::REVERSED)
        } else {
            Style::default().fg(self.error)
        }
    }

    pub fn warn_style(&self) -> Style {
        if self.is_mono {
            Style::default()
                .fg(Color::Reset)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(self.warn)
        }
    }

    pub fn priority_1_style(&self) -> Style {
        if self.is_mono {
            Style::default()
                .fg(Color::Reset)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(self.accent)
        }
    }

    pub fn priority_2_style(&self) -> Style {
        if self.is_mono {
            Style::default()
                .fg(Color::Reset)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else {
            Style::default().fg(self.warn)
        }
    }
}

pub fn supports_truecolor(env: &impl EnvLookup) -> bool {
    if let Some(ct) = env.get("COLORTERM") {
        let ct_lower = ct.trim().to_ascii_lowercase();
        ct_lower == "truecolor" || ct_lower == "24bit"
    } else {
        false
    }
}

pub fn rgb_to_ansi256(r: u8, g: u8, b: u8) -> u8 {
    let r_i = r as i32;
    let g_i = g as i32;
    let b_i = b as i32;

    let mut best_idx = 16u8;
    let mut best_dist = i32::MAX;

    // 6x6x6 color cube: indices 16..=231
    let cube_steps = [0, 95, 135, 175, 215, 255];
    for (ri, &cr) in cube_steps.iter().enumerate() {
        for (gi, &cg) in cube_steps.iter().enumerate() {
            for (bi, &cb) in cube_steps.iter().enumerate() {
                let dist =
                    (r_i - cr) * (r_i - cr) + (g_i - cg) * (g_i - cg) + (b_i - cb) * (b_i - cb);
                if dist < best_dist {
                    best_dist = dist;
                    best_idx = 16 + (ri as u8) * 36 + (gi as u8) * 6 + (bi as u8);
                }
            }
        }
    }

    // 24 grayscale ramp: indices 232..=255
    for i in 0..24 {
        let gray = 8 + i * 10;
        let dist =
            (r_i - gray) * (r_i - gray) + (g_i - gray) * (g_i - gray) + (b_i - gray) * (b_i - gray);
        if dist < best_dist {
            best_dist = dist;
            best_idx = 232 + i as u8;
        }
    }

    best_idx
}

fn map_color_to_truecolor(color: Color, truecolor: bool) -> Color {
    if truecolor {
        color
    } else {
        match color {
            Color::Rgb(r, g, b) => Color::Indexed(rgb_to_ansi256(r, g, b)),
            other => other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::MapEnv;
    use std::collections::HashMap;

    #[test]
    fn test_theme_presets() {
        let env_truecolor = MapEnv(HashMap::from([("COLORTERM", "truecolor".to_string())]));

        let config_teal = Config {
            theme: "teal".to_string(),
            ..Default::default()
        };
        let teal = Theme::from_config_with_env(&config_teal, &env_truecolor);
        assert_eq!(teal.accent, Color::Rgb(78, 186, 175));
        assert!(!teal.is_mono);

        let config_orange = Config {
            theme: "orange".to_string(),
            ..Default::default()
        };
        let orange = Theme::from_config_with_env(&config_orange, &env_truecolor);
        assert_eq!(orange.accent, Color::Rgb(228, 140, 75));
        assert!(!orange.is_mono);

        let config_lavender = Config {
            theme: "lavender".to_string(),
            ..Default::default()
        };
        let lavender = Theme::from_config_with_env(&config_lavender, &env_truecolor);
        assert_eq!(lavender.accent, Color::Rgb(180, 140, 220));
        assert!(!lavender.is_mono);

        let config_mono = Config {
            theme: "mono".to_string(),
            ..Default::default()
        };
        let mono = Theme::from_config_with_env(&config_mono, &env_truecolor);
        assert!(mono.is_mono);
        // mono has no Color::Rgb anywhere
        let colors = [
            mono.accent,
            mono.text,
            mono.dim,
            mono.subtext,
            mono.divider,
            mono.error,
            mono.warn,
        ];
        for c in colors {
            assert!(!matches!(c, Color::Rgb(_, _, _)));
        }
    }

    #[test]
    fn test_accent_override() {
        let env = MapEnv(HashMap::from([("COLORTERM", "truecolor".to_string())]));
        let config = Config {
            theme: "teal".to_string(),
            accent: Some("#123456".to_string()),
            ..Default::default()
        };

        let theme = Theme::from_config_with_env(&config, &env);
        assert_eq!(theme.accent, Color::Rgb(0x12, 0x34, 0x56));
    }

    #[test]
    fn test_rgb_to_ansi256_mapping() {
        assert_eq!(rgb_to_ansi256(0, 0, 0), 16);
        assert_eq!(rgb_to_ansi256(255, 255, 255), 231);
        assert_eq!(rgb_to_ansi256(255, 0, 0), 196);
        assert_eq!(rgb_to_ansi256(0, 255, 0), 46);
        assert_eq!(rgb_to_ansi256(0, 0, 255), 21);
    }

    #[test]
    fn test_truecolor_fallback_when_colorterm_missing() {
        let env_no_truecolor = MapEnv(HashMap::new());
        let config = Config::default(); // default teal: Rgb(78, 186, 175)
        let theme = Theme::from_config_with_env(&config, &env_no_truecolor);

        // Should have mapped Rgb accent to Color::Indexed
        assert!(matches!(theme.accent, Color::Indexed(_)));
        assert_eq!(theme.accent, Color::Indexed(rgb_to_ansi256(78, 186, 175)));
    }

    #[test]
    fn test_no_color_strips_all_colors() {
        let env_no_color = MapEnv(HashMap::from([("NO_COLOR", "1".to_string())]));
        let config = Config {
            theme: "orange".to_string(),
            accent: Some("#123456".to_string()),
            ..Default::default()
        };
        let theme = Theme::from_config_with_env(&config, &env_no_color);
        assert!(theme.is_mono);
        let colors = [
            theme.accent,
            theme.text,
            theme.dim,
            theme.subtext,
            theme.divider,
            theme.error,
            theme.warn,
        ];
        for c in colors {
            assert_eq!(c, Color::Reset);
        }

        // Empty NO_COLOR should NOT strip colors
        let env_empty_no_color = MapEnv(HashMap::from([
            ("NO_COLOR", "".to_string()),
            ("COLORTERM", "truecolor".to_string()),
        ]));
        let theme_not_stripped = Theme::from_config_with_env(&config, &env_empty_no_color);
        assert!(!theme_not_stripped.is_mono);
        assert_eq!(theme_not_stripped.accent, Color::Rgb(0x12, 0x34, 0x56));
    }
}
