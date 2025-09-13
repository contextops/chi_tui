use crossterm::event::KeyCode;
use ratatui::prelude::*;

pub struct PanelWidget {
    pub layout: crate::ui::PanelLayout,
    pub ratio: crate::ui::PanelRatio,
    pub a: crate::ui::PaneData,
    pub b: crate::ui::PaneData,
    nested_focus: crate::ui::PanelPane,
    title_a: String,
    title_b: String,
    a_w: Option<Box<dyn crate::widgets::Widget>>,
    b_w: Option<Box<dyn crate::widgets::Widget>>,
}

impl PanelWidget {
    pub fn from_panel_state(st: crate::ui::PanelState) -> Self {
        Self::from_panel_state_with_titles(st, "Pane B.A", "Pane B.B")
    }

    pub fn from_panel_state_with_titles(
        st: crate::ui::PanelState,
        title_a: impl Into<String>,
        title_b: impl Into<String>,
    ) -> Self {
        // Attempt to seed pretty viewers based on existing JSON strings
        let title_a_str = title_a.into();
        let title_b_str = title_b.into();
        let a_w =
            st.a.last_json_pretty
                .as_ref()
                .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
                .map(|v| {
                    Box::new(crate::widgets::result_viewer::ResultViewerWidget::new(
                        title_a_str.clone(),
                        v,
                    )) as Box<dyn crate::widgets::Widget>
                });
        let b_w =
            st.b.last_json_pretty
                .as_ref()
                .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
                .map(|v| {
                    Box::new(crate::widgets::result_viewer::ResultViewerWidget::new(
                        title_b_str.clone(),
                        v,
                    )) as Box<dyn crate::widgets::Widget>
                });

        Self {
            layout: st.layout,
            ratio: st.ratio,
            a: st.a,
            b: st.b,
            nested_focus: crate::ui::PanelPane::A,
            title_a: title_a_str,
            title_b: title_b_str,
            a_w,
            b_w,
        }
    }
    pub fn nested_focus(&self) -> crate::ui::PanelPane {
        self.nested_focus
    }
    pub fn set_nested_focus(&mut self, f: crate::ui::PanelPane) {
        self.nested_focus = f;
    }
    fn constraints(&self) -> [Constraint; 2] {
        match self.ratio {
            crate::ui::PanelRatio::Half => [Constraint::Percentage(50), Constraint::Percentage(50)],
            crate::ui::PanelRatio::OneToThree => {
                [Constraint::Percentage(25), Constraint::Percentage(75)]
            }
            crate::ui::PanelRatio::ThreeToOne => {
                [Constraint::Percentage(75), Constraint::Percentage(25)]
            }
            crate::ui::PanelRatio::OneToTwo => {
                [Constraint::Percentage(33), Constraint::Percentage(67)]
            }
            crate::ui::PanelRatio::TwoToOne => {
                [Constraint::Percentage(67), Constraint::Percentage(33)]
            }
            crate::ui::PanelRatio::TwoToThree => {
                [Constraint::Percentage(40), Constraint::Percentage(60)]
            }
            crate::ui::PanelRatio::ThreeToTwo => {
                [Constraint::Percentage(60), Constraint::Percentage(40)]
            }
        }
    }
    pub fn set_subpane_text(&mut self, sub: crate::ui::PanelPane, text: String) {
        match sub {
            crate::ui::PanelPane::A => {
                self.a.last_error = None;
                self.a.last_json_pretty = Some(text.clone());
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                    self.a_w = Some(Box::new(
                        crate::widgets::result_viewer::ResultViewerWidget::new(
                            self.title_a.clone(),
                            v,
                        ),
                    ));
                }
            }
            crate::ui::PanelPane::B => {
                self.b.last_error = None;
                self.b.last_json_pretty = Some(text.clone());
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                    self.b_w = Some(Box::new(
                        crate::widgets::result_viewer::ResultViewerWidget::new(
                            self.title_b.clone(),
                            v,
                        ),
                    ));
                }
            }
        }
    }
    pub fn set_subpane_error(&mut self, sub: crate::ui::PanelPane, err: String) {
        match sub {
            crate::ui::PanelPane::A => {
                self.a.last_error = Some(err);
                self.a.last_json_pretty = None;
                self.a_w = None;
            }
            crate::ui::PanelPane::B => {
                self.b.last_error = Some(err);
                self.b.last_json_pretty = None;
                self.b_w = None;
            }
        }
    }
}

impl crate::widgets::Widget for PanelWidget {
    fn render(&mut self, f: &mut Frame, area: Rect, _focused: bool, _tick: u64) {
        let chunks = if matches!(self.layout, crate::ui::PanelLayout::Horizontal) {
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints(self.constraints())
                .split(area)
        } else {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints(self.constraints())
                .split(area)
        };
        // Pane A
        if self.a.last_error.is_some() || self.a_w.is_none() {
            let mut lines_a: Vec<Line> = Vec::new();
            if let Some(err) = &self.a.last_error {
                lines_a.push(Line::from(err.clone()).style(Style::default().fg(Color::Red)));
                lines_a.push(Line::from(""));
            }
            if let Some(txt) = &self.a.last_json_pretty {
                for l in txt.lines() {
                    lines_a.push(Line::from(l.to_string()));
                }
            }
            let block_a = crate::widgets::chrome::panel_block(
                &self.title_a,
                matches!(self.nested_focus, crate::ui::PanelPane::A),
            );
            let pa = ratatui::widgets::Paragraph::new(lines_a).block(block_a);
            f.render_widget(pa, chunks[0]);
        } else if let Some(w) = &mut self.a_w {
            let focused = matches!(self.nested_focus, crate::ui::PanelPane::A);
            w.render(f, chunks[0], focused, 0);
        }
        // Pane B
        if self.b.last_error.is_some() || self.b_w.is_none() {
            let mut lines_b: Vec<Line> = Vec::new();
            if let Some(err) = &self.b.last_error {
                lines_b.push(Line::from(err.clone()).style(Style::default().fg(Color::Red)));
                lines_b.push(Line::from(""));
            }
            if let Some(txt) = &self.b.last_json_pretty {
                for l in txt.lines() {
                    lines_b.push(Line::from(l.to_string()));
                }
            }
            let block_b = crate::widgets::chrome::panel_block(
                &self.title_b,
                matches!(self.nested_focus, crate::ui::PanelPane::B),
            );
            let pb = ratatui::widgets::Paragraph::new(lines_b).block(block_b);
            f.render_widget(pb, chunks[1]);
        } else if let Some(w) = &mut self.b_w {
            let focused = matches!(self.nested_focus, crate::ui::PanelPane::B);
            w.render(f, chunks[1], focused, 0);
        }
    }
    fn on_key(&mut self, key: KeyCode) -> Vec<crate::app::Effect> {
        let mut effects: Vec<crate::app::Effect> = Vec::new();
        match key {
            KeyCode::Tab | KeyCode::BackTab => {
                self.nested_focus = if matches!(self.nested_focus, crate::ui::PanelPane::A) {
                    crate::ui::PanelPane::B
                } else {
                    crate::ui::PanelPane::A
                };
            }
            other => {
                // Forward common keys to the focused pretty viewer (if any)
                match self.nested_focus {
                    crate::ui::PanelPane::A => {
                        if let Some(w) = &mut self.a_w {
                            let effs = w.on_key(other);
                            effects.extend(effs);
                        }
                    }
                    crate::ui::PanelPane::B => {
                        if let Some(w) = &mut self.b_w {
                            let effs = w.on_key(other);
                            effects.extend(effs);
                        }
                    }
                }
            }
        }
        effects
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl PanelWidget {
    pub fn set_subpane_widget(
        &mut self,
        sub: crate::ui::PanelPane,
        widget: Box<dyn crate::widgets::Widget>,
    ) {
        match sub {
            crate::ui::PanelPane::A => {
                self.a.last_error = None;
                self.a.last_json_pretty = None;
                self.a_w = Some(widget);
            }
            crate::ui::PanelPane::B => {
                self.b.last_error = None;
                self.b.last_json_pretty = None;
                self.b_w = Some(widget);
            }
        }
    }
}
