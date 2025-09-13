use ratatui::layout::Rect;
use ratatui::prelude::Frame;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::ui::AppState;

/// Draw a top banner with a centered ASCII logo, subtle gradients and side strips.
pub fn draw_banner(f: &mut Frame, area: Rect, state: &AppState) {
    // Bottom border to separate header visually
    let border = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(if state.status_text.is_some() {
            crate::theme::ACCENT
        } else {
            crate::theme::MUTED
        }));
    let inner = border.inner(area);

    // Ambient background (very subtle)
    crate::visuals::draw_ambient_bg(f, inner, &state.theme, state.tick);

    // Centered ASCII logo from state.logo_lines
    let logo = if state.logo_lines.is_empty() {
        vec!["".to_string(), "chi-tui".to_string(), "".to_string()]
    } else {
        state.logo_lines.clone()
    };
    let lw = logo
        .iter()
        .map(|s| s.chars().count() as u16)
        .max()
        .unwrap_or(0);
    let logo_w = lw.min(inner.width);
    let lx = if inner.width > lw {
        inner.x + (inner.width - lw) / 2
    } else {
        inner.x
    };
    // Try to center vertically if there is room, otherwise pin to top
    let needed_h: u16 = logo.len() as u16;
    let ly = if inner.height > needed_h {
        inner.y + (inner.height - needed_h) / 2
    } else {
        inner.y
    };
    let logo_style = Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);
    for (i, s) in logo.iter().enumerate() {
        let area_i = Rect {
            x: lx,
            y: ly.saturating_add(i as u16),
            width: logo_w,
            height: 1,
        };
        let p = Paragraph::new(Line::from(Span::styled(s.clone(), logo_style)))
            .style(Style::default())
            .alignment(ratatui::layout::Alignment::Left)
            .wrap(Wrap { trim: true });
        f.render_widget(p, area_i);
    }

    // Side strips with matrix-like subtle animation
    let palette = [
        crate::theme::PRIMARY,
        crate::theme::ACCENT,
        crate::theme::SECONDARY,
    ];
    let gap: u16 = 1;
    // Left strip
    if lx > inner.x + gap {
        let avail = lx.saturating_sub(inner.x + gap);
        let w = avail.clamp(0, 3);
        if w > 0 {
            let x = lx - gap - w;
            let left = Rect {
                x,
                y: inner.y,
                width: w,
                height: inner.height,
            };
            crate::visuals::draw_matrix_bg_custom(f, left, &palette, state.tick);
        }
    }
    // Right strip
    let rx = lx.saturating_add(logo_w).saturating_add(gap);
    if rx < inner.x.saturating_add(inner.width) {
        let avail = inner.x.saturating_add(inner.width).saturating_sub(rx);
        let w = avail.clamp(0, 3);
        if w > 0 {
            let right = Rect {
                x: rx,
                y: inner.y,
                width: w,
                height: inner.height,
            };
            let palette_r = [
                crate::theme::SECONDARY,
                crate::theme::ACCENT,
                crate::theme::PRIMARY,
            ];
            crate::visuals::draw_matrix_bg_custom(f, right, &palette_r, state.tick);
        }
    }

    // Render the separating bottom border last
    f.render_widget(border, area);
}
