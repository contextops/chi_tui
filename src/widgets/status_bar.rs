use ratatui::prelude::*;
use ratatui::text::{Line, Span};
use ratatui::widgets::*;

use crate::ui::AppState;

pub fn draw_status(f: &mut Frame, area: Rect, state: &AppState) {
    let mut spans: Vec<Span> = Vec::new();
    if let Some(msg) = &state.status_text {
        let spinner = ["⠋", "⠙", "⠸", "⠴", "⠦", "⠇"][state.tick as usize % 6];
        spans.push(Span::raw(format!(" {spinner} {msg}")));
        if let Some(p) = state.status_percent {
            spans.push(Span::raw(format!(" — {p:>5.1}%")));
        }
    }
    if let Some(t) = &state.toast {
        if !spans.is_empty() {
            spans.push(Span::raw("  |  "));
        }
        let color = crate::theme::toast_color(t.level);
        let tag = match t.level {
            crate::ui::ToastLevel::Success => "[OK]",
            crate::ui::ToastLevel::Error => "[ERROR]",
            crate::ui::ToastLevel::Info => "[INFO]",
        };
        spans.push(Span::styled(
            format!("{tag} "),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(t.text.clone(), Style::default().fg(color)));
    }
    if matches!(state.view, crate::ui::View::Panel) {
        let focus = if let Some(ps) = &state.panel {
            if matches!(state.panel_focus, crate::ui::PanelPane::B) {
                if let crate::ui::PaneContent::Panel(_) = ps.b_content {
                    match state.panel_nested_focus {
                        crate::ui::PanelPane::A => "B.A",
                        crate::ui::PanelPane::B => "B.B",
                    }
                } else {
                    "B"
                }
            } else {
                "A"
            }
        } else {
            "A"
        };
        if !spans.is_empty() {
            spans.push(Span::raw("  |  "));
        }
        spans.push(Span::styled(
            format!("focus: {focus}"),
            Style::default().fg(Color::Magenta),
        ));
        // Editing indicator (form editing in Pane B)
        if let Some(ps) = &state.panel {
            if let crate::ui::PaneContent::Widget(w) = &ps.b_content {
                if let Some(fw) = w
                    .as_any()
                    .downcast_ref::<crate::widgets::form_widget::FormWidget>()
                {
                    if fw.form.editing {
                        spans.push(Span::raw("  |  editing"));
                    }
                }
            }
        }
    }
    let p = Paragraph::new(Line::from(spans)).style(Style::default().fg(Color::Magenta));
    f.render_widget(p, area);
}

pub fn draw_footer_combined(f: &mut Frame, area: Rect, state: &AppState, help_text: &str) {
    let mut spans: Vec<Span> = Vec::new();
    if let Some(msg) = &state.status_text {
        let spinner = ["⠋", "⠙", "⠸", "⠴", "⠦", "⠇"][state.tick as usize % 6];
        spans.push(Span::raw(format!(" {spinner} {msg}")));
        if let Some(p) = state.status_percent {
            spans.push(Span::raw(format!(" — {p:>5.1}%")));
        }
        spans.push(Span::raw("  |  "));
    }
    if let Some(t) = &state.toast {
        let color = crate::theme::toast_color(t.level);
        let tag = match t.level {
            crate::ui::ToastLevel::Success => "[OK]",
            crate::ui::ToastLevel::Error => "[ERROR]",
            crate::ui::ToastLevel::Info => "[INFO]",
        };
        spans.push(Span::styled(
            format!("{tag} "),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            format!("{}  |  ", t.text),
            Style::default().fg(color),
        ));
    }
    if matches!(state.view, crate::ui::View::Panel) {
        let focus = if let Some(ps) = &state.panel {
            if matches!(state.panel_focus, crate::ui::PanelPane::B) {
                if let crate::ui::PaneContent::Panel(_) = ps.b_content {
                    match state.panel_nested_focus {
                        crate::ui::PanelPane::A => "B.A",
                        crate::ui::PanelPane::B => "B.B",
                    }
                } else {
                    "B"
                }
            } else {
                "A"
            }
        } else {
            "A"
        };
        spans.push(Span::styled(
            format!("focus: {focus}"),
            Style::default().fg(Color::Magenta),
        ));
        // Editing indicator
        if let Some(ps) = &state.panel {
            if let crate::ui::PaneContent::Widget(w) = &ps.b_content {
                if let Some(fw) = w
                    .as_any()
                    .downcast_ref::<crate::widgets::form_widget::FormWidget>()
                {
                    if fw.form.editing {
                        spans.push(Span::raw("  |  editing"));
                    }
                }
            }
        }
        spans.push(Span::raw("  |  "));
    }
    spans.push(Span::styled(
        help_text.to_string(),
        Style::default().fg(Color::DarkGray),
    ));
    let p = Paragraph::new(Line::from(spans));
    f.render_widget(p, area);
}
