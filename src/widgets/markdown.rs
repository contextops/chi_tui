use crate::widgets::chrome::panel_block;
use crossterm::event::KeyCode;
use ratatui::prelude::*;
use ratatui::widgets::*;
use std::sync::OnceLock;

// syntect setup (lazy)
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style as SynStyle, Theme, ThemeSet};
use syntect::parsing::{SyntaxReference, SyntaxSet};

/// Minimal Markdown viewer
/// MVP:
/// - Headers (#, ##, ###) styled bold
/// - Code blocks (``` ... ```) styled with a distinct color
/// - Simple paragraph lines otherwise
pub struct MarkdownWidget {
    title: String,
    lines: Vec<Line<'static>>,
    scroll_y: u16,
    wrap: bool,
    last_viewport_h: u16,
    pub raw_content: String,
}

impl MarkdownWidget {
    pub fn from_text(title: impl Into<String>, text: &str) -> Self {
        let raw_content = text.to_string();
        let mut lines: Vec<Line<'static>> = Vec::new();
        // Parse line by line and syntax-highlight fenced code blocks using syntect
        let mut in_code = false;
        let mut code_buf: Vec<String> = Vec::new();
        let mut code_lang: Option<String> = None;
        for raw in text.lines() {
            let trimmed = raw.trim_end_matches('\r');
            if trimmed.starts_with("```") {
                if in_code {
                    // flush code_buf as highlighted lines
                    let code_text = code_buf.join("\n");
                    let mut hlines = highlight_code(&code_text, code_lang.as_deref());
                    lines.append(&mut hlines);
                    code_buf.clear();
                    code_lang = None;
                } else {
                    // opening fence can specify language: ```rust
                    let lang = trimmed.trim_start_matches("```").trim();
                    if !lang.is_empty() {
                        code_lang = Some(lang.to_string());
                    }
                }
                in_code = !in_code;
                // Show fence line faint
                lines.push(Line::from(Span::styled(
                    trimmed.to_string(),
                    Style::default().fg(Color::DarkGray),
                )));
                continue;
            }
            if in_code {
                code_buf.push(trimmed.to_string());
                continue;
            }
            // Headings and plain lines
            if trimmed.starts_with("### ")
                || trimmed.starts_with("## ")
                || trimmed.starts_with("# ")
            {
                lines.push(Line::from(Span::styled(
                    trimmed.to_string(),
                    Style::default().add_modifier(Modifier::BOLD),
                )));
            } else {
                lines.push(Line::from(trimmed.to_string()));
            }
        }
        // If file ended within a code block, flush it
        if in_code && !code_buf.is_empty() {
            let code_text = code_buf.join("\n");
            let mut hlines = highlight_code(&code_text, code_lang.as_deref());
            lines.append(&mut hlines);
        }
        Self {
            title: title.into(),
            lines,
            scroll_y: 0,
            wrap: true,
            last_viewport_h: 0,
            raw_content,
        }
    }

    pub fn from_path(title: impl Into<String>, path: &std::path::Path) -> Self {
        let content = std::fs::read_to_string(path)
            .unwrap_or_else(|_| format!("# Error\nFailed to read file: {}", path.display()));
        Self::from_text(title, &content)
    }
}

// ---------------- Syntax highlighting helpers ----------------
static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
static THEME_SET: OnceLock<ThemeSet> = OnceLock::new();
static THEME: OnceLock<Theme> = OnceLock::new();

fn get_syntax_set() -> &'static SyntaxSet {
    SYNTAX_SET.get_or_init(SyntaxSet::load_defaults_newlines)
}
fn get_theme() -> &'static Theme {
    THEME.get_or_init(|| {
        let ts = THEME_SET.get_or_init(ThemeSet::load_defaults);
        // Choose a widely available theme
        ts.themes
            .get("base16-ocean.dark")
            .cloned()
            .unwrap_or_else(|| ts.themes.values().next().cloned().unwrap_or_default())
    })
}

fn syn_to_tui_color(c: syntect::highlighting::Color) -> Color {
    Color::Rgb(c.r, c.g, c.b)
}

fn highlight_code(code: &str, lang: Option<&str>) -> Vec<Line<'static>> {
    let ps = get_syntax_set();
    let theme = get_theme();
    let syn: &SyntaxReference = match lang {
        Some(l) if !l.is_empty() => ps
            .find_syntax_by_token(l)
            .unwrap_or_else(|| ps.find_syntax_plain_text()),
        _ => ps.find_syntax_plain_text(),
    };
    let mut high = HighlightLines::new(syn, theme);
    let mut out: Vec<Line<'static>> = Vec::new();
    for line in code.split('\n') {
        let regions: Vec<(SynStyle, &str)> = high.highlight_line(line, ps).unwrap_or_default();
        let mut spans: Vec<Span<'static>> = Vec::new();
        for (st, seg) in regions {
            let mut style = Style::default().fg(syn_to_tui_color(st.foreground));
            if st
                .font_style
                .contains(syntect::highlighting::FontStyle::BOLD)
            {
                style = style.add_modifier(Modifier::BOLD);
            }
            if st
                .font_style
                .contains(syntect::highlighting::FontStyle::ITALIC)
            {
                style = style.add_modifier(Modifier::ITALIC);
            }
            // convert seg to owned String for 'static
            spans.push(Span::styled(seg.to_string(), style));
        }
        out.push(Line::from(spans));
    }
    out
}

impl crate::widgets::Widget for MarkdownWidget {
    fn render(&mut self, f: &mut Frame, area: Rect, focused: bool, _tick: u64) {
        self.last_viewport_h = area.height.saturating_sub(2);
        let total_lines = self.lines.len() as u16;
        let max_scroll = total_lines.saturating_sub(self.last_viewport_h);
        if self.scroll_y > max_scroll {
            self.scroll_y = max_scroll;
        }
        let block = panel_block(&self.title, focused);
        let p = Paragraph::new(self.lines.clone())
            .block(block)
            .wrap(Wrap { trim: !self.wrap })
            .scroll((self.scroll_y, 0));
        f.render_widget(p, area);
    }

    fn on_key(&mut self, key: KeyCode) -> Vec<crate::app::Effect> {
        match key {
            KeyCode::Up => {
                if self.scroll_y > 0 {
                    self.scroll_y -= 1;
                }
            }
            KeyCode::Down => {
                self.scroll_y = self.scroll_y.saturating_add(1);
            }
            KeyCode::PageUp => {
                let step = self.last_viewport_h;
                self.scroll_y = self.scroll_y.saturating_sub(step);
            }
            KeyCode::PageDown => {
                let step = self.last_viewport_h;
                self.scroll_y = self.scroll_y.saturating_add(step);
            }
            KeyCode::Home => {
                self.scroll_y = 0;
            }
            KeyCode::End => {
                let max_scroll = (self.lines.len() as u16).saturating_sub(self.last_viewport_h);
                self.scroll_y = max_scroll;
            }
            KeyCode::Char('w') | KeyCode::Char('W') => {
                self.wrap = !self.wrap;
            }
            _ => {}
        }
        Vec::new()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
