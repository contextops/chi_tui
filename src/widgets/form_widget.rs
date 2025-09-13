use crate::widgets::form::{draw_form, FieldKind, FieldValue, FormState, OPTIONS_VISIBLE};
use crossterm::event::KeyCode;
use ratatui::crossterm::event as rt_event;
use ratatui::prelude::*;
use std::time::Duration;
use tui_textarea::TextArea;

// Parameters describing numeric field constraints
struct NumParams {
    is_integer: bool,
    minimum: Option<f64>,
    maximum: Option<f64>,
    exclusive_minimum: bool,
    exclusive_maximum: bool,
    multiple_of: Option<f64>,
}

pub struct FormWidget {
    pub form: FormState,
    ta_map: std::collections::HashMap<String, TextArea<'static>>,
}

impl FormWidget {
    pub fn new(mut form: FormState) -> Self {
        crate::widgets::form::capture_initial(&mut form);
        // Prepare TextArea state for textarea fields
        let mut ta_map: std::collections::HashMap<String, TextArea<'static>> =
            std::collections::HashMap::new();
        for f in &form.fields {
            if let FieldKind::TextArea { .. } = f.kind {
                let mut ta = TextArea::default();
                if let FieldValue::Text(txt) = &f.value {
                    if !txt.is_empty() {
                        ta.insert_str(txt);
                    }
                }
                ta.set_block(
                    ratatui::widgets::Block::default()
                        .borders(ratatui::widgets::Borders::ALL)
                        .title(format!("Editing: {} — Ctrl+S Save • Esc Cancel", f.label)),
                );
                ta_map.insert(f.name.clone(), ta);
            }
        }
        Self { form, ta_map }
    }
    fn options_ttl() -> Option<Duration> {
        match std::env::var("CHI_TUI_OPTIONS_TTL_SEC")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
        {
            Some(0) => None,
            Some(secs) => Some(Duration::from_secs(secs)),
            None => Some(Duration::from_secs(30)),
        }
    }
    fn should_fetch_options(fld: &crate::widgets::form::FormField) -> bool {
        if fld.dyn_options_cmd.is_none() {
            return false;
        }
        if !fld.dyn_loaded {
            return true;
        }
        if let Some(ttl) = Self::options_ttl() {
            if let Some(ts) = fld.dyn_loaded_at {
                return ts.elapsed() > ttl;
            }
        }
        false
    }

    pub fn commit_textarea(&mut self) -> bool {
        if !self.form.editing {
            return false;
        }
        let sel = self
            .form
            .selected
            .min(self.form.fields.len().saturating_sub(1));
        if let Some(fld) = self.form.fields.get_mut(sel) {
            if let FieldKind::TextArea { .. } = fld.kind {
                if let Some(ta) = self.ta_map.get_mut(&fld.name) {
                    let text = ta.lines().join("\n");
                    fld.value = FieldValue::Text(text);
                    crate::widgets::form::compute_dirty(&mut self.form);
                    self.form.editing = false;
                    self.form.message = None;
                    return true;
                }
            }
        }
        false
    }

    #[allow(dead_code)]
    pub fn cancel_textarea(&mut self) -> bool {
        if !self.form.editing {
            return false;
        }
        let sel = self
            .form
            .selected
            .min(self.form.fields.len().saturating_sub(1));
        if let Some(fld) = self.form.fields.get(sel) {
            if let FieldKind::TextArea { .. } = fld.kind {
                self.form.editing = false;
                self.form.message = None;
                return true;
            }
        }
        false
    }
}

impl crate::widgets::Widget for FormWidget {
    fn render(&mut self, f: &mut Frame, area: Rect, focused: bool, tick: u64) {
        let mut cursor_on = tick % 2 == 0;
        // Suppress underlying blinking cursor when textarea modal is active to avoid layout jitter
        if self.form.editing {
            let sel = self
                .form
                .selected
                .min(self.form.fields.len().saturating_sub(1));
            if let Some(fld) = self.form.fields.get(sel) {
                if let FieldKind::TextArea { .. } = fld.kind {
                    cursor_on = false;
                }
            }
        }
        draw_form(f, area, &mut self.form, focused, cursor_on);
        // Overlay textarea editor when editing a textarea field
        if self.form.editing {
            let sel = self
                .form
                .selected
                .min(self.form.fields.len().saturating_sub(1));
            if let Some(fld) = self.form.fields.get(sel) {
                if let FieldKind::TextArea { .. } = fld.kind {
                    if let Some(ta) = self.ta_map.get_mut(&fld.name) {
                        ta.set_block(
                            ratatui::widgets::Block::default()
                                .borders(ratatui::widgets::Borders::ALL)
                                .title(format!(
                                    "Editing: {} — Ctrl+S Save • Esc Cancel",
                                    fld.label
                                )),
                        );
                        let rect = centered_rect(80, 70, area);
                        f.render_widget(ratatui::widgets::Clear, rect);
                        f.render_widget(&*ta, rect);
                    }
                }
            }
        }
    }
    fn on_key(&mut self, key: KeyCode) -> Vec<crate::app::Effect> {
        use crate::app::Effect;
        let mut effects: Vec<Effect> = Vec::new();
        match key {
            KeyCode::Up => {
                // When editing a textarea, route to TextArea state
                if self.form.editing {
                    if let Some(fld) = self.form.fields.get(self.form.selected) {
                        if let FieldKind::TextArea { .. } = fld.kind {
                            if let Some(ta) = self.ta_map.get_mut(&fld.name) {
                                let _ = ta.input(rt_event::KeyEvent::new(
                                    rt_event::KeyCode::Up,
                                    rt_event::KeyModifiers::NONE,
                                ));
                                return effects;
                            }
                        }
                    }
                }
                if self.form.editing {
                    let sel = self.form.selected;
                    if let Some(fld) = self.form.fields.get_mut(sel) {
                        let mut num_params: Option<NumParams> = None;
                        if let FieldKind::Number {
                            is_integer,
                            minimum,
                            maximum,
                            exclusive_minimum,
                            exclusive_maximum,
                            multiple_of,
                        } = &fld.kind
                        {
                            num_params = Some(NumParams {
                                is_integer: *is_integer,
                                minimum: *minimum,
                                maximum: *maximum,
                                exclusive_minimum: *exclusive_minimum,
                                exclusive_maximum: *exclusive_maximum,
                                multiple_of: *multiple_of,
                            });
                        }
                        match &mut fld.kind {
                            FieldKind::Number { .. } => {
                                if let Some(params) = num_params.as_ref() {
                                    step_number_value(fld, 1, params);
                                    crate::widgets::form::compute_dirty(&mut self.form);
                                }
                            }
                            FieldKind::Select { cursor, offset, .. }
                            | FieldKind::MultiSelect { cursor, offset, .. } => {
                                if *cursor > 0 {
                                    *cursor -= 1;
                                }
                                if *cursor < *offset {
                                    *offset = *cursor;
                                }
                            }
                            FieldKind::TextArea { edit_lines, offset } => {
                                let total = if let FieldValue::Text(s) = &fld.value {
                                    s.lines().count()
                                } else {
                                    0
                                };
                                if *offset > 0 {
                                    *offset -= 1;
                                }
                                if *offset + *edit_lines > total {
                                    *offset = total.saturating_sub(*edit_lines);
                                }
                            }
                            _ => {}
                        }
                    }
                } else if self.form.selected > 0 {
                    self.form.selected -= 1;
                    let sel = self.form.selected;
                    if let Some(fld) = self.form.fields.get(sel) {
                        if Self::should_fetch_options(fld) {
                            if let Some(cmd) = &fld.dyn_options_cmd {
                                effects.push(Effect::LoadFormOptions {
                                    field: fld.name.clone(),
                                    cmdline: cmd.clone(),
                                    unwrap: fld.dyn_unwrap.clone(),
                                    force: false,
                                });
                            }
                        }
                    }
                }
                effects
            }
            KeyCode::Down => {
                if self.form.editing {
                    if let Some(fld) = self.form.fields.get(self.form.selected) {
                        if let FieldKind::TextArea { .. } = fld.kind {
                            if let Some(ta) = self.ta_map.get_mut(&fld.name) {
                                let _ = ta.input(rt_event::KeyEvent::new(
                                    rt_event::KeyCode::Down,
                                    rt_event::KeyModifiers::NONE,
                                ));
                                return effects;
                            }
                        }
                    }
                }
                if self.form.editing {
                    let sel = self.form.selected;
                    if let Some(fld) = self.form.fields.get_mut(sel) {
                        let mut num_params: Option<NumParams> = None;
                        if let FieldKind::Number {
                            is_integer,
                            minimum,
                            maximum,
                            exclusive_minimum,
                            exclusive_maximum,
                            multiple_of,
                        } = &fld.kind
                        {
                            num_params = Some(NumParams {
                                is_integer: *is_integer,
                                minimum: *minimum,
                                maximum: *maximum,
                                exclusive_minimum: *exclusive_minimum,
                                exclusive_maximum: *exclusive_maximum,
                                multiple_of: *multiple_of,
                            });
                        }
                        match &mut fld.kind {
                            FieldKind::Number { .. } => {
                                if let Some(params) = num_params.as_ref() {
                                    step_number_value(fld, -1, params);
                                    crate::widgets::form::compute_dirty(&mut self.form);
                                }
                            }
                            FieldKind::Select {
                                cursor,
                                options,
                                offset,
                                ..
                            } => {
                                if *cursor + 1 < options.len() {
                                    *cursor += 1;
                                }
                                if *cursor >= *offset + OPTIONS_VISIBLE {
                                    *offset = *cursor + 1 - OPTIONS_VISIBLE;
                                }
                            }
                            FieldKind::MultiSelect {
                                cursor,
                                options,
                                offset,
                                ..
                            } => {
                                if *cursor + 1 < options.len() {
                                    *cursor += 1;
                                }
                                if *cursor >= *offset + OPTIONS_VISIBLE {
                                    *offset = *cursor + 1 - OPTIONS_VISIBLE;
                                }
                            }
                            FieldKind::TextArea { edit_lines, offset } => {
                                let total = if let FieldValue::Text(s) = &fld.value {
                                    s.lines().count()
                                } else {
                                    0
                                };
                                if *offset + *edit_lines < total {
                                    *offset += 1;
                                }
                            }
                            _ => {}
                        }
                    }
                } else {
                    let max_idx = self.form.fields.len() + 2; // include Reset and Cancel
                    if self.form.selected < max_idx {
                        self.form.selected += 1;
                    }
                    let sel = self
                        .form
                        .selected
                        .min(self.form.fields.len().saturating_sub(1));
                    if let Some(fld) = self.form.fields.get(sel) {
                        if Self::should_fetch_options(fld) {
                            if let Some(cmd) = &fld.dyn_options_cmd {
                                effects.push(Effect::LoadFormOptions {
                                    field: fld.name.clone(),
                                    cmdline: cmd.clone(),
                                    unwrap: fld.dyn_unwrap.clone(),
                                    force: false,
                                });
                            }
                        }
                    }
                }
                effects
            }
            KeyCode::Left => {
                if self.form.editing {
                    if let Some(fld) = self.form.fields.get(self.form.selected) {
                        if let FieldKind::TextArea { .. } = fld.kind {
                            if let Some(ta) = self.ta_map.get_mut(&fld.name) {
                                let _ = ta.input(rt_event::KeyEvent::new(
                                    rt_event::KeyCode::Left,
                                    rt_event::KeyModifiers::NONE,
                                ));
                                return effects;
                            }
                        }
                    }
                }
                if self.form.editing {
                    let sel = self.form.selected;
                    if let Some(fld) = self.form.fields.get_mut(sel) {
                        if let FieldKind::Select {
                            cursor, selected, ..
                        } = &mut fld.kind
                        {
                            *selected = *cursor;
                            self.form.editing = false;
                            crate::widgets::form::compute_dirty(&mut self.form);
                        }
                    }
                } else {
                    // quick change for Select when browsing
                    let sel = self.form.selected;
                    if sel < self.form.fields.len() {
                        if let Some(fld) = self.form.fields.get_mut(sel) {
                            if let FieldKind::Select {
                                options, selected, ..
                            } = &mut fld.kind
                            {
                                if !options.is_empty() {
                                    if *selected == 0 {
                                        *selected = options.len().saturating_sub(1);
                                    } else {
                                        *selected -= 1;
                                    }
                                    crate::widgets::form::compute_dirty(&mut self.form);
                                }
                            }
                        }
                    } else {
                        // move between buttons: Cancel -> Reset -> Save
                        let save_idx = self.form.fields.len();
                        let reset_idx = self.form.fields.len() + 1;
                        let cancel_idx = self.form.fields.len() + 2;
                        if self.form.selected == cancel_idx {
                            self.form.selected = reset_idx;
                        } else if self.form.selected == reset_idx {
                            self.form.selected = save_idx;
                        }
                    }
                }
                effects
            }
            KeyCode::Right => {
                if self.form.editing {
                    if let Some(fld) = self.form.fields.get(self.form.selected) {
                        if let FieldKind::TextArea { .. } = fld.kind {
                            if let Some(ta) = self.ta_map.get_mut(&fld.name) {
                                let _ = ta.input(rt_event::KeyEvent::new(
                                    rt_event::KeyCode::Right,
                                    rt_event::KeyModifiers::NONE,
                                ));
                                return effects;
                            }
                        }
                    }
                }
                if self.form.editing {
                    let sel = self.form.selected;
                    if let Some(fld) = self.form.fields.get_mut(sel) {
                        if let FieldKind::Select {
                            cursor, selected, ..
                        } = &mut fld.kind
                        {
                            *selected = *cursor;
                            self.form.editing = false;
                        }
                    }
                } else {
                    // quick change for Select when browsing
                    let sel = self.form.selected;
                    if sel < self.form.fields.len() {
                        if let Some(fld) = self.form.fields.get_mut(sel) {
                            if let FieldKind::Select {
                                options, selected, ..
                            } = &mut fld.kind
                            {
                                if !options.is_empty() {
                                    if *selected + 1 < options.len() {
                                        *selected += 1;
                                    } else {
                                        *selected = 0;
                                    }
                                    crate::widgets::form::compute_dirty(&mut self.form);
                                }
                            }
                        }
                    } else {
                        // move between buttons: Save -> Reset -> Cancel
                        let save_idx = self.form.fields.len();
                        let reset_idx = self.form.fields.len() + 1;
                        let cancel_idx = self.form.fields.len() + 2;
                        if self.form.selected == save_idx {
                            self.form.selected = reset_idx;
                        } else if self.form.selected == reset_idx {
                            self.form.selected = cancel_idx;
                        }
                    }
                }
                effects
            }
            KeyCode::Enter => {
                if self.form.editing {
                    if let Some(fld) = self.form.fields.get(self.form.selected) {
                        if let FieldKind::TextArea { .. } = fld.kind {
                            if let Some(ta) = self.ta_map.get_mut(&fld.name) {
                                let _ = ta.input(rt_event::KeyEvent::new(
                                    rt_event::KeyCode::Enter,
                                    rt_event::KeyModifiers::NONE,
                                ));
                                return effects;
                            }
                        }
                    }
                }
                let save_idx = self.form.fields.len();
                let reset_idx = self.form.fields.len() + 1;
                let cancel_idx = self.form.fields.len() + 2;
                if !self.form.editing && self.form.selected == save_idx {
                    if crate::widgets::form::validate_form(&mut self.form) {
                        if let Some(cmdline) = crate::widgets::form::build_cmdline(&self.form) {
                            effects.push(Effect::SubmitForm {
                                pane: crate::ui::PanelPane::B,
                                cmdline,
                            });
                        }
                    }
                } else if !self.form.editing && self.form.selected == reset_idx {
                    if self.form.dirty {
                        // two-step confirm
                        if self.form.confirm == Some(crate::widgets::form::ConfirmAction::Reset) {
                            crate::widgets::form::reset_to_initial(&mut self.form);
                            effects.push(Effect::ShowToast {
                                text: "Reset".into(),
                                level: crate::ui::ToastLevel::Info,
                                seconds: 2,
                            });
                            self.form.confirm = None;
                        } else {
                            self.form.confirm = Some(crate::widgets::form::ConfirmAction::Reset);
                            self.form.message =
                                Some("Press Enter to confirm Reset • Esc to cancel".into());
                        }
                    }
                } else if !self.form.editing && self.form.selected == cancel_idx {
                    // two-step confirm: cancel closes the form
                    if self.form.confirm == Some(crate::widgets::form::ConfirmAction::Cancel) {
                        effects.push(Effect::CancelForm {
                            pane: crate::ui::PanelPane::B,
                        });
                        self.form.confirm = None;
                    } else {
                        self.form.confirm = Some(crate::widgets::form::ConfirmAction::Cancel);
                        self.form.message =
                            Some("Press Enter to confirm Cancel • Esc to stay".into());
                    }
                } else {
                    let sel = self.form.selected;
                    if let Some(fld) = self.form.fields.get_mut(sel) {
                        match (&mut fld.kind, &mut fld.value) {
                            (FieldKind::Checkbox, FieldValue::Bool(b)) => {
                                *b = !*b;
                            }
                            (FieldKind::Text, FieldValue::Text(_))
                            | (FieldKind::Password, FieldValue::Text(_))
                            | (FieldKind::Number { .. }, FieldValue::Text(_)) => {
                                self.form.editing = !self.form.editing;
                            }
                            // Array UX: Enter in array field
                            // - if not editing: enter edit mode
                            // - if editing: commit current token as a chip by appending ", " when needed
                            (FieldKind::Array { .. }, FieldValue::Text(s)) => {
                                if self.form.editing {
                                    let trimmed = s.trim_end();
                                    if !trimmed.is_empty() && !trimmed.ends_with(',') {
                                        s.push(',');
                                        s.push(' ');
                                    }
                                } else {
                                    self.form.editing = true;
                                }
                            }
                            (FieldKind::TextArea { .. }, FieldValue::Text(_)) => {
                                // Enter editing, initialize TextArea content
                                if !self.form.editing {
                                    self.form.editing = true;
                                    let name = fld.name.clone();
                                    if let Some(ta) = self.ta_map.get_mut(&name) {
                                        *ta = TextArea::default();
                                        if let FieldValue::Text(txt) = &fld.value {
                                            if !txt.is_empty() {
                                                ta.insert_str(txt);
                                            }
                                        }
                                        ta.set_block(
                                            ratatui::widgets::Block::default()
                                                .borders(ratatui::widgets::Borders::ALL)
                                                .title(format!(
                                                    "Editing: {} — Ctrl+S Save • Esc Cancel",
                                                    fld.label
                                                )),
                                        );
                                    }
                                }
                            }
                            (
                                FieldKind::Select {
                                    cursor, selected, ..
                                },
                                _,
                            ) => {
                                if self.form.editing {
                                    *selected = *cursor;
                                    self.form.editing = false;
                                    crate::widgets::form::compute_dirty(&mut self.form);
                                } else {
                                    *cursor = *selected;
                                    self.form.editing = true;
                                }
                            }
                            (
                                FieldKind::MultiSelect {
                                    cursor, selected, ..
                                },
                                _,
                            ) => {
                                if self.form.editing {
                                    if let Some(slot) = selected.get_mut(*cursor) {
                                        *slot = !*slot;
                                        crate::widgets::form::compute_dirty(&mut self.form);
                                    }
                                } else {
                                    // Enter editing mode on first Enter; do not toggle yet
                                    self.form.editing = true;
                                }
                            }
                            _ => {}
                        }
                        if let Some(f) = self.form.fields.get(sel) {
                            if Self::should_fetch_options(f) {
                                if let Some(cmd) = &f.dyn_options_cmd {
                                    effects.push(Effect::LoadFormOptions {
                                        field: f.name.clone(),
                                        cmdline: cmd.clone(),
                                        unwrap: f.dyn_unwrap.clone(),
                                        force: false,
                                    });
                                }
                            }
                        }
                    }
                }
                effects
            }
            KeyCode::Backspace => {
                if self.form.editing {
                    if let Some(fld) = self.form.fields.get(self.form.selected) {
                        if let FieldKind::TextArea { .. } = fld.kind {
                            if let Some(ta) = self.ta_map.get_mut(&fld.name) {
                                let _ = ta.input(rt_event::KeyEvent::new(
                                    rt_event::KeyCode::Backspace,
                                    rt_event::KeyModifiers::NONE,
                                ));
                                return effects;
                            }
                        }
                    }
                }
                if self.form.editing {
                    let sel = self.form.selected;
                    if let Some(fld) = self.form.fields.get_mut(sel) {
                        match (&mut fld.kind, &mut fld.value) {
                            // Array UX: when token is empty, Backspace removes the last chip
                            (FieldKind::Array { .. }, FieldValue::Text(s)) => {
                                // Remove trailing cursor helper if present by design (handled in rendering), then operate on content
                                // Trim right spaces for decision
                                let mut had_trailing_ws = false;
                                while s.ends_with(' ') {
                                    s.pop();
                                    had_trailing_ws = true;
                                }
                                if s.is_empty() {
                                    // nothing to do
                                } else if s.ends_with(',') || had_trailing_ws {
                                    // We are at an empty token (", " just typed). Remove the entire previous chip.
                                    // First remove the trailing comma (and any spaces before it)
                                    while s.ends_with(',') || s.ends_with(' ') {
                                        s.pop();
                                    }
                                    if let Some(pos) = s.rfind(',') {
                                        s.truncate(pos);
                                    } else {
                                        s.clear();
                                    }
                                } else {
                                    // Default: delete one character
                                    s.pop();
                                }
                            }
                            // Default single-character delete for other text-like fields
                            (_, FieldValue::Text(s)) => {
                                if !s.is_empty() {
                                    s.pop();
                                }
                            }
                            _ => {}
                        }
                        // inline validate after editing
                        crate::widgets::form::validate_text_inline(fld);
                        crate::widgets::form::compute_dirty(&mut self.form);
                    }
                }
                effects
            }
            KeyCode::Esc => {
                // Close confirmation or cancel textarea editing without saving
                if self.form.confirm.is_some() {
                    self.form.confirm = None;
                    self.form.message = None;
                } else if self.form.editing {
                    self.form.editing = false;
                    self.form.message = None;
                }
                effects
            }
            KeyCode::Char(c) => {
                if self.form.editing {
                    if let Some(fld) = self.form.fields.get(self.form.selected) {
                        if let FieldKind::TextArea { .. } = fld.kind {
                            if let Some(ta) = self.ta_map.get_mut(&fld.name) {
                                let _ = ta.input(rt_event::KeyEvent::new(
                                    rt_event::KeyCode::Char(c),
                                    rt_event::KeyModifiers::NONE,
                                ));
                                return effects;
                            }
                        }
                    }
                }
                // Special-case: when not editing and pressing 'r'/'R', treat as options refresh
                if !self.form.editing && (c == 'r' || c == 'R') {
                    let sel = self
                        .form
                        .selected
                        .min(self.form.fields.len().saturating_sub(1));
                    if let Some(fld) = self.form.fields.get(sel) {
                        if let Some(cmd) = &fld.dyn_options_cmd {
                            effects.push(Effect::LoadFormOptions {
                                field: fld.name.clone(),
                                cmdline: cmd.clone(),
                                unwrap: fld.dyn_unwrap.clone(),
                                force: true,
                            });
                            return effects;
                        }
                    }
                    return effects;
                }
                if self.form.editing {
                    let sel = self.form.selected;
                    if let Some(fld) = self.form.fields.get_mut(sel) {
                        match (&mut fld.kind, &mut fld.value) {
                            (FieldKind::Text, FieldValue::Text(s))
                            | (FieldKind::Password, FieldValue::Text(s))
                            | (FieldKind::TextArea { .. }, FieldValue::Text(s)) => {
                                s.push(c);
                                crate::widgets::form::validate_text_inline(fld);
                            }
                            (FieldKind::Number { is_integer, .. }, FieldValue::Text(s)) => {
                                if c.is_ascii_digit()
                                    || (c == '.' && !*is_integer && !s.contains('.'))
                                    || (c == '-' && s.is_empty())
                                {
                                    s.push(c);
                                }
                                // Validate number-like input inline too (shared validator handles bounds/type later)
                                crate::widgets::form::validate_text_inline(fld);
                            }
                            (
                                FieldKind::MultiSelect {
                                    cursor, selected, ..
                                },
                                _,
                            ) if c == ' ' => {
                                if let Some(slot) = selected.get_mut(*cursor) {
                                    *slot = !*slot;
                                }
                            }
                            _ => {}
                        }
                        crate::widgets::form::compute_dirty(&mut self.form);
                    }
                } else {
                    // Not editing: support quick toggle for MultiSelect with Space
                    if c == ' ' {
                        let sel = self.form.selected;
                        if let Some(fld) = self.form.fields.get_mut(sel) {
                            if let FieldKind::MultiSelect {
                                cursor, selected, ..
                            } = &mut fld.kind
                            {
                                if let Some(slot) = selected.get_mut(*cursor) {
                                    *slot = !*slot;
                                    // Enter editing so options list becomes visible
                                    self.form.editing = true;
                                    crate::widgets::form::compute_dirty(&mut self.form);
                                }
                            }
                        }
                    }
                }
                effects
            }
            _ => effects,
        }
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let v = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(area);
    let h = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(v[1]);
    h[1]
}

fn step_number_value(fld: &mut crate::widgets::form::FormField, dir: i32, params: &NumParams) {
    // Parse current value
    let mut cur = if let crate::widgets::form::FieldValue::Text(s) = &fld.value {
        s.trim().parse::<f64>().unwrap_or(0.0)
    } else {
        0.0
    };
    // Determine step
    let mut step = params
        .multiple_of
        .unwrap_or(if params.is_integer { 1.0 } else { 0.1 });
    if step <= 0.0 {
        step = if params.is_integer { 1.0 } else { 0.1 };
    }
    cur += step * (if dir >= 0 { 1.0 } else { -1.0 });
    // Clamp to bounds
    if let Some(minv) = params.minimum {
        if params.exclusive_minimum {
            if cur <= minv {
                cur = minv + step;
            }
        } else if cur < minv {
            cur = minv;
        }
    }
    if let Some(maxv) = params.maximum {
        if params.exclusive_maximum {
            if cur >= maxv {
                cur = maxv - step;
            }
        } else if cur > maxv {
            cur = maxv;
        }
    }
    // Snap to multiple_of if provided (avoid drift)
    if let Some(m) = params.multiple_of {
        if m > 0.0 {
            let k = (cur / m).round();
            cur = k * m;
        }
    }
    // Integer formatting
    let s = if params.is_integer {
        format!("{cur:.0}")
    } else {
        trim_float(cur)
    };
    fld.value = crate::widgets::form::FieldValue::Text(s);
}

fn trim_float(v: f64) -> String {
    let mut s = format!("{v:.6}");
    while s.contains('.') && s.ends_with('0') {
        s.pop();
    }
    if s.ends_with('.') {
        s.pop();
    }
    if s.is_empty() {
        s.push('0');
    }
    s
}

// Helper to get mutable vec of fields (to satisfy borrow checker in on_key)
// no-op

// (no tests here to avoid requiring widget trait imports in unit context)
