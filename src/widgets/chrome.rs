use crate::theme::Theme;
use ratatui::widgets::{Block, Borders};

pub fn panel_block<'a>(title: &'a str, focused: bool) -> Block<'a> {
    let mut b = Block::default().borders(Borders::ALL).title(title);
    if focused {
        b = b.border_style(crate::theme::border_focused());
    }
    b
}

#[allow(dead_code)]
pub fn panel_block_themed<'a>(title: &'a str, focused: bool, theme: &Theme) -> Block<'a> {
    let mut b = Block::default().borders(Borders::ALL).title(title);
    if focused {
        b = b.border_style(theme.border_focused());
    } else {
        b = b.border_style(theme.border_unfocused());
    }
    b
}
