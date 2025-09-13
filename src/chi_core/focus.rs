use crate::ui::PanelPane;

#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
pub struct FocusState {
    pub panel_focus: PanelPane,
    pub nested_focus: PanelPane,
}

#[allow(dead_code)]
impl FocusState {
    pub fn new(panel_focus: PanelPane, nested_focus: PanelPane) -> Self {
        Self {
            panel_focus,
            nested_focus,
        }
    }
    pub fn toggle_panel(&mut self) {
        self.panel_focus = match self.panel_focus {
            PanelPane::A => PanelPane::B,
            PanelPane::B => PanelPane::A,
        };
    }
    #[allow(dead_code)]
    pub fn toggle_nested(&mut self) {
        self.nested_focus = match self.nested_focus {
            PanelPane::A => PanelPane::B,
            PanelPane::B => PanelPane::A,
        };
    }
}
