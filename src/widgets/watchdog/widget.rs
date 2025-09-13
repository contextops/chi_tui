use super::config::WatchdogConfig;
use super::session::{CmdLog, WatchdogSessionRef};
use super::util::push_line;
use super::StatsAggregator;
use crate::widgets::chrome::panel_block;
use crossterm::event::KeyCode;
use ratatui::prelude::*;
use ratatui::style::Modifier;
use ratatui::text::Span;
use ratatui::widgets::*;
use std::sync::Arc;

pub struct WatchdogWidget {
    #[allow(dead_code)]
    title: String,
    pub cmds: Vec<CmdLog>,
    scroll_offsets: Vec<u16>,
    last_viewport_h: u16,
    #[allow(dead_code)]
    cfg: WatchdogConfig,
    stats: Option<StatsAggregator>,
    // Session reference (source of truth for process lifecycle)
    session: WatchdogSessionRef,
    // When true, keep the view pinned to the latest output (bottom).
    auto_follow: bool,
    // Focused subpane index (when this widget is focused in Pane B)
    focused_idx: usize,
}

impl WatchdogWidget {
    // Create a fresh session and attach to it.
    pub fn new(title: impl Into<String>, commands: Vec<String>, cfg: WatchdogConfig) -> Self {
        let session = super::session::WatchdogSession::create(commands.clone(), cfg.clone());
        // Build view of command outputs
        let cmds: Vec<CmdLog> = {
            let s = session.lock().unwrap();
            s.cmds
                .iter()
                .map(|c| CmdLog {
                    cmd: c.cmd.clone(),
                    output: Arc::clone(&c.output),
                })
                .collect()
        };
        let scroll_offsets = vec![0u16; cmds.len()];
        let stats = if cfg.stats.is_empty() {
            None
        } else {
            Some(StatsAggregator::new(&cfg.stats, cmds.len()))
        };
        Self {
            title: title.into(),
            cmds,
            scroll_offsets,
            last_viewport_h: 0,
            cfg,
            stats,
            session,
            auto_follow: true,
            focused_idx: 0,
        }
    }

    // Attach to an existing session (do not spawn new processes)
    pub fn from_session(title: impl Into<String>, session: &WatchdogSessionRef) -> Self {
        // Snapshot view of commands (Arc buffers ensure live updates)
        let (cmds, cfg) = {
            let s = session.lock().unwrap();
            (
                s.cmds
                    .iter()
                    .map(|c| CmdLog {
                        cmd: c.cmd.clone(),
                        output: Arc::clone(&c.output),
                    })
                    .collect::<Vec<_>>(),
                s.cfg.clone(),
            )
        };
        let scroll_offsets = vec![0u16; cmds.len()];
        let stats = if cfg.stats.is_empty() {
            None
        } else {
            Some(StatsAggregator::new(&cfg.stats, cmds.len()))
        };
        let widget = Self {
            title: title.into(),
            cmds,
            scroll_offsets,
            last_viewport_h: 0,
            cfg,
            stats,
            session: Arc::clone(session),
            auto_follow: true,
            focused_idx: 0,
        };
        // Add visible notice
        for c in &widget.cmds {
            push_line(&c.output, "[re-attached to running session]".to_string());
        }
        widget
    }

    // Expose the underlying session for external coordination/registration
    pub fn session_ref(&self) -> WatchdogSessionRef {
        Arc::clone(&self.session)
    }
}

impl crate::widgets::Widget for WatchdogWidget {
    fn render(&mut self, f: &mut Frame, area: Rect, focused: bool, _tick: u64) {
        // Reserve footer area for stats if configured
        let stats_h: u16 = self.stats.as_ref().map(|s| s.len() as u16).unwrap_or(0);
        let mut logs_area = area;
        if stats_h > 0 && logs_area.height > stats_h {
            logs_area.height = logs_area.height.saturating_sub(stats_h);
        }

        // Split logs area into N vertical chunks
        let n = self.cmds.len().max(1) as u16;
        let pct = (100 / n).max(1);
        let mut constraints: Vec<Constraint> = Vec::new();
        for _ in 0..n - 1 {
            constraints.push(Constraint::Percentage(pct));
        }
        // Last chunk eats the remainder
        let used = pct * (n - 1);
        constraints.push(Constraint::Percentage(100 - used));
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(logs_area);

        self.last_viewport_h = logs_area.height.saturating_sub(2);

        for (i, (cmd, chunk)) in self.cmds.iter().zip(chunks.iter()).enumerate() {
            // clamp scroll per section based on total length
            let (_total_len, mut visible_lines): (usize, Vec<Line>) =
                if let Ok(q) = cmd.output.lock() {
                    let total = q.len();
                    // Viewport height (minus borders)
                    let viewport = chunk.height.saturating_sub(2);
                    let max_scroll = (total as u16).saturating_sub(viewport);
                    if let Some(off) = self.scroll_offsets.get_mut(i) {
                        if self.auto_follow || *off > max_scroll {
                            *off = max_scroll;
                        }
                    }
                    let start = self.scroll_offsets[i] as usize;
                    let end = start.saturating_add(viewport as usize).min(total);
                    let mut lines: Vec<Line> = Vec::with_capacity(end.saturating_sub(start));
                    for s in q.iter().skip(start).take(end.saturating_sub(start)) {
                        lines.push(Line::from(s.clone()));
                    }
                    (total, lines)
                } else {
                    (0usize, Vec::new())
                };

            // Render the visible slice
            let block = panel_block(&cmd.cmd, focused && self.focused_idx == i);
            let p = Paragraph::new(std::mem::take(&mut visible_lines)).block(block);
            f.render_widget(p, *chunk);
        }

        // Footer: stats + controls
        if stats_h > 0 {
            if let Some(aggr) = self.stats.as_mut() {
                let buffers: Vec<_> = self.cmds.iter().map(|c| Arc::clone(&c.output)).collect();
                aggr.update_from_buffers(&buffers);
            }
            // Prepare footer area (aligned to bottom of the widget area)
            let footer_y = area.y + area.height.saturating_sub(stats_h);
            let footer_area = Rect {
                x: area.x,
                y: footer_y,
                width: area.width,
                height: stats_h,
            };
            // Subtle background for the stats footer
            let bg = Block::default().style(Style::default().bg(Color::Rgb(24, 24, 24)));
            f.render_widget(bg, footer_area);

            // Helper to pick a color based on the label
            let color_for = |label: &str| -> Color {
                let l = label.to_ascii_lowercase();
                if l.contains("error") || l.contains("err") {
                    Color::Red
                } else if l.contains("warn") {
                    Color::Yellow
                } else if l.contains("info") {
                    Color::Cyan
                } else if l.contains("debug") {
                    Color::Blue
                } else {
                    Color::Gray
                }
            };

            // Stats lines
            let (labels, counts) = if let Some(aggr) = self.stats.as_ref() {
                (aggr.labels(), aggr.counts().to_vec())
            } else {
                (Vec::new(), Vec::new())
            };
            for (i, label) in labels.iter().enumerate() {
                let y = footer_y + i as u16;
                let rect = Rect {
                    x: area.x + 1,
                    y,
                    width: area.width.saturating_sub(2),
                    height: 1,
                };
                let col = color_for(label);
                // Format: ● LABEL  × COUNT
                let line = Line::from(vec![
                    Span::styled("●", Style::default().fg(col)),
                    Span::raw(" "),
                    Span::styled(
                        label.clone(),
                        Style::default().fg(col).add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("  × "),
                    Span::styled(
                        counts.get(i).copied().unwrap_or(0).to_string(),
                        Style::default().fg(Color::White),
                    ),
                ]);
                let p = Paragraph::new(line).style(Style::default());
                f.render_widget(p, rect);
            }
        }
    }

    fn on_key(&mut self, key: KeyCode) -> Vec<crate::app::Effect> {
        match key {
            KeyCode::Char('r') => {
                // Restart: in external mode not supported; otherwise clear buffers and spawn again
                if let Ok(mut s) = self.session.lock() {
                    if s.external {
                        return vec![crate::app::Effect::ShowToast {
                            text: "External mode: restart not supported".to_string(),
                            level: crate::ui::ToastLevel::Info,
                            seconds: 2,
                        }];
                    } else {
                        s.restart_all(true);
                        return vec![crate::app::Effect::ShowToast {
                            text: "Watchdog restarting...".to_string(),
                            level: crate::ui::ToastLevel::Info,
                            seconds: 2,
                        }];
                    }
                }
                // Fallthrough: no session
            }
            KeyCode::Char('s') => {
                // Toggle start/stop or kill in external mode
                if let Ok(mut s) = self.session.lock() {
                    if s.external {
                        let ok = s.kill_external();
                        if ok {
                            for c in &self.cmds {
                                push_line(&c.output, "[external] kill invoked".to_string());
                            }
                            return vec![crate::app::Effect::ShowToast {
                                text: "External kill invoked".to_string(),
                                level: crate::ui::ToastLevel::Info,
                                seconds: 2,
                            }];
                        } else {
                            return vec![crate::app::Effect::ShowToast {
                                text: "External mode: no kill command configured".to_string(),
                                level: crate::ui::ToastLevel::Error,
                                seconds: 3,
                            }];
                        }
                    } else if s.started {
                        // Note in each pane that stop was requested
                        for c in &self.cmds {
                            push_line(&c.output, "[stop requested]".to_string());
                        }
                        s.stop_all();
                        return vec![crate::app::Effect::ShowToast {
                            text: "Watchdog stop requested".to_string(),
                            level: crate::ui::ToastLevel::Info,
                            seconds: 2,
                        }];
                    } else {
                        s.start();
                        return vec![crate::app::Effect::ShowToast {
                            text: "Watchdog started".to_string(),
                            level: crate::ui::ToastLevel::Success,
                            seconds: 2,
                        }];
                    }
                }
                // Fallthrough: no session
            }
            KeyCode::Char('f') | KeyCode::End => {
                // Resume auto-follow and jump to bottom on next render
                self.auto_follow = true;
                return vec![crate::app::Effect::ShowToast {
                    text: "Auto-follow resumed".to_string(),
                    level: crate::ui::ToastLevel::Success,
                    seconds: 2,
                }];
            }
            _ => {}
        }
        // Scroll all subpanes together (global scroll)
        match key {
            KeyCode::Up => {
                self.auto_follow = false;
                for off in &mut self.scroll_offsets {
                    if *off > 0 {
                        *off -= 1;
                    }
                }
            }
            KeyCode::Down => {
                self.auto_follow = false;
                for off in &mut self.scroll_offsets {
                    *off = off.saturating_add(1);
                }
            }
            KeyCode::PageUp => {
                let step = self.last_viewport_h;
                self.auto_follow = false;
                for off in &mut self.scroll_offsets {
                    *off = off.saturating_sub(step);
                }
            }
            KeyCode::PageDown => {
                let step = self.last_viewport_h;
                self.auto_follow = false;
                for off in &mut self.scroll_offsets {
                    *off = off.saturating_add(step);
                }
            }
            KeyCode::Home => {
                self.auto_follow = false;
                for off in &mut self.scroll_offsets {
                    *off = 0;
                }
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

// Inherent methods for external focus control (used by UI Tab cycling)
impl WatchdogWidget {
    pub fn pane_count(&self) -> usize {
        self.cmds.len()
    }
    pub fn focused_pane(&self) -> usize {
        self.focused_idx.min(self.cmds.len().saturating_sub(1))
    }
    pub fn set_focused_pane(&mut self, idx: usize) {
        if self.cmds.is_empty() {
            self.focused_idx = 0;
        } else {
            self.focused_idx = idx.min(self.cmds.len() - 1);
        }
    }
}
