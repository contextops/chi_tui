use crossterm::event::KeyCode;
use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::nav::flatten::flatten_nodes;
use crate::nav::keys::menu_key;
use crate::ui::AppState;
use crate::widgets::chrome::panel_block;

#[allow(dead_code)]
pub(crate) fn compute_scroll_window(total: usize, selected: usize, inner_h: u16) -> (usize, usize) {
    if inner_h == 0 || total == 0 {
        return (0, 0);
    }
    let sel = selected.min(total.saturating_sub(1));
    let ih = inner_h as usize;
    let start = sel.saturating_sub(ih - 1);
    let end = (start + ih).min(total);
    (start, end)
}

pub fn draw_menu(f: &mut Frame, area: Rect, state: &AppState) {
    let nodes = flatten_nodes(state);
    // Use persistent offset window; adjusted by key handlers in ui.rs
    let inner_h = area.height.saturating_sub(2); // account for borders
    let total = nodes.len();
    let ih = inner_h as usize;
    let max_start = total.saturating_sub(ih);
    let start = state.menu_offset.min(max_start);
    let end = (start + ih).min(total);
    let items: Vec<ListItem> = nodes
        .iter()
        .enumerate()
        .skip(start)
        .take(end - start)
        .map(|(idx, node)| {
            let is_sel = idx == state.selected;
            let sel = if is_sel { "> " } else { "  " };
            match node {
                crate::ui::FlatNode::Header { idx, depth } => {
                    let m = &state.config.menu[*idx];
                    let indent = "  ".repeat(*depth);
                    let text = format!("{sel}{indent}{}", m.title);
                    let mut item = ListItem::new(text);
                    item = item.style(Style::default().fg(Color::Yellow));
                    item
                }
                crate::ui::FlatNode::Menu { idx, depth } => {
                    let m = &state.config.menu[*idx];
                    let indent = "  ".repeat(*depth);
                    let mut text = m.title.clone();
                    if crate::ui::is_lazy(m) {
                        let hint = m
                            .initial_text
                            .clone()
                            .unwrap_or_else(|| "Press Enter to load".to_string());
                        let key = menu_key(m);
                        let chevron = if state.expanded.contains(&key) {
                            "▾"
                        } else {
                            "▸"
                        };
                        text = if state.loading.contains(&key) {
                            let spinner = ["⠋", "⠙", "⠸", "⠴", "⠦", "⠇"][state.tick as usize % 6];
                            format!("{chevron} {text} ({spinner} loading) — {hint}")
                        } else if state.children.contains_key(&key) {
                            format!("{chevron} {text} (loaded) — {hint}")
                        } else {
                            format!("{chevron} {text} — {hint}")
                        };
                    } else if crate::ui::is_autoload(m) {
                        let key = menu_key(m);
                        let chevron = if state.expanded.contains(&key) {
                            "▾"
                        } else {
                            "▸"
                        };
                        let on_enter = crate::ui::expand_on_enter_menu(m);
                        text = if state.loading.contains(&key) {
                            let spinner = ["⠋", "⠙", "⠸", "⠴", "⠦", "⠇"][state.tick as usize % 6];
                            format!("{chevron} {text} ({spinner} loading)")
                        } else if state.children.contains_key(&key) {
                            format!("{chevron} {text} (auto-loaded)")
                        } else if on_enter {
                            format!("{chevron} {text} (press Enter)")
                        } else {
                            format!("{chevron} {text} (auto)")
                        };
                    }
                    // Watchdog running indicator (blinking orange '* running...' or '(external init)')
                    if crate::ui::is_watchdog(m) {
                        let key = menu_key(m);
                        if let Some(s) = state.watchdog_sessions.get(&key) {
                            if let Ok(g) = s.lock() {
                                let internal = g.started;
                                let external = g.external && g.external_running;
                                if internal || external {
                                    let blink_on = (state.tick / 2) % 2 == 0; // slower blink
                                    let star = if blink_on { "*" } else { " " };
                                    let mut spans: Vec<Span<'_>> = Vec::new();
                                    spans.push(Span::raw(format!("{sel}{indent}{text}  ")));
                                    spans.push(Span::styled(
                                        star,
                                        Style::default()
                                            .fg(Color::Rgb(255, 140, 0))
                                            .add_modifier(Modifier::BOLD),
                                    ));
                                    if external && !internal {
                                        spans.push(Span::raw(" running (external init)"));
                                    } else {
                                        spans.push(Span::raw(" running..."));
                                    }
                                    return ListItem::new(Line::from(spans));
                                }
                            }
                        }
                    } else if crate::ui::is_panel(m) {
                        // If a panel spawns a watchdog (via YAML/spec), we register its session
                        // under the parent menu key. Show running indicator here as well, including
                        // nested subpane sessions keyed as "menu:<id>/nested:A|B".
                        let key = menu_key(m);
                        let mut status: Option<&'static str> = None;
                        if let Some(s) = state.watchdog_sessions.get(&key) {
                            if let Ok(g) = s.lock() {
                                if g.external && g.external_running && !g.started {
                                    status = Some("running (external init)");
                                } else if g.started {
                                    status = Some("running...");
                                }
                            }
                        }
                        if status.is_none() {
                            let prefix = format!("{key}/nested:");
                            for (k, s) in &state.watchdog_sessions {
                                if k.starts_with(&prefix) {
                                    if let Ok(g) = s.lock() {
                                        if g.external && g.external_running && !g.started {
                                            status = Some("running (external init)");
                                            break;
                                        } else if g.started {
                                            status = Some("running...");
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                        if let Some(lbl) = status {
                            let blink_on = (state.tick / 2) % 2 == 0; // slower blink
                            let star = if blink_on { "*" } else { " " };
                            let mut spans: Vec<Span<'_>> = Vec::new();
                            spans.push(Span::raw(format!("{sel}{indent}{text}  ")));
                            spans.push(Span::styled(
                                star,
                                Style::default()
                                    .fg(Color::Rgb(255, 140, 0))
                                    .add_modifier(Modifier::BOLD),
                            ));
                            spans.push(Span::raw(format!(" {lbl}")));
                            return ListItem::new(Line::from(spans));
                        }
                    }
                    ListItem::new(format!("{sel}{indent}{text}"))
                }
                crate::ui::FlatNode::Child { key, val, depth } => {
                    let indent = "  ".repeat(*depth);
                    let title = crate::ui::title_from_value(val);
                    if crate::ui::is_lazy_value(val) {
                        let hint =
                            crate::ui::initial_text_value(val).unwrap_or("Press Enter to load");
                        let chevron = if state.expanded.contains(key) {
                            "▾"
                        } else {
                            "▸"
                        };
                        let text = if state.loading.contains(key) {
                            let spinner = ["⠋", "⠙", "⠸", "⠴", "⠦", "⠇"][state.tick as usize % 6];
                            format!("{chevron} {title} ({spinner} loading) — {hint}")
                        } else if state.children.contains_key(key) {
                            format!("{chevron} {title} (loaded) — {hint}")
                        } else {
                            format!("{chevron} {title} — {hint}")
                        };
                        ListItem::new(format!("{sel}{indent}{text}"))
                    } else if crate::ui::is_autoload_value(val) {
                        let chevron = if state.expanded.contains(key) {
                            "▾"
                        } else {
                            "▸"
                        };
                        let on_enter = crate::ui::expand_on_enter_value(val);
                        let text = if state.loading.contains(key) {
                            let spinner = ["⠋", "⠙", "⠸", "⠴", "⠦", "⠇"][state.tick as usize % 6];
                            format!("{chevron} {title} ({spinner} loading)")
                        } else if state.children.contains_key(key) {
                            format!("{chevron} {title} (auto-loaded)")
                        } else if on_enter {
                            format!("{chevron} {title} (press Enter)")
                        } else {
                            format!("{chevron} {title} (auto)")
                        };
                        ListItem::new(format!("{sel}{indent}{text}"))
                    } else {
                        // Meta elements styling: pagination controls and page info
                        let is_pagination = val
                            .get("__is_pagination")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);
                        let is_info = val
                            .get("__is_info")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);

                        if is_pagination {
                            // Pagination controls: neutral/muted color, no bullet prefix
                            ListItem::new(format!("{sel}{indent}{title}"))
                                .style(Style::default().fg(crate::theme::MUTED))
                        } else if is_info {
                            // Muted color, no bullet prefix
                            ListItem::new(format!("{sel}{indent}{title}"))
                                .style(Style::default().fg(crate::theme::MUTED))
                        } else {
                            // Default children rendering with a simple bullet
                            // Add watchdog running indicator for child items that are watchdog specs
                            let is_watchdog_child = val
                                .get("widget")
                                .and_then(|s| s.as_str())
                                .map(|w| w.eq_ignore_ascii_case("watchdog"))
                                .unwrap_or(false)
                                || val
                                    .get("type")
                                    .and_then(|s| s.as_str())
                                    .map(|w| w.eq_ignore_ascii_case("watchdog"))
                                    .unwrap_or(false);
                            if is_watchdog_child {
                                // Derive parent menu key from child key: "menu:<parent_id>/..."
                                let parent_key = key
                                    .split('/')
                                    .next()
                                    .map(|s| s.to_string())
                                    .unwrap_or_else(|| key.clone());
                                // Running indicator for exact child session or parent-level session
                                let running_label = {
                                    let mut label: Option<&'static str> = None;
                                    if let Some(s) = state.watchdog_sessions.get(key) {
                                        if let Ok(g) = s.lock() {
                                            if g.external && g.external_running && !g.started {
                                                label = Some("running (external init)");
                                            } else if g.started {
                                                label = Some("running...");
                                            }
                                        }
                                    }
                                    if label.is_none() {
                                        if let Some(s) = state.watchdog_sessions.get(&parent_key) {
                                            if let Ok(g) = s.lock() {
                                                if g.external && g.external_running && !g.started {
                                                    label = Some("running (external init)");
                                                } else if g.started {
                                                    label = Some("running...");
                                                }
                                            }
                                        }
                                    }
                                    label
                                };
                                if let Some(lbl) = running_label {
                                    let blink_on = (state.tick / 2) % 2 == 0;
                                    let star = if blink_on { "*" } else { " " };
                                    let line = Line::from(vec![
                                        Span::raw(format!("{sel}{indent}• {title}  ")),
                                        Span::styled(
                                            star,
                                            Style::default()
                                                .fg(Color::Rgb(255, 140, 0))
                                                .add_modifier(Modifier::BOLD),
                                        ),
                                        Span::raw(format!(" {lbl}")),
                                    ]);
                                    return ListItem::new(line);
                                }
                            }
                            ListItem::new(format!("{sel}{indent}• {title}"))
                        }
                    }
                }
            }
        })
        .collect();
    let block = panel_block(
        "Menu",
        // Rule: always highlight when it's the only panel (view != Panel)
        // or when focus is on Pane A in Panel mode
        !matches!(state.view, crate::ui::View::Panel)
            || matches!(state.panel_focus, crate::ui::PanelPane::A),
    );
    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

pub struct MenuWidget {
    pub title: String,
    pub config: crate::model::AppConfig,
    pub selected: usize,
    pub offset: usize,
    last_viewport_h: u16,
}

impl MenuWidget {
    pub fn from_config(title: impl Into<String>, config: crate::model::AppConfig) -> Self {
        Self {
            title: title.into(),
            config,
            selected: 0,
            offset: 0,
            last_viewport_h: 0,
        }
    }
    fn keep_selected_visible(&mut self) {
        let ih = self.last_viewport_h as usize;
        if ih == 0 {
            self.offset = 0;
            return;
        }
        if self.selected < self.offset {
            self.offset = self.selected;
        } else if self.selected >= self.offset.saturating_add(ih) {
            self.offset = self.selected.saturating_sub(ih.saturating_sub(1));
        }
    }
}

impl crate::widgets::Widget for MenuWidget {
    fn render(&mut self, f: &mut Frame, area: Rect, focused: bool, _tick: u64) {
        let inner_h = area.height.saturating_sub(2);
        self.last_viewport_h = inner_h;
        if self.selected > self.config.menu.len().saturating_sub(1) {
            self.selected = self.config.menu.len().saturating_sub(1);
        }
        self.keep_selected_visible();
        let ih = inner_h as usize;
        let total = self.config.menu.len();
        let max_start = total.saturating_sub(ih);
        let start = self.offset.min(max_start);
        let end = (start + ih).min(total);
        let items: Vec<ListItem> = self
            .config
            .menu
            .iter()
            .enumerate()
            .skip(start)
            .take(end - start)
            .map(|(i, m)| {
                let sel_mark = if self.selected == i { "> " } else { "  " };
                let mut text = format!("{}{}", sel_mark, m.title);
                if let Some(w) = &m.widget {
                    match w.as_str() {
                        "panel" => text.push_str(" [panel]"),
                        "lazy_items" => text.push_str(" [lazy]"),
                        "autoload_items" => text.push_str(" [autoload]"),
                        _ => {}
                    }
                } else if m.command.is_some() {
                    text.push_str(" [cmd]");
                }
                ListItem::new(text)
            })
            .collect();
        let block = panel_block(&self.title, focused);
        let list = List::new(items).block(block);
        f.render_widget(list, area);
    }
    fn on_key(&mut self, key: KeyCode) -> Vec<crate::app::Effect> {
        let total = self.config.menu.len();
        match key {
            KeyCode::Up => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
                self.keep_selected_visible();
            }
            KeyCode::Down => {
                if !self.config.menu.is_empty() && self.selected + 1 < total {
                    self.selected += 1;
                }
                self.keep_selected_visible();
            }
            KeyCode::PageUp => {
                let step = self.last_viewport_h as usize;
                if step > 0 {
                    self.selected = self.selected.saturating_sub(step);
                }
                self.keep_selected_visible();
            }
            KeyCode::PageDown => {
                let step = self.last_viewport_h as usize;
                if step > 0 {
                    self.selected = (self.selected + step).min(total.saturating_sub(1));
                }
                self.keep_selected_visible();
            }
            KeyCode::Home => {
                self.selected = 0;
                self.keep_selected_visible();
            }
            KeyCode::End => {
                if total > 0 {
                    self.selected = total - 1;
                }
                self.keep_selected_visible();
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
    use super::compute_scroll_window;

    #[test]
    fn window_keeps_selected_visible() {
        // total 20, height 5 → window size 5
        let (s1, e1) = compute_scroll_window(20, 0, 5);
        assert_eq!((s1, e1), (0, 5));
        let (s2, e2) = compute_scroll_window(20, 4, 5);
        assert_eq!((s2, e2), (0, 5));
        let (s3, e3) = compute_scroll_window(20, 5, 5);
        assert_eq!((s3, e3), (1, 6));
        let (s4, e4) = compute_scroll_window(20, 19, 5);
        assert_eq!((s4, e4), (15, 20));
    }

    #[test]
    fn menu_widget_scroll_and_bounds() {
        use crate::widgets::Widget;
        use crossterm::event::KeyCode;
        use ratatui::prelude::Rect;
        let cfg = crate::model::AppConfig {
            header: None,
            menu: (0..20)
                .map(|i| crate::model::MenuItem {
                    id: format!("id{i}"),
                    title: format!("Item {i}"),
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        };
        let mut w = super::MenuWidget::from_config("Pane B — Menu", cfg);
        // Simulate first render at height 8 (inner 6)
        let backend = ratatui::backend::TestBackend::new(40, 8);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let _ = terminal.draw(|f| {
            let area = Rect {
                x: 0,
                y: 0,
                width: 40,
                height: 8,
            };
            w.render(f, area, true, 0);
        });
        // PageDown should advance roughly by viewport height
        let _ = w.on_key(KeyCode::PageDown);
        assert!(w.selected >= 5);
        // End should clamp to last item
        let _ = w.on_key(KeyCode::End);
        assert_eq!(w.selected, 19);
        // Next Down stays clamped
        let _ = w.on_key(KeyCode::Down);
        assert_eq!(w.selected, 19);
        // Home goes back to 0
        let _ = w.on_key(KeyCode::Home);
        assert_eq!(w.selected, 0);
    }
}
