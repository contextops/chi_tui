pub mod banner;
pub mod chrome;
pub mod form;
pub mod form_widget;
pub mod header;
pub mod horizontal_menu;
pub mod json_viewer;
pub mod markdown;
pub mod menu;
pub mod panel;
pub mod result_viewer;
pub mod status_bar;
pub mod watchdog;

use crate::app::Effect;
use crossterm::event::KeyCode;
use ratatui::prelude::*;
use std::any::Any;

pub trait Widget {
    fn render(&mut self, f: &mut Frame, area: Rect, focused: bool, tick: u64);
    fn on_key(&mut self, key: KeyCode) -> Vec<Effect> {
        let _ = key;
        Vec::new()
    }
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
