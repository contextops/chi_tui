use ratatui::style::{Color, Modifier, Style};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ThemeMode {
    #[allow(dead_code)]
    Light,
    Dark,
}

#[derive(Clone, Debug)]
pub struct Theme {
    #[allow(dead_code)]
    pub mode: ThemeMode,
    pub bg: Color,
    #[allow(dead_code)]
    pub fg: Color,
    pub primary: Color,
    pub secondary: Color,
    pub accent: Color,
    #[allow(dead_code)]
    pub frame: Color,
    pub selected: Color,
    pub success: Color,
    pub error: Color,
    pub muted: Color,
}

impl Theme {
    pub fn synthwave_dark() -> Self {
        Self {
            mode: ThemeMode::Dark,
            bg: Color::Rgb(24, 24, 26),
            fg: Color::White,
            primary: Color::Rgb(255, 0, 153),
            secondary: Color::Rgb(0, 255, 255),
            accent: Color::Rgb(64, 160, 255),
            frame: Color::Rgb(90, 90, 100),
            selected: Color::Rgb(255, 120, 0),
            success: Color::Green,
            error: Color::Red,
            muted: Color::DarkGray,
        }
    }

    #[allow(dead_code)]
    pub fn synthwave_light() -> Self {
        Self {
            mode: ThemeMode::Light,
            bg: Color::Rgb(245, 245, 247),
            fg: Color::Rgb(20, 20, 22),
            primary: Color::Rgb(200, 0, 120),
            secondary: Color::Rgb(0, 160, 160),
            accent: Color::Rgb(40, 120, 220),
            frame: Color::Rgb(200, 200, 210),
            selected: Color::Rgb(220, 100, 0),
            success: Color::Rgb(0, 150, 0),
            error: Color::Rgb(200, 0, 0),
            muted: Color::Rgb(120, 120, 130),
        }
    }

    #[allow(dead_code)]
    pub fn from_mode(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Dark => Self::synthwave_dark(),
            ThemeMode::Light => Self::synthwave_light(),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::synthwave_dark()
    }
}

// Style helpers that use the theme
impl Theme {
    pub fn border_focused(&self) -> Style {
        Style::default().fg(self.selected)
    }

    #[allow(dead_code)]
    pub fn border_unfocused(&self) -> Style {
        Style::default().fg(self.frame)
    }

    pub fn text_active_bold(&self) -> Style {
        Style::default()
            .fg(self.accent)
            .add_modifier(Modifier::BOLD)
    }

    pub fn text_editing_bold(&self) -> Style {
        Style::default()
            .fg(self.selected)
            .add_modifier(Modifier::BOLD)
    }

    pub fn text_muted(&self) -> Style {
        Style::default().fg(self.muted)
    }

    pub fn text_error(&self) -> Style {
        Style::default().fg(self.error)
    }

    #[allow(dead_code)]
    pub fn text_success(&self) -> Style {
        Style::default().fg(self.success)
    }

    pub fn list_cursor_style(&self) -> Style {
        Style::default()
            .fg(self.bg)
            .bg(self.selected)
            .add_modifier(Modifier::BOLD)
    }

    #[allow(dead_code)]
    pub fn panel_style(&self, active: bool) -> Style {
        let border_color = if active { self.selected } else { self.frame };
        Style::default().fg(border_color)
    }

    #[allow(dead_code)]
    pub fn title_style(&self) -> Style {
        Style::default().fg(self.accent)
    }

    #[allow(dead_code)]
    pub fn base_style(&self) -> Style {
        Style::default().bg(self.bg).fg(self.fg)
    }

    pub fn toast_color(&self, level: crate::ui::ToastLevel) -> Color {
        match level {
            crate::ui::ToastLevel::Success => self.success,
            crate::ui::ToastLevel::Error => self.error,
            crate::ui::ToastLevel::Info => self.accent,
        }
    }
}

// Legacy compatibility mappings
pub const ACCENT: Color = Color::Rgb(64, 160, 255);
pub const PRIMARY: Color = Color::Rgb(255, 0, 153);
pub const SECONDARY: Color = Color::Rgb(0, 255, 255);
pub const ACTIVE: Color = Color::Cyan;
#[allow(dead_code)]
pub const SUCCESS: Color = Color::Green;
#[allow(dead_code)]
pub const ERROR: Color = Color::Red;
pub const MUTED: Color = Color::DarkGray;
#[allow(dead_code)]
pub const INVERT: Color = Color::Black;

// Legacy helper functions that now use default theme
pub fn border_focused() -> Style {
    Theme::default().border_focused()
}

pub fn text_active_bold() -> Style {
    Theme::default().text_active_bold()
}

pub fn text_editing_bold() -> Style {
    Theme::default().text_editing_bold()
}

pub fn text_muted() -> Style {
    Theme::default().text_muted()
}

pub fn text_error() -> Style {
    Theme::default().text_error()
}

pub fn toast_color(level: crate::ui::ToastLevel) -> Color {
    Theme::default().toast_color(level)
}

pub fn list_cursor_style() -> Style {
    Theme::default().list_cursor_style()
}
