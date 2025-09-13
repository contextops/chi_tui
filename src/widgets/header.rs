use ratatui::prelude::*;

use crate::ui::AppState;

pub fn draw_header(f: &mut Frame, area: Rect, state: &AppState) {
    // Draw top banner with subtle animation; title text remains in ASCII art.
    crate::widgets::banner::draw_banner(f, area, state);
}
