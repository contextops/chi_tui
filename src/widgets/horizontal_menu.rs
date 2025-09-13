use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Tabs};

use crate::ui::AppState;

pub fn draw_horizontal_menu(f: &mut Frame, area: Rect, state: &AppState) {
    let theme = &state.theme;
    let current_index = state.horizontal_tab_index;

    // Build tab titles with function key indicators
    let mut titles: Vec<Line> = Vec::new();

    // If no menu configured, show default [F1] Main
    if state.config.horizontal_menu.is_empty() {
        let is_selected = current_index == 0;
        let text_style = if is_selected {
            Style::default()
                .fg(theme.selected)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.muted)
        };

        let key_style = if is_selected {
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.muted)
        };

        let line = Line::from(vec![
            Span::styled("[", Style::default().fg(theme.frame)),
            Span::styled("F1", key_style),
            Span::styled("]", Style::default().fg(theme.frame)),
            Span::raw(" "),
            Span::styled("Main", text_style),
        ]);

        titles.push(line);
    } else {
        for (i, item) in state.config.horizontal_menu.iter().enumerate() {
            let is_selected = i == current_index;
            let fn_key = format!("F{}", i + 1);

            // Style for the tab
            let text_style = if is_selected {
                Style::default()
                    .fg(theme.selected)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.muted)
            };

            let key_style = if is_selected {
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.muted)
            };

            // Build the tab line: [F1] Title
            let line = Line::from(vec![
                Span::styled("[", Style::default().fg(theme.frame)),
                Span::styled(fn_key, key_style),
                Span::styled("]", Style::default().fg(theme.frame)),
                Span::raw(" "),
                Span::styled(&item.title, text_style),
            ]);

            titles.push(line);
        }
    }

    // Create the tabs widget
    let tabs = Tabs::new(titles)
        .select(current_index)
        .style(Style::default().fg(theme.fg))
        .highlight_style(
            Style::default()
                .fg(theme.selected)
                .add_modifier(Modifier::BOLD),
        )
        .divider(Span::styled(" │ ", Style::default().fg(theme.frame)));

    // Add a border at the bottom
    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(theme.frame));

    f.render_widget(tabs.block(block), area);
}

/// Handle function key presses for horizontal menu
/// Returns Some(config_path) if a new config should be loaded
pub fn handle_function_key(state: &mut AppState, key_num: u8) -> Option<String> {
    // F1 = 1, F2 = 2, etc.
    let index = (key_num - 1) as usize;

    // Handle default [F1] Main when no menu configured
    if state.config.horizontal_menu.is_empty() {
        if key_num == 1 {
            state.horizontal_tab_index = 0;
        }
        return None;
    }

    if index < state.config.horizontal_menu.len() {
        // Don't reload if we're already on this tab
        if state.horizontal_tab_index == index {
            return None;
        }

        state.horizontal_tab_index = index;

        // Return the config path to load (if specified)
        state.config.horizontal_menu[index].config.clone()
    } else {
        None
    }
}
