use crate::widgets::chrome::panel_block;
use crate::widgets::result_viewer::ResultViewerWidget;
use crate::widgets::Widget;
use crossterm::event::KeyCode;
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::ui::AppState;

pub fn draw_json(f: &mut Frame, area: Rect, state: &mut AppState) {
    if let Some(err) = &state.last_error {
        // Show error in simple paragraph
        let lines = vec![
            Line::from(err.clone()).style(Style::default().fg(Color::Red)),
            Line::from(""),
        ];
        let block = panel_block("JSON Output", !matches!(state.view, crate::ui::View::Panel));
        let p = Paragraph::new(lines).block(block);
        f.render_widget(p, area);
        return;
    }
    // Ensure viewer is present; seed from last_json_pretty if needed
    if state.json_viewer.is_none() {
        if let Some(txt) = &state.last_json_pretty {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(txt) {
                state.json_viewer = Some(ResultViewerWidget::new("JSON Output", v));
            }
        }
    }
    if let Some(w) = &mut state.json_viewer {
        // Render pretty viewer
        w.render(
            f,
            area,
            // Single main view => highlight frame
            !matches!(state.view, crate::ui::View::Panel),
            0,
        );
    } else {
        // Fallback: nothing to render
        let block = panel_block("JSON Output", !matches!(state.view, crate::ui::View::Panel));
        let p = Paragraph::new("").block(block);
        f.render_widget(p, area);
    }
}

pub struct JsonViewerWidget {
    pub title: String,
    pub error: Option<String>,
    pub text: String,
    pub scroll_y: u16,
    pub wrap: bool,
    last_viewport_h: u16,
}

impl JsonViewerWidget {
    pub fn from_text(title: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            error: None,
            text: text.into(),
            scroll_y: 0,
            wrap: false,
            last_viewport_h: 0,
        }
    }
    #[allow(dead_code)]
    pub fn from_error(title: impl Into<String>, err: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            error: Some(err.into()),
            text: String::new(),
            scroll_y: 0,
            wrap: false,
            last_viewport_h: 0,
        }
    }
}

impl crate::widgets::Widget for JsonViewerWidget {
    fn render(&mut self, f: &mut Frame, area: Rect, focused: bool, _tick: u64) {
        let mut lines: Vec<Line> = Vec::new();
        if let Some(err) = &self.error {
            lines.push(Line::from(err.clone()).style(Style::default().fg(Color::Red)));
            lines.push(Line::from(""));
        }
        for l in self.text.lines() {
            lines.push(Line::from(l.to_string()));
        }
        // viewport
        self.last_viewport_h = area.height.saturating_sub(2);
        let total_lines = lines.len() as u16;
        let max_scroll = total_lines.saturating_sub(self.last_viewport_h);
        if self.scroll_y > max_scroll {
            self.scroll_y = max_scroll;
        }
        let block = panel_block(&self.title, focused);
        let p = Paragraph::new(lines)
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
                let mut total: u16 = 0;
                if self.error.is_some() {
                    total = total.saturating_add(2);
                }
                total = total.saturating_add(self.text.lines().count() as u16);
                let max_scroll = total.saturating_sub(self.last_viewport_h);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::Widget; // bring trait in scope for render/on_key
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    #[test]
    fn end_jumps_to_bottom_and_w_toggles_wrap() {
        let text = (0..30)
            .map(|i| format!("line-{i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut w = JsonViewerWidget::from_text("JSON", text);
        // Initial render to capture viewport height (12 → 10 inner, but Paragraph uses area.height - 2)
        let backend = TestBackend::new(40, 12);
        let mut terminal = Terminal::new(backend).unwrap();
        let _ = terminal.draw(|f| {
            let area = ratatui::layout::Rect {
                x: 0,
                y: 0,
                width: 40,
                height: 12,
            };
            w.render(f, area, true, 0);
        });
        // End key → scroll to bottom
        let _ = w.on_key(KeyCode::End);
        let expected_max = (30u16).saturating_sub(w.last_viewport_h);
        assert_eq!(w.scroll_y, expected_max);
        // Toggle wrap
        assert!(!w.wrap);
        let _ = w.on_key(KeyCode::Char('w'));
        assert!(w.wrap);
    }
}
