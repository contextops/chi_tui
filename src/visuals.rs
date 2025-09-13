use ratatui::layout::Rect;
use ratatui::prelude::Frame;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Padding, Paragraph, Wrap};

use crate::theme::Theme;

#[allow(dead_code)]
pub fn panel_block(active: bool, theme: &Theme) -> Block<'static> {
    let border = if active { theme.selected } else { theme.frame };
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border))
        .padding(Padding::new(1, 1, 1, 1))
}

#[allow(dead_code)]
pub fn panel_block_with_title<'a>(active: bool, theme: &Theme, title: Line<'a>) -> Block<'a> {
    panel_block(active, theme).title(title)
}

#[allow(dead_code)]
pub fn title_line(_active: bool, text: &str, theme: &Theme, _tick: u64) -> Line<'static> {
    Line::from(Span::styled(
        text.to_string(),
        Style::default().fg(theme.accent),
    ))
}

pub fn spinner_head(tick: u64) -> char {
    const SPINNERS: [char; 8] = ['⣾', '⣽', '⣻', '⢿', '⡿', '⣟', '⣯', '⣷'];
    SPINNERS[(tick as usize / 2) % SPINNERS.len()]
}

/// Draw a subtle animated ambient background in the given area.
/// A dim dotted pattern that slowly shifts over time.
pub fn draw_ambient_bg(f: &mut Frame, area: Rect, theme: &Theme, tick: u64) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let mut lines: Vec<Line> = Vec::with_capacity(area.height as usize);
    let phase = (tick % 16) as u16;
    for y in 0..area.height {
        let mut s = String::with_capacity(area.width as usize);
        for x in 0..area.width {
            let v = (x + y + phase) % 8;
            if v == 0 {
                s.push('·');
            } else {
                s.push(' ');
            }
        }
        lines.push(Line::from(Span::styled(
            s,
            Style::default()
                .bg(theme.bg)
                .fg(theme.muted)
                .add_modifier(Modifier::DIM),
        )));
    }
    let p = Paragraph::new(lines)
        .style(Style::default().bg(theme.bg))
        .wrap(Wrap { trim: false });
    f.render_widget(p, area);
}

/// Draw animated loading border around given area
pub fn draw_loading_border(f: &mut Frame, area: Rect, theme: &Theme, tick: u64) {
    if area.width < 4 || area.height < 3 {
        return;
    }
    let w = area.width as usize;
    let h = area.height as usize;

    let mut path: Vec<(usize, usize)> = Vec::with_capacity(w * 2 + h * 2);
    for x in 0..w {
        path.push((x, 0));
    }
    for y in 1..h.saturating_sub(1) {
        path.push((w - 1, y));
    }
    for x in (0..w).rev() {
        path.push((x, h - 1));
    }
    for y in (1..h.saturating_sub(1)).rev() {
        path.push((0, y));
    }
    let perim = path.len();
    if perim == 0 {
        return;
    }
    let head = (tick as usize) % perim;

    let color_period: u64 = 30;
    let color_phase = ((tick / color_period) % 3) as u8;
    let sel_color = match color_phase {
        0 => theme.primary,
        1 => theme.accent,
        _ => theme.secondary,
    };

    let next_color = match color_phase {
        0 => theme.accent,
        1 => theme.secondary,
        _ => theme.primary,
    };

    let shades: [(char, bool); 4] = [('█', false), ('▓', false), ('▒', true), ('░', true)];
    let repeats_per_shade: usize = 4;
    let mut pattern: Vec<(char, bool)> = Vec::with_capacity(shades.len() * repeats_per_shade);
    for (ch, dim) in shades.iter() {
        for _ in 0..repeats_per_shade {
            pattern.push((*ch, *dim));
        }
    }
    let pattern_len = pattern.len().max(1);
    let stroke_len = std::cmp::min(pattern_len, perim);

    let phase_pos = tick % color_period;
    let transition_ticks: u64 = 10;
    let mut blend_cells: usize = 0;
    if phase_pos >= color_period.saturating_sub(transition_ticks) {
        let t = (phase_pos - (color_period - transition_ticks)) as f32 / transition_ticks as f32;
        let max_frac = 0.4f32;
        blend_cells = ((t * max_frac) * stroke_len as f32).round() as usize;
    }

    for i in 0..stroke_len {
        let idx = (head + perim - i) % perim;
        let (px, py) = path[idx];

        let (ch, dim) = if i == 0 {
            (spinner_head(tick), false)
        } else {
            pattern[i % pattern_len]
        };

        let col = if i < blend_cells {
            next_color
        } else {
            sel_color
        };

        let st = if i == 0 {
            let palette = [
                Color::Rgb(255, 200, 0),
                Color::Rgb(255, 64, 129),
                Color::Rgb(0, 255, 234),
            ];
            let c = palette[((tick / 3) as usize) % palette.len()];
            Style::default().fg(c).add_modifier(Modifier::BOLD)
        } else {
            let base = Style::default().fg(col);
            if dim {
                base.add_modifier(Modifier::DIM)
            } else {
                base.add_modifier(Modifier::BOLD)
            }
        };

        let cell = Rect {
            x: area.x.saturating_add(px as u16),
            y: area.y.saturating_add(py as u16),
            width: 1,
            height: 1,
        };
        let p = Paragraph::new(Line::from(Span::styled(ch.to_string(), st)))
            .style(Style::default().bg(theme.bg))
            .wrap(Wrap { trim: false });
        f.render_widget(p, cell);
    }

    let tail_start_i = stroke_len;
    if tail_start_i + 10 < perim {
        let idx0 = (head + perim - tail_start_i) % perim;
        let idx1 = (head + perim - (tail_start_i + 1)) % perim;
        let (x0, y0) = path[idx0];
        let (x1, y1) = path[idx1];
        let text_fwd = "loading...";
        let text_rev = "...gnidaol";
        let use_rev = if y1 == y0 { x1 < x0 } else { false };
        let chars: Vec<char> = if use_rev {
            text_rev.chars().collect()
        } else {
            text_fwd.chars().collect()
        };
        for (j, ch) in chars.into_iter().enumerate() {
            if tail_start_i + j >= perim {
                break;
            }
            let idx = (head + perim - (tail_start_i + j)) % perim;
            let (px, py) = path[idx];
            let cell = Rect {
                x: area.x.saturating_add(px as u16),
                y: area.y.saturating_add(py as u16),
                width: 1,
                height: 1,
            };
            let st = Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD);
            let p = Paragraph::new(Line::from(Span::styled(ch.to_string(), st)))
                .style(Style::default().bg(theme.bg))
                .wrap(Wrap { trim: false });
            f.render_widget(p, cell);
        }
    }
}

/// Draw horizontal color palette bars aligned with content area (5% margins on each side)
pub fn draw_color_bars(f: &mut Frame, screen: Rect, theme: &Theme) {
    // Fixed width for both bars
    const BAR_WIDTH: u16 = 10;

    // Calculate 5% margins (matching the main layout)
    let margin = screen.width / 20; // 5% = 1/20
    let content_width = screen.width.saturating_sub(margin * 2);

    // Only draw if content area is wide enough for both bars
    if content_width < BAR_WIDTH * 2 {
        return;
    }

    // Left corner bar - aligned with content area start
    let colors_left = [
        theme.primary,
        theme.accent,
        theme.secondary,
        theme.selected,
        theme.success,
    ];

    let mut left_spans = Vec::new();
    for i in 0..BAR_WIDTH {
        let color = colors_left[(i as usize) % colors_left.len()];
        left_spans.push(Span::styled("█", Style::default().fg(color)));
    }

    let left_bar = Rect {
        x: screen.x + margin, // Start at content area
        y: screen.y,
        width: BAR_WIDTH,
        height: 1,
    };

    let p_left = Paragraph::new(Line::from(left_spans)).style(Style::default().bg(theme.bg));
    f.render_widget(p_left, left_bar);

    // Right corner bar - aligned with content area end
    let colors_right = [
        theme.success,
        theme.selected,
        theme.secondary,
        theme.accent,
        theme.primary,
    ];

    let mut right_spans = Vec::new();
    for i in 0..BAR_WIDTH {
        let color = colors_right[(i as usize) % colors_right.len()];
        right_spans.push(Span::styled("█", Style::default().fg(color)));
    }

    let right_bar = Rect {
        x: screen.x + margin + content_width - BAR_WIDTH, // End of content area
        y: screen.y,
        width: BAR_WIDTH,
        height: 1,
    };

    let p_right = Paragraph::new(Line::from(right_spans)).style(Style::default().bg(theme.bg));
    f.render_widget(p_right, right_bar);
}

/// Draw a simple matrix-like falling glyphs background using a custom palette.
/// Intended for narrow side strips; subtle to not hurt readability.
pub fn draw_matrix_bg_custom(f: &mut Frame, area: Rect, palette: &[Color], tick: u64) {
    if area.width == 0 || area.height == 0 || palette.is_empty() {
        return;
    }
    let h = area.height as usize;
    let w = area.width as usize;
    let mut out: Vec<Line> = Vec::with_capacity(h);
    for y in 0..h {
        let mut spans: Vec<Span> = Vec::with_capacity(w);
        for x in 0..w {
            let offset = (x * 37 + 17) % h;
            let base = tick as usize / 3;
            let head = (offset + base) % h;
            let core_len = 2 + (x % 2);
            let mid_len = 6 + (x % 3);
            let long_len = 12 + (x % 4);
            let dist = if y <= head { head - y } else { head + (h - y) };
            let col = palette[x % palette.len()];
            let (ch, style) = if dist == 0 {
                ('█', Style::default().fg(col).add_modifier(Modifier::BOLD))
            } else if dist <= core_len {
                ('▓', Style::default().fg(col))
            } else if dist <= core_len + mid_len {
                ('▒', Style::default().fg(col).add_modifier(Modifier::DIM))
            } else if dist <= core_len + mid_len + long_len {
                ('░', Style::default().fg(col).add_modifier(Modifier::DIM))
            } else {
                (' ', Style::default())
            };
            spans.push(Span::styled(ch.to_string(), style));
        }
        out.push(Line::from(spans));
    }
    let p = Paragraph::new(out).wrap(Wrap { trim: false });
    f.render_widget(p, area);
}
