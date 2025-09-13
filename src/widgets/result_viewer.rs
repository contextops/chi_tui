use crate::widgets::chrome::panel_block;
use crossterm::event::KeyCode;
use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

pub struct ResultViewerWidget {
    pub title: String,
    json_pretty: String,
    json_value: serde_json::Value,
    mode_raw: bool,
    wrap: bool,
    scroll_y: u16,
    last_viewport_h: u16,
}

impl ResultViewerWidget {
    pub fn new(title: impl Into<String>, value: serde_json::Value) -> Self {
        let title = title.into();
        let json_pretty =
            serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string());
        Self {
            title,
            json_pretty,
            json_value: value,
            mode_raw: false,
            wrap: false,
            scroll_y: 0,
            last_viewport_h: 0,
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    fn render_value_pretty(&self, v: &serde_json::Value, indent: usize, lines: &mut Vec<Line>) {
        // Skip empty values for a cleaner view
        if is_empty_value(v) {
            return;
        }
        let indent_sp = " ".repeat(indent);
        let arrow_span = || {
            if indent > 0 {
                // subtle arrow marker for nested levels
                vec![Span::styled(
                    "-> ",
                    Style::default().fg(crate::theme::MUTED),
                )]
            } else {
                Vec::new()
            }
        };
        match v {
            serde_json::Value::Null => {
                // null is treated as empty and was filtered above
            }
            serde_json::Value::Bool(b) => {
                let mut parts = vec![Span::raw(indent_sp)];
                parts.extend(arrow_span());
                parts.push(Span::styled(
                    b.to_string(),
                    Style::default().fg(Color::Magenta),
                ));
                lines.push(Line::from(parts));
            }
            serde_json::Value::Number(n) => {
                let mut parts = vec![Span::raw(indent_sp)];
                parts.extend(arrow_span());
                parts.push(Span::styled(
                    n.to_string(),
                    Style::default().fg(Color::Yellow),
                ));
                lines.push(Line::from(parts));
            }
            serde_json::Value::String(s) => {
                if !s.is_empty() {
                    let mut parts = vec![Span::raw(indent_sp)];
                    parts.extend(arrow_span());
                    parts.push(Span::styled(s.clone(), Style::default().fg(Color::Green)));
                    lines.push(Line::from(parts));
                }
            }
            serde_json::Value::Array(arr) => {
                if arr.is_empty() {
                    // skip empty arrays entirely
                    return;
                }
                for item in arr {
                    if is_empty_value(item) {
                        continue;
                    }
                    // Bullet for each item
                    let mut hdr = vec![Span::raw(indent_sp.clone())];
                    hdr.extend(arrow_span());
                    hdr.push(Span::raw("• "));
                    let mut sublines: Vec<Line> = Vec::new();
                    match item {
                        serde_json::Value::Object(obj) if obj.contains_key("title") => {
                            let title = obj.get("title").and_then(|s| s.as_str()).unwrap_or("");
                            hdr.push(Span::styled(
                                title.to_string(),
                                Style::default().fg(Color::Cyan),
                            ));
                            lines.push(Line::from(hdr));
                            // Render rest of fields indented (skip empties)
                            let mut other = obj.clone();
                            other.remove("title");
                            for (k, v) in other.iter() {
                                if is_empty_value(v) || is_technical_field(k, v) {
                                    continue;
                                }
                                let mut l = vec![
                                    Span::raw(" ".repeat(indent + 2)),
                                    // show arrow for nested entries
                                    Span::styled("-> ", Style::default().fg(crate::theme::MUTED)),
                                    Span::styled(
                                        format!("{k}: "),
                                        Style::default().fg(Color::Cyan),
                                    ),
                                ];
                                l.push(value_preview_span(v));
                                lines.push(Line::from(l));
                            }
                        }
                        _ => {
                            // Render simple or nested value with additional indent
                            lines.push(Line::from(hdr));
                            self.render_value_pretty(item, indent + 2, &mut sublines);
                            lines.extend(sublines);
                        }
                    }
                }
            }
            serde_json::Value::Object(map) => {
                if map.is_empty() {
                    // skip empty objects entirely
                    return;
                }
                let mut keys: Vec<&String> = map
                    .iter()
                    .filter(|(k, v)| !is_empty_value(v) && !is_technical_field(k, v))
                    .map(|(k, _)| k)
                    .collect();
                keys.sort();
                for k in keys {
                    let v = &map[k];
                    if is_empty_value(v) || is_technical_field(k, v) {
                        continue;
                    }
                    let mut l = vec![Span::raw(indent_sp.clone())];
                    l.extend(arrow_span());
                    l.push(Span::styled(
                        format!("{k}: "),
                        Style::default().fg(Color::Cyan),
                    ));
                    match v {
                        serde_json::Value::Object(_) | serde_json::Value::Array(_) => {
                            lines.push(Line::from(l));
                            self.render_value_pretty(v, indent + 2, lines);
                        }
                        _ => {
                            l.push(value_preview_span(v));
                            lines.push(Line::from(l));
                        }
                    }
                }
            }
        }
    }
}

fn value_preview_span(v: &serde_json::Value) -> Span<'static> {
    match v {
        serde_json::Value::Null => Span::styled("null", Style::default().fg(crate::theme::MUTED)),
        serde_json::Value::Bool(b) => {
            Span::styled(b.to_string(), Style::default().fg(Color::Magenta))
        }
        serde_json::Value::Number(n) => {
            Span::styled(n.to_string(), Style::default().fg(Color::Yellow))
        }
        serde_json::Value::String(s) => Span::styled(s.clone(), Style::default().fg(Color::Green)),
        serde_json::Value::Array(arr) => Span::styled(
            format!("[{} items]", arr.len()),
            Style::default().fg(crate::theme::MUTED),
        ),
        serde_json::Value::Object(map) => Span::styled(
            format!("{{{} keys}}", map.len()),
            Style::default().fg(crate::theme::MUTED),
        ),
    }
}

fn is_empty_value(v: &serde_json::Value) -> bool {
    match v {
        serde_json::Value::Null => true,
        serde_json::Value::String(s) => s.is_empty(),
        serde_json::Value::Array(arr) => arr.is_empty(),
        serde_json::Value::Object(map) => map.is_empty(),
        _ => false,
    }
}

fn is_technical_field(key: &str, v: &serde_json::Value) -> bool {
    match key {
        // Always skip these common metadata fields
        "version" | "ts" | "request_id" => true,
        // Skip ok=true, but keep ok=false to highlight errors
        "ok" => v.as_bool().unwrap_or(false),
        // Often a transport/result envelope marker; omit from pretty view
        "type" => true,
        _ => false,
    }
}

impl crate::widgets::Widget for ResultViewerWidget {
    fn render(&mut self, f: &mut Frame, area: Rect, focused: bool, _tick: u64) {
        // Build lines according to mode
        let mut lines: Vec<Line> = Vec::new();
        if self.mode_raw {
            for l in self.json_pretty.lines() {
                lines.push(Line::from(l.to_string()));
            }
        } else {
            // Optional first hint line
            lines.push(Line::from(vec![Span::styled(
                "Press j to toggle raw JSON  •  Backspace to go back",
                Style::default().fg(crate::theme::MUTED),
            )]));
            self.render_value_pretty(&self.json_value, 0, &mut lines);
        }
        // Viewport calcs
        self.last_viewport_h = area.height.saturating_sub(2);
        let total = lines.len() as u16;
        let max_scroll = total.saturating_sub(self.last_viewport_h);
        if self.scroll_y > max_scroll {
            self.scroll_y = max_scroll;
        }
        let block = panel_block(&self.title, focused);
        let p = Paragraph::new(lines)
            .block(block)
            .wrap(ratatui::widgets::Wrap { trim: !self.wrap })
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
            KeyCode::Down => self.scroll_y = self.scroll_y.saturating_add(1),
            KeyCode::PageUp => {
                let step = self.last_viewport_h;
                self.scroll_y = self.scroll_y.saturating_sub(step);
            }
            KeyCode::PageDown => {
                let step = self.last_viewport_h;
                self.scroll_y = self.scroll_y.saturating_add(step);
            }
            KeyCode::Home => self.scroll_y = 0,
            KeyCode::End => {
                let max_scroll =
                    self.json_pretty
                        .lines()
                        .count()
                        .saturating_sub(self.last_viewport_h as usize) as u16;
                self.scroll_y = max_scroll;
            }
            KeyCode::Char('w') | KeyCode::Char('W') => self.wrap = !self.wrap,
            KeyCode::Char('j') | KeyCode::Char('J') => {
                // Toggle raw/pretty
                self.mode_raw = !self.mode_raw;
                // Reset scroll to top to avoid confusing jumps
                self.scroll_y = 0;
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
