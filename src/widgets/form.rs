use crate::widgets::chrome::panel_block;
use ratatui::prelude::*;
use ratatui::widgets::*;
use std::time::Instant;

#[derive(Clone, Debug, PartialEq)]
pub enum FieldValue {
    Text(String),
    Bool(bool),
}

#[derive(Clone, Debug)]
pub enum FieldKind {
    Text,
    Password,
    TextArea {
        edit_lines: usize,
        offset: usize,
    },
    // Numeric input with optional constraints from JSON Schema
    Number {
        is_integer: bool,
        minimum: Option<f64>,
        maximum: Option<f64>,
        exclusive_minimum: bool,
        exclusive_maximum: bool,
        multiple_of: Option<f64>,
    },
    // Array input of primitive items via comma-separated text
    Array {
        item_kind: ArrayItemKind,
        min_items: Option<usize>,
        max_items: Option<usize>,
    },
    Checkbox,
    Select {
        options: Vec<String>,
        values: Vec<String>,
        cursor: usize,
        selected: usize,
        offset: usize,
    },
    MultiSelect {
        options: Vec<String>,
        values: Vec<String>,
        cursor: usize,
        selected: Vec<bool>,
        offset: usize,
    },
}

#[derive(Clone, Debug)]
pub enum ArrayItemKind {
    String,
    Integer,
    Number,
}

pub const OPTIONS_VISIBLE: usize = 8;

#[derive(Clone, Debug)]
pub struct FormField {
    pub name: String,
    pub label: String,
    pub required: bool,
    pub kind: FieldKind,
    pub value: FieldValue,
    pub error: Option<String>,
    // Text-specific constraints (from JSON Schema)
    pub text_min_len: Option<usize>,
    pub text_max_len: Option<usize>,
    pub text_pattern: Option<String>,
    // TextArea-specific
    pub textarea_max_lines: Option<usize>,
    // Dynamic options (for select/multiselect)
    pub dyn_options_cmd: Option<String>,
    pub dyn_unwrap: Option<String>,
    pub dyn_loaded: bool,
    pub dyn_loaded_at: Option<Instant>,
    // Grouping & ordering
    pub group: Option<String>,
    pub order: Option<i32>,
}

#[derive(Clone, Debug, Default)]
pub struct FormState {
    pub title: String,
    pub fields: Vec<FormField>,
    pub selected: usize,
    pub editing: bool,
    pub message: Option<String>,
    pub submit_cmd: Option<String>,
    pub disabled: bool,
    pub dirty: bool,
    pub initial: Vec<FieldInitial>,
    pub confirm: Option<ConfirmAction>,
}

#[derive(Clone, Debug)]
pub struct FieldInitial {
    pub name: String,
    pub value: FieldValue,
    pub select_value: Option<String>,
    pub multi_values: Option<Vec<String>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConfirmAction {
    Reset,
    Cancel,
}

pub fn capture_initial(form: &mut FormState) {
    let mut init: Vec<FieldInitial> = Vec::new();
    for f in &form.fields {
        let mut fi = FieldInitial {
            name: f.name.clone(),
            value: f.value.clone(),
            select_value: None,
            multi_values: None,
        };
        match &f.kind {
            FieldKind::Select {
                values, selected, ..
            } => {
                let val = values.get(*selected).cloned();
                fi.select_value = val;
            }
            FieldKind::MultiSelect {
                values, selected, ..
            } => {
                let mut arr: Vec<String> = Vec::new();
                for (i, v) in values.iter().enumerate() {
                    if *selected.get(i).unwrap_or(&false) {
                        arr.push(v.clone());
                    }
                }
                fi.multi_values = Some(arr);
            }
            _ => {}
        }
        init.push(fi);
    }
    form.initial = init;
    form.dirty = false;
}

pub fn compute_dirty(form: &mut FormState) -> bool {
    let mut any = false;
    for f in &form.fields {
        if let Some(init) = form.initial.iter().find(|i| i.name == f.name) {
            match &f.kind {
                FieldKind::Select {
                    values, selected, ..
                } => {
                    let cur = values.get(*selected).cloned();
                    if cur != init.select_value {
                        any = true;
                        break;
                    }
                }
                FieldKind::MultiSelect {
                    values, selected, ..
                } => {
                    let mut cur_set: Vec<String> = Vec::new();
                    for (i, v) in values.iter().enumerate() {
                        if *selected.get(i).unwrap_or(&false) {
                            cur_set.push(v.clone());
                        }
                    }
                    let base = init.multi_values.clone().unwrap_or_default();
                    if cur_set != base {
                        any = true;
                        break;
                    }
                }
                FieldKind::Checkbox => {
                    if f.value != init.value {
                        any = true;
                        break;
                    }
                }
                _ => {
                    if f.value != init.value {
                        any = true;
                        break;
                    }
                }
            }
        }
    }
    form.dirty = any;
    any
}

pub fn reset_to_initial(form: &mut FormState) {
    for f in &mut form.fields {
        if let Some(init) = form.initial.iter().find(|i| i.name == f.name) {
            match &mut f.kind {
                FieldKind::Select {
                    values,
                    selected,
                    cursor,
                    offset,
                    ..
                } => {
                    if let Some(target) = &init.select_value {
                        if let Some(idx) = values.iter().position(|v| v == target) {
                            *selected = idx;
                        }
                    }
                    *cursor = *selected;
                    *offset = 0;
                }
                FieldKind::MultiSelect {
                    values,
                    selected,
                    cursor,
                    offset,
                    ..
                } => {
                    if let Some(targets) = &init.multi_values {
                        for (i, v) in values.iter().enumerate() {
                            let wanted = targets.iter().any(|t| t == v);
                            if let Some(slot) = selected.get_mut(i) {
                                *slot = wanted;
                            }
                        }
                    } else {
                        for b in selected.iter_mut() {
                            *b = false;
                        }
                    }
                    *cursor = 0;
                    *offset = 0;
                }
                FieldKind::Checkbox => {
                    f.value = init.value.clone();
                }
                _ => {
                    f.value = init.value.clone();
                }
            }
            f.error = None;
        }
    }
    form.message = Some("Reset to defaults".into());
    compute_dirty(form);
}

pub fn draw_form(
    f: &mut Frame,
    area: Rect,
    form: &mut FormState,
    highlight: bool,
    cursor_on: bool,
) {
    let mut lines: Vec<Line> = Vec::new();
    let mut last_group: Option<String> = None;
    for (i, fld) in form.fields.iter().enumerate() {
        if let Some(g) = &fld.group {
            if last_group.as_ref() != Some(g) {
                lines.push(Line::from(Span::styled(
                    format!("-- {g} --"),
                    Style::default()
                        .fg(Color::Magenta)
                        .add_modifier(Modifier::BOLD),
                )));
                last_group = Some(g.clone());
            }
        }
        let sel = if i == form.selected { '›' } else { ' ' };
        let req = if fld.required { " *" } else { "" };
        match &fld.kind {
            FieldKind::Text => {
                let mut val = match &fld.value {
                    FieldValue::Text(s) => s.clone(),
                    FieldValue::Bool(b) => {
                        if *b {
                            "On".into()
                        } else {
                            "Off".into()
                        }
                    }
                };
                if form.editing && i == form.selected && cursor_on {
                    val.push('▏');
                }
                let value_style = if i == form.selected {
                    if form.editing {
                        crate::theme::text_editing_bold()
                    } else {
                        crate::theme::text_active_bold()
                    }
                } else {
                    Style::default()
                };
                lines.push(Line::from(vec![
                    Span::raw(format!("{sel} {}{req}: ", fld.label)),
                    Span::styled(val, value_style),
                ]));
            }
            FieldKind::Password => {
                // Render masked, keep actual text in value
                let mut masked = String::new();
                if let FieldValue::Text(s) = &fld.value {
                    let n = s.chars().count();
                    masked = "•".repeat(n);
                }
                if form.editing && i == form.selected && cursor_on {
                    masked.push('▏');
                }
                let value_style = if i == form.selected {
                    if form.editing {
                        crate::theme::text_editing_bold()
                    } else {
                        crate::theme::text_active_bold()
                    }
                } else {
                    Style::default()
                };
                lines.push(Line::from(vec![
                    Span::raw(format!("{sel} {}{req}: ", fld.label)),
                    Span::styled(masked, value_style),
                ]));
            }
            FieldKind::TextArea { edit_lines, offset } => {
                let value_style = if i == form.selected {
                    if form.editing {
                        crate::theme::text_editing_bold()
                    } else {
                        crate::theme::text_active_bold()
                    }
                } else {
                    Style::default()
                };
                // Header line
                lines.push(Line::from(vec![Span::raw(format!(
                    "{sel} {}{req}:",
                    fld.label
                ))]));
                // Body lines (indented)
                let mut text = String::new();
                if let FieldValue::Text(s) = &fld.value {
                    text = s.clone();
                }
                let mut body_lines: Vec<String> = if text.is_empty() {
                    vec![String::new()]
                } else {
                    text.lines().map(|l| l.to_string()).collect()
                };
                // Fold in non-edit mode based on textarea_max_lines
                if !form.editing {
                    if let Some(maxl) = fld.textarea_max_lines {
                        if body_lines.len() > maxl {
                            body_lines.truncate(maxl);
                            lines.push(Line::from(vec![
                                Span::raw("  "),
                                Span::styled(body_lines[0].clone(), value_style),
                            ]));
                            for bl in body_lines.iter().skip(1) {
                                lines.push(Line::from(vec![
                                    Span::raw("  "),
                                    Span::styled(bl.clone(), value_style),
                                ]));
                            }
                            let more = text.lines().count() - maxl;
                            lines.push(Line::from(Span::styled(
                                format!(
                                    "  … ({} more line{})",
                                    more,
                                    if more == 1 { "" } else { "s" }
                                ),
                                crate::theme::text_muted(),
                            )));
                            continue;
                        }
                    }
                }
                if form.editing && i == form.selected {
                    let total = body_lines.len();
                    let h = *edit_lines;
                    let start = (*offset).min(total);
                    let end = (start + h).min(total);
                    let mut window: Vec<String> = body_lines
                        .iter()
                        .skip(start)
                        .take(end - start)
                        .cloned()
                        .collect();
                    if cursor_on {
                        if let Some(last) = window.last_mut() {
                            last.push('▏');
                        }
                    }
                    for bl in window {
                        lines.push(Line::from(vec![
                            Span::raw("  "),
                            Span::styled(bl, value_style),
                        ]));
                    }
                } else {
                    if form.editing && cursor_on {
                        if let Some(last) = body_lines.last_mut() {
                            last.push('▏');
                        }
                    }
                    for bl in body_lines {
                        lines.push(Line::from(vec![
                            Span::raw("  "),
                            Span::styled(bl, value_style),
                        ]));
                    }
                }
            }
            FieldKind::Number { .. } => {
                let mut val = match &fld.value {
                    FieldValue::Text(s) => s.clone(),
                    FieldValue::Bool(b) => {
                        if *b {
                            "1".into()
                        } else {
                            "0".into()
                        }
                    }
                };
                if form.editing && i == form.selected && cursor_on {
                    val.push('▏');
                }
                let value_style = if i == form.selected {
                    if form.editing {
                        crate::theme::text_editing_bold()
                    } else {
                        crate::theme::text_active_bold()
                    }
                } else {
                    Style::default()
                };
                lines.push(Line::from(vec![
                    Span::raw(format!("{sel} {}{req}: ", fld.label)),
                    Span::styled(val, value_style),
                ]));
            }
            FieldKind::Array { .. } => {
                let mut val = match &fld.value {
                    FieldValue::Text(s) => s.clone(),
                    FieldValue::Bool(_) => String::new(),
                };
                if form.editing && i == form.selected && cursor_on {
                    val.push('▏');
                }
                let value_style = if i == form.selected {
                    if form.editing {
                        crate::theme::text_editing_bold()
                    } else {
                        crate::theme::text_active_bold()
                    }
                } else {
                    Style::default()
                };
                lines.push(Line::from(vec![
                    Span::raw(format!("{sel} {}{req}: ", fld.label)),
                    Span::styled(val, value_style),
                ]));
            }
            FieldKind::Checkbox => {
                let checked = matches!(fld.value, FieldValue::Bool(true));
                let val = if checked { "[x]" } else { "[ ]" };
                let value_style = if i == form.selected {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                lines.push(Line::from(vec![
                    Span::raw(format!("{sel} {}{req}: ", fld.label)),
                    Span::styled(val.to_string(), value_style),
                ]));
            }
            FieldKind::Select {
                options,
                cursor,
                selected,
                offset,
                ..
            } => {
                // Header line with current selection summary
                let summary = options
                    .get(*selected)
                    .cloned()
                    .unwrap_or_else(|| "(none)".into());
                let header_style = if i == form.selected && form.editing {
                    crate::theme::text_editing_bold()
                } else if i == form.selected {
                    crate::theme::text_active_bold()
                } else {
                    Style::default()
                };
                lines.push(Line::from(vec![
                    Span::raw(format!("{sel} {}{req}: ", fld.label)),
                    Span::styled(summary, header_style),
                ]));
                // Options list when editing this field
                if form.editing && i == form.selected {
                    let start = (*offset).min(options.len());
                    let end = (start + OPTIONS_VISIBLE).min(options.len());
                    for (oi, opt) in options.iter().enumerate().take(end).skip(start) {
                        let mark = if oi == *selected { "(•)" } else { "( )" };
                        let cur = if oi == *cursor { '›' } else { ' ' };
                        let st = if oi == *cursor {
                            crate::theme::list_cursor_style()
                        } else {
                            crate::theme::text_muted()
                        };
                        lines.push(Line::from(vec![Span::styled(
                            format!("  {cur} {mark} {opt}"),
                            st,
                        )]));
                    }
                }
            }
            FieldKind::MultiSelect {
                options,
                cursor,
                selected,
                offset,
                ..
            } => {
                // Header with count summary
                let count = selected.iter().filter(|b| **b).count();
                let summary = format!("{count} selected");
                let header_style = if i == form.selected && form.editing {
                    Style::default()
                        .fg(Color::Rgb(255, 165, 0))
                        .add_modifier(Modifier::BOLD)
                } else if i == form.selected {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                lines.push(Line::from(vec![
                    Span::raw(format!("{sel} {}{req}: ", fld.label)),
                    Span::styled(summary, header_style),
                ]));
                if form.editing && i == form.selected {
                    let start = (*offset).min(options.len());
                    let end = (start + OPTIONS_VISIBLE).min(options.len());
                    for (oi, opt) in options.iter().enumerate().take(end).skip(start) {
                        let chk = if *selected.get(oi).unwrap_or(&false) {
                            "[x]"
                        } else {
                            "[ ]"
                        };
                        let cur = if oi == *cursor { '›' } else { ' ' };
                        let st = if oi == *cursor {
                            Style::default()
                                .fg(Color::Black)
                                .bg(Color::Rgb(255, 165, 0))
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(Color::DarkGray)
                        };
                        lines.push(Line::from(vec![Span::styled(
                            format!("  {cur} {chk} {opt}"),
                            st,
                        )]));
                    }
                }
            }
        }
        if let Some(err) = &fld.error {
            lines.push(Line::from(Span::styled(
                format!("  ! {err}"),
                crate::theme::text_error(),
            )));
        }
    }
    // Buttons: Save | Reset | Cancel
    if !form.fields.is_empty() {
        lines.push(Line::from(""));
    }
    let save_idx = form.fields.len();
    let reset_idx = form.fields.len() + 1;
    let cancel_idx = form.fields.len() + 2;
    let can_save = form.submit_cmd.is_some() && !form.disabled && form.dirty;
    let can_reset = form.dirty && !form.disabled;
    let save_label = "[ Save ]";
    let mut save_style = if can_save {
        crate::theme::text_active_bold()
    } else {
        crate::theme::text_muted()
    };
    let reset_label = "Reset";
    let mut reset_style = if can_reset {
        Style::default().fg(crate::theme::ACTIVE)
    } else {
        crate::theme::text_muted()
    };
    let mut cancel_style = crate::theme::text_muted();
    if form.selected == save_idx {
        save_style = if can_save {
            crate::theme::list_cursor_style()
        } else {
            Style::default()
                .fg(crate::theme::MUTED)
                .bg(crate::theme::ACCENT)
        };
    }
    if form.selected == reset_idx {
        reset_style = crate::theme::list_cursor_style();
    }
    if form.selected == cancel_idx {
        cancel_style = crate::theme::list_cursor_style();
    }
    lines.push(Line::from(vec![
        Span::styled(format!("  {save_label}  "), save_style),
        Span::styled(format!("{reset_label}  "), reset_style),
        Span::styled("Cancel", cancel_style),
    ]));
    if let Some(msg) = &form.message {
        lines.push(Line::from(Span::styled(
            msg.clone(),
            crate::theme::text_muted(),
        )));
    }
    let title = if form.editing {
        format!("{} — editing", form.title)
    } else {
        form.title.clone()
    };
    let block = panel_block(&title, highlight);
    let p = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });
    f.render_widget(p, area);
}

pub fn kebab_case(name: &str) -> String {
    let mut out = String::new();
    for (i, ch) in name.chars().enumerate() {
        if ch == '_' {
            out.push('-');
        } else if ch.is_uppercase() {
            if i > 0 {
                out.push('-');
            }
            for c in ch.to_lowercase() {
                out.push(c);
            }
        } else {
            out.push(ch);
        }
    }
    out
}

pub fn build_cmdline(form: &FormState) -> Option<String> {
    let base = form.submit_cmd.clone()?;
    let mut parts: Vec<String> = vec![base];
    for fld in &form.fields {
        match (&fld.kind, &fld.value) {
            (FieldKind::Checkbox, FieldValue::Bool(b)) => {
                if *b {
                    parts.push(format!("--{}", kebab_case(&fld.name)));
                }
            }
            (
                FieldKind::Select {
                    options,
                    values,
                    selected,
                    ..
                },
                _,
            ) => {
                if !options.is_empty() {
                    let v = values
                        .get(*selected)
                        .cloned()
                        .unwrap_or_else(|| options.get(*selected).cloned().unwrap_or_default());
                    if !v.is_empty() {
                        parts.push(format!("--{}", kebab_case(&fld.name)));
                        if v.contains(' ') {
                            parts.push(format!("'{}'", v.replace("'", "'\\''")));
                        } else {
                            parts.push(v);
                        }
                    }
                }
            }
            (
                FieldKind::MultiSelect {
                    options,
                    values,
                    selected,
                    ..
                },
                _,
            ) => {
                for (oi, on) in selected.iter().enumerate() {
                    if *on {
                        let v = values
                            .get(oi)
                            .cloned()
                            .unwrap_or_else(|| options.get(oi).cloned().unwrap_or_default());
                        if !v.is_empty() {
                            parts.push(format!("--{}", kebab_case(&fld.name)));
                            if v.contains(' ') {
                                parts.push(format!("'{}'", v.replace("'", "'\\''")));
                            } else {
                                parts.push(v);
                            }
                        }
                    }
                }
            }
            (FieldKind::Array { .. }, FieldValue::Text(s)) => {
                let items: Vec<String> = s
                    .split(',')
                    .map(|t| t.trim())
                    .filter(|t| !t.is_empty())
                    .map(|t| t.to_string())
                    .collect();
                for it in items {
                    parts.push(format!("--{}", kebab_case(&fld.name)));
                    if it.contains(' ') {
                        parts.push(format!("'{}'", it.replace("'", "'\\''")));
                    } else {
                        parts.push(it);
                    }
                }
            }
            (FieldKind::Number { .. }, FieldValue::Text(s)) | (_, FieldValue::Text(s)) => {
                if !s.is_empty() {
                    parts.push(format!("--{}", kebab_case(&fld.name)));
                    // naive quoting if whitespace/newlines present
                    if s.contains(' ') || s.contains('\n') || s.contains('\t') {
                        parts.push(format!("'{}'", s.replace("'", "'\\''")));
                    } else {
                        parts.push(s.clone());
                    }
                }
            }
            _ => {}
        }
    }
    Some(parts.join(" "))
}

/// Build form fields from a JSON Schema-like object (Pydantic input_schema).
/// Supports: required flags, enums -> select, arrays with items.enum -> multiselect,
/// numbers/integers -> number, booleans -> checkbox, strings -> text.
pub fn fields_from_json_schema(input_schema: &serde_json::Value) -> Vec<FormField> {
    use std::collections::HashSet;
    let mut fields: Vec<FormField> = Vec::new();
    let required_list: HashSet<String> = input_schema
        .get("required")
        .and_then(|x| x.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    if let Some(props) = input_schema.get("properties").and_then(|x| x.as_object()) {
        for (name, prop) in props.iter() {
            let ty = prop
                .get("type")
                .and_then(|s| s.as_str())
                .unwrap_or("string")
                .to_ascii_lowercase();
            let label = prop
                .get("title")
                .and_then(|s| s.as_str())
                .unwrap_or(name)
                .to_string();
            let required = required_list.contains(name);
            let kind = if let Some(en) = prop.get("enum").and_then(|x| x.as_array()) {
                let opts: Vec<String> = en
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
                FieldKind::Select {
                    options: opts.clone(),
                    values: opts,
                    cursor: 0,
                    selected: 0,
                    offset: 0,
                }
            } else if ty == "array" {
                if let Some(items) = prop.get("items").and_then(|x| x.as_object()) {
                    if let Some(en) = items.get("enum").and_then(|x| x.as_array()) {
                        let opts: Vec<String> = en
                            .iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect();
                        let sel = vec![false; opts.len()];
                        FieldKind::MultiSelect {
                            options: opts.clone(),
                            values: opts,
                            cursor: 0,
                            selected: sel,
                            offset: 0,
                        }
                    } else {
                        let itype = items
                            .get("type")
                            .and_then(|s| s.as_str())
                            .unwrap_or("string")
                            .to_ascii_lowercase();
                        let item_kind = match itype.as_str() {
                            "integer" => ArrayItemKind::Integer,
                            "number" => ArrayItemKind::Number,
                            _ => ArrayItemKind::String,
                        };
                        let min_items = prop
                            .get("minItems")
                            .and_then(|x| x.as_u64())
                            .map(|x| x as usize);
                        let max_items = prop
                            .get("maxItems")
                            .and_then(|x| x.as_u64())
                            .map(|x| x as usize);
                        FieldKind::Array {
                            item_kind,
                            min_items,
                            max_items,
                        }
                    }
                } else {
                    FieldKind::Array {
                        item_kind: ArrayItemKind::String,
                        min_items: None,
                        max_items: None,
                    }
                }
            } else {
                match ty.as_str() {
                    "boolean" => FieldKind::Checkbox,
                    "integer" | "number" => FieldKind::Number {
                        is_integer: ty == "integer",
                        minimum: prop.get("minimum").and_then(|x| x.as_f64()),
                        maximum: prop.get("maximum").and_then(|x| x.as_f64()),
                        exclusive_minimum: prop
                            .get("exclusiveMinimum")
                            .and_then(|x| x.as_bool())
                            .unwrap_or(false),
                        exclusive_maximum: prop
                            .get("exclusiveMaximum")
                            .and_then(|x| x.as_bool())
                            .unwrap_or(false),
                        multiple_of: prop.get("multipleOf").and_then(|x| x.as_f64()),
                    },
                    _ => FieldKind::Text,
                }
            };
            let field = FormField {
                name: name.to_string(),
                label,
                required,
                kind,
                value: FieldValue::Text(String::new()),
                error: None,
                text_min_len: None,
                text_max_len: None,
                text_pattern: None,
                textarea_max_lines: None,
                dyn_options_cmd: None,
                dyn_unwrap: None,
                dyn_loaded: false,
                dyn_loaded_at: None,
                group: None,
                order: None,
            };
            fields.push(field);
        }
    }
    fields
}

/// Attempt to populate fields based on the CLI's `schema` output for the configured submit_cmd.
#[allow(dead_code)]
pub fn populate_fields_from_cli_schema(form: &mut FormState) {
    if form.fields.is_empty() {
        if let Some(cmdline) = &form.submit_cmd {
            if let Some((prog, cmd_name)) = shlex::split(cmdline).and_then(|parts| {
                if parts.len() >= 2 {
                    Some((parts[0].clone(), parts[1].clone()))
                } else {
                    None
                }
            }) {
                let schema_cmd = format!("{} {}", prog, "schema");
                if let Ok(schema_env) =
                    crate::services::cli_runner::run_cmdline_to_json(&schema_cmd)
                {
                    let data = schema_env
                        .get("data")
                        .cloned()
                        .unwrap_or(serde_json::Value::Null);
                    let commands = data
                        .get("commands")
                        .and_then(|x| x.as_array())
                        .cloned()
                        .unwrap_or_default();
                    if let Some(spec) = commands
                        .iter()
                        .find(|c| c.get("name").and_then(|s| s.as_str()) == Some(cmd_name.as_str()))
                    {
                        if let Some(inp) = spec.get("input_schema") {
                            form.fields = fields_from_json_schema(inp);
                        }
                    }
                }
            }
        }
    }
}

pub fn validate_form(form: &mut FormState) -> bool {
    let mut ok = true;
    for fld in &mut form.fields {
        fld.error = None;
        match (&mut fld.kind, &mut fld.value) {
            (FieldKind::Text, FieldValue::Text(s)) => {
                let st = s.trim();
                if fld.required && st.is_empty() {
                    fld.error = Some("This field is required".into());
                    ok = false;
                }
                if let Some(minl) = fld.text_min_len {
                    if st.len() < minl {
                        fld.error = Some(format!("Must be at least {minl} characters"));
                        ok = false;
                    }
                }
                if let Some(maxl) = fld.text_max_len {
                    if st.len() > maxl {
                        fld.error = Some(format!("Must be at most {maxl} characters"));
                        ok = false;
                    }
                }
                if let Some(pat) = &fld.text_pattern {
                    if let Ok(re) = regex::Regex::new(pat) {
                        if !st.is_empty() && !re.is_match(st) {
                            fld.error = Some("Does not match required pattern".into());
                            ok = false;
                        }
                    }
                }
            }
            (
                FieldKind::Number {
                    is_integer,
                    minimum,
                    maximum,
                    exclusive_minimum,
                    exclusive_maximum,
                    multiple_of,
                },
                FieldValue::Text(s),
            ) => {
                let raw = s.trim();
                if fld.required && raw.is_empty() {
                    fld.error = Some("This field is required".into());
                    ok = false;
                } else if !raw.is_empty() {
                    if *is_integer {
                        match raw.parse::<i64>() {
                            Ok(mut v) => {
                                // exclusive/inclusive bounds validation or clamp
                                if let Some(minv) = minimum {
                                    let m = (*minv).floor() as i64;
                                    if *exclusive_minimum {
                                        if v <= m {
                                            fld.error = Some(format!("Must be > {m}"));
                                            ok = false;
                                        }
                                    } else if v < m {
                                        v = m;
                                    }
                                }
                                if let Some(maxv) = maximum {
                                    let m = (*maxv).ceil() as i64;
                                    if *exclusive_maximum {
                                        if v >= m {
                                            fld.error = Some(format!("Must be < {m}"));
                                            ok = false;
                                        }
                                    } else if v > m {
                                        v = m;
                                    }
                                }
                                if let Some(mof) = multiple_of {
                                    let mof_i = (*mof).round() as i64;
                                    if mof_i != 0 && v % mof_i != 0 {
                                        fld.error = Some(format!("Must be a multiple of {mof_i}"));
                                        ok = false;
                                    }
                                }
                                // update possibly clamped
                                if fld.error.is_none() {
                                    *s = v.to_string();
                                }
                            }
                            Err(_) => {
                                fld.error = Some("Invalid integer".into());
                                ok = false;
                            }
                        }
                    } else {
                        match raw.parse::<f64>() {
                            Ok(mut v) => {
                                if let Some(minv) = minimum {
                                    if *exclusive_minimum {
                                        if v <= *minv {
                                            fld.error = Some(format!("Must be > {minv}"));
                                            ok = false;
                                        }
                                    } else if v < *minv {
                                        v = *minv;
                                    }
                                }
                                if let Some(maxv) = maximum {
                                    if *exclusive_maximum {
                                        if v >= *maxv {
                                            fld.error = Some(format!("Must be < {maxv}"));
                                            ok = false;
                                        }
                                    } else if v > *maxv {
                                        v = *maxv;
                                    }
                                }
                                if let Some(mof) = multiple_of {
                                    let ratio = v / *mof;
                                    let nearest = ratio.round();
                                    if (ratio - nearest).abs() > 1e-9 {
                                        fld.error = Some(format!("Must be a multiple of {mof}"));
                                        ok = false;
                                    }
                                }
                                if fld.error.is_none() {
                                    if v.fract().abs() < 1e-12 {
                                        *s = format!("{v:.0}");
                                    } else {
                                        *s = v.to_string();
                                    }
                                }
                            }
                            Err(_) => {
                                fld.error = Some("Invalid number".into());
                                ok = false;
                            }
                        }
                    }
                }
            }
            (FieldKind::Checkbox, _) => {
                // no-op for required checkboxes
            }
            (
                FieldKind::Array {
                    item_kind,
                    min_items,
                    max_items,
                },
                FieldValue::Text(s),
            ) => {
                let items: Vec<&str> = s
                    .split(',')
                    .map(|t| t.trim())
                    .filter(|t| !t.is_empty())
                    .collect();
                if fld.required && items.is_empty() {
                    fld.error = Some("Please provide at least one item".into());
                    ok = false;
                }
                if let Some(mi) = *min_items {
                    if items.len() < mi {
                        fld.error = Some(format!("At least {mi} item(s) required"));
                        ok = false;
                    }
                }
                if let Some(mx) = *max_items {
                    if items.len() > mx {
                        fld.error = Some(format!("At most {mx} item(s) allowed"));
                        ok = false;
                    }
                }
                // Per-item type validation for numeric kinds
                if fld.error.is_none() {
                    match item_kind {
                        ArrayItemKind::Integer => {
                            for (i, it) in items.iter().enumerate() {
                                if it.parse::<i64>().is_err() {
                                    fld.error = Some(format!("Invalid integer at #{}", i + 1));
                                    ok = false;
                                    break;
                                }
                            }
                        }
                        ArrayItemKind::Number => {
                            for (i, it) in items.iter().enumerate() {
                                if it.parse::<f64>().is_err() {
                                    fld.error = Some(format!("Invalid number at #{}", i + 1));
                                    ok = false;
                                    break;
                                }
                            }
                        }
                        ArrayItemKind::String => {}
                    }
                }
            }
            (FieldKind::Select { options, .. }, _) => {
                if fld.required && options.is_empty() {
                    fld.error = Some("No options available".into());
                    ok = false;
                }
            }
            (
                FieldKind::MultiSelect {
                    options, selected, ..
                },
                _,
            ) => {
                if fld.required && !selected.iter().any(|b| *b) {
                    fld.error = Some("Please select at least one".into());
                    ok = false;
                }
                if options.is_empty() {
                    fld.error = Some("No options available".into());
                    ok = false;
                }
            }
            _ => {}
        }
    }
    if !ok {
        form.message = Some("Please fix the highlighted errors".into());
    } else {
        form.message = None;
    }
    ok
}

// Validate a single text field inline while editing (for immediate feedback)
pub fn validate_text_inline(fld: &mut FormField) {
    if let FieldValue::Text(s) = &fld.value {
        let st = s.trim();
        fld.error = None;
        if fld.required && st.is_empty() {
            fld.error = Some("This field is required".into());
            return;
        }
        if let Some(minl) = fld.text_min_len {
            if st.len() < minl {
                fld.error = Some(format!("Must be at least {minl} characters"));
                return;
            }
        }
        if let Some(maxl) = fld.text_max_len {
            if st.len() > maxl {
                fld.error = Some(format!("Must be at most {maxl} characters"));
                return;
            }
        }
        if let Some(pat) = &fld.text_pattern {
            if let Ok(re) = regex::Regex::new(pat) {
                if !st.is_empty() && !re.is_match(st) {
                    fld.error = Some("Does not match required pattern".into());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn num_field(
        is_int: bool,
        min: Option<f64>,
        max: Option<f64>,
        excl_min: bool,
        excl_max: bool,
        multiple: Option<f64>,
        val: &str,
    ) -> FormField {
        FormField {
            name: "n".into(),
            label: "N".into(),
            required: true,
            kind: FieldKind::Number {
                is_integer: is_int,
                minimum: min,
                maximum: max,
                exclusive_minimum: excl_min,
                exclusive_maximum: excl_max,
                multiple_of: multiple,
            },
            value: FieldValue::Text(val.into()),
            error: None,
            text_min_len: None,
            text_max_len: None,
            text_pattern: None,
            textarea_max_lines: None,
            dyn_options_cmd: None,
            dyn_unwrap: None,
            dyn_loaded: false,
            dyn_loaded_at: None,
            group: None,
            order: None,
        }
    }

    #[test]
    fn validate_number_clamps_inclusive_bounds() {
        let mut form = FormState {
            title: "t".into(),
            fields: vec![num_field(
                true,
                Some(0.0),
                Some(10.0),
                false,
                false,
                None,
                "100",
            )],
            selected: 0,
            editing: false,
            message: None,
            submit_cmd: Some("prog cmd".into()),
            disabled: false,
            dirty: false,
            initial: vec![],
            confirm: None,
        };
        assert!(validate_form(&mut form));
        match &form.fields[0].value {
            FieldValue::Text(s) => assert_eq!(s, "10"),
            _ => panic!(),
        }
    }

    #[test]
    fn validate_number_exclusive_raises_error() {
        let mut form = FormState {
            title: "t".into(),
            fields: vec![num_field(
                false,
                Some(0.0),
                Some(1.0),
                true,
                true,
                None,
                "1.0",
            )],
            selected: 0,
            editing: false,
            message: None,
            submit_cmd: Some("x".into()),
            disabled: false,
            dirty: false,
            initial: vec![],
            confirm: None,
        };
        assert!(!validate_form(&mut form));
        assert!(form.fields[0].error.as_deref().unwrap().contains("< 1"));
    }

    #[test]
    fn validate_array_min_max_and_types() {
        let mut fld = FormField {
            name: "nums".into(),
            label: "Numbers".into(),
            required: true,
            kind: FieldKind::Array {
                item_kind: ArrayItemKind::Integer,
                min_items: Some(1),
                max_items: Some(2),
            },
            value: FieldValue::Text("1, 2, 3".into()),
            error: None,
            text_min_len: None,
            text_max_len: None,
            text_pattern: None,
            textarea_max_lines: None,
            dyn_options_cmd: None,
            dyn_unwrap: None,
            dyn_loaded: false,
            dyn_loaded_at: None,
            group: None,
            order: None,
        };
        let mut form = FormState {
            title: "t".into(),
            fields: vec![fld.clone()],
            selected: 0,
            editing: false,
            message: None,
            submit_cmd: Some("x".into()),
            disabled: false,
            dirty: false,
            initial: vec![],
            confirm: None,
        };
        assert!(!validate_form(&mut form));
        assert!(form.fields[0]
            .error
            .as_deref()
            .unwrap()
            .contains("At most 2"));
        // invalid item
        fld.value = FieldValue::Text("1, a".into());
        form.fields[0] = fld.clone();
        assert!(!validate_form(&mut form));
        assert!(form.fields[0]
            .error
            .as_deref()
            .unwrap()
            .contains("Invalid integer"));
    }

    #[test]
    fn build_cmdline_mixed_fields() {
        let mut form = FormState {
            title: "t".into(),
            fields: vec![],
            selected: 0,
            editing: false,
            message: None,
            submit_cmd: Some("prog sub".into()),
            disabled: false,
            dirty: false,
            initial: vec![],
            confirm: None,
        };
        form.fields.push(FormField {
            name: "name".into(),
            label: "Name".into(),
            required: true,
            kind: FieldKind::Text,
            value: FieldValue::Text("Ada".into()),
            error: None,
            text_min_len: None,
            text_max_len: None,
            text_pattern: None,
            textarea_max_lines: None,
            dyn_options_cmd: None,
            dyn_unwrap: None,
            dyn_loaded: false,
            dyn_loaded_at: None,
            group: None,
            order: None,
        });
        form.fields.push(FormField {
            name: "shout".into(),
            label: "Shout".into(),
            required: false,
            kind: FieldKind::Checkbox,
            value: FieldValue::Bool(true),
            error: None,
            text_min_len: None,
            text_max_len: None,
            text_pattern: None,
            textarea_max_lines: None,
            dyn_options_cmd: None,
            dyn_unwrap: None,
            dyn_loaded: false,
            dyn_loaded_at: None,
            group: None,
            order: None,
        });
        form.fields.push(FormField {
            name: "color".into(),
            label: "Color".into(),
            required: false,
            kind: FieldKind::Select {
                options: vec!["red".into(), "blue".into()],
                values: vec!["r".into(), "b".into()],
                cursor: 0,
                selected: 1,
                offset: 0,
            },
            value: FieldValue::Text(String::new()),
            error: None,
            text_min_len: None,
            text_max_len: None,
            text_pattern: None,
            textarea_max_lines: None,
            dyn_options_cmd: None,
            dyn_unwrap: None,
            dyn_loaded: false,
            dyn_loaded_at: None,
            group: None,
            order: None,
        });
        form.fields.push(FormField {
            name: "tags".into(),
            label: "Tags".into(),
            required: false,
            kind: FieldKind::MultiSelect {
                options: vec!["one".into(), "two".into()],
                values: vec!["1".into(), "2".into()],
                cursor: 0,
                selected: vec![true, false],
                offset: 0,
            },
            value: FieldValue::Text(String::new()),
            error: None,
            text_min_len: None,
            text_max_len: None,
            text_pattern: None,
            textarea_max_lines: None,
            dyn_options_cmd: None,
            dyn_unwrap: None,
            dyn_loaded: false,
            dyn_loaded_at: None,
            group: None,
            order: None,
        });
        form.fields.push(FormField {
            name: "nums".into(),
            label: "Nums".into(),
            required: false,
            kind: FieldKind::Array {
                item_kind: ArrayItemKind::String,
                min_items: None,
                max_items: None,
            },
            value: FieldValue::Text("a, b".into()),
            error: None,
            text_min_len: None,
            text_max_len: None,
            text_pattern: None,
            textarea_max_lines: None,
            dyn_options_cmd: None,
            dyn_unwrap: None,
            dyn_loaded: false,
            dyn_loaded_at: None,
            group: None,
            order: None,
        });
        let cmd = build_cmdline(&form).unwrap();
        assert!(cmd.contains("prog sub"));
        assert!(cmd.contains("--name Ada"));
        assert!(cmd.contains("--shout"));
        assert!(cmd.contains("--color b"));
        assert!(cmd.contains("--tags 1"));
        assert!(cmd.contains("--nums a"));
        assert!(cmd.contains("--nums b"));
    }

    #[test]
    fn golden_select_editor_renders_expected_window() {
        // Prepare a form with a single required Select field in editing mode
        let options = vec![
            "Alpha".to_string(),
            "Bravo".to_string(),
            "Charlie".to_string(),
            "Delta".to_string(),
            "Echo".to_string(),
            "Foxtrot".to_string(),
            "Golf".to_string(),
            "Hotel".to_string(),
            "India".to_string(),
        ];
        let field = FormField {
            name: "pick".into(),
            label: "Pick".into(),
            required: true,
            kind: FieldKind::Select {
                options: options.clone(),
                values: options.clone(),
                cursor: 1,
                selected: 1,
                offset: 0,
            },
            value: FieldValue::Text(String::new()),
            error: None,
            text_min_len: None,
            text_max_len: None,
            text_pattern: None,
            textarea_max_lines: None,
            dyn_options_cmd: None,
            dyn_unwrap: None,
            dyn_loaded: false,
            dyn_loaded_at: None,
            group: None,
            order: None,
        };
        let mut form = FormState {
            title: "Select Test".into(),
            fields: vec![field],
            selected: 0,
            editing: true,
            message: None,
            submit_cmd: Some("x".into()),
            disabled: false,
            dirty: false,
            initial: vec![],
            confirm: None,
        };
        let backend = TestBackend::new(40, 12);
        let mut terminal = Terminal::new(backend).unwrap();
        let _ = terminal.draw(|f| {
            let area = ratatui::layout::Rect {
                x: 0,
                y: 0,
                width: 40,
                height: 12,
            };
            draw_form(f, area, &mut form, true, true);
        });
        // Extract inner content (strip 1-char border)
        let buf = terminal.backend().buffer().clone();
        let mut inner_lines: Vec<String> = Vec::new();
        for y in 1..(buf.area.height - 1) {
            let mut line = String::new();
            for x in 1..(buf.area.width - 1) {
                let cell = &buf[(x, y)];
                let ch = cell.symbol().chars().next().unwrap_or(' ');
                line.push(ch);
            }
            while line.ends_with(' ') {
                line.pop();
            }
            inner_lines.push(line);
        }
        // Compare first lines with golden snapshot (ignore Save/Cancel area for stability)
        let current_top = inner_lines
            .iter()
            .take(9)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
        let golden = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/golden/select_editor.txt"
        ));
        assert_eq!(current_top.trim_end(), golden.trim_end());
    }

    #[test]
    fn golden_multiselect_editor_renders_expected_window() {
        let options = vec![
            "Alpha".to_string(),
            "Bravo".to_string(),
            "Charlie".to_string(),
            "Delta".to_string(),
            "Echo".to_string(),
            "Foxtrot".to_string(),
            "Golf".to_string(),
            "Hotel".to_string(),
            "India".to_string(),
            "Juliet".to_string(),
        ];
        let mut selected_flags = vec![false; options.len()];
        selected_flags[2] = true; // Charlie
        selected_flags[5] = true; // Foxtrot
        let field = FormField {
            name: "pick".into(),
            label: "Pick".into(),
            required: true,
            kind: FieldKind::MultiSelect {
                options: options.clone(),
                values: options.clone(),
                cursor: 3,
                selected: selected_flags,
                offset: 2,
            },
            value: FieldValue::Text(String::new()),
            error: None,
            text_min_len: None,
            text_max_len: None,
            text_pattern: None,
            textarea_max_lines: None,
            dyn_options_cmd: None,
            dyn_unwrap: None,
            dyn_loaded: false,
            dyn_loaded_at: None,
            group: None,
            order: None,
        };
        let mut form = FormState {
            title: "MultiSelect Test".into(),
            fields: vec![field],
            selected: 0,
            editing: true,
            message: None,
            submit_cmd: Some("x".into()),
            disabled: false,
            dirty: false,
            initial: vec![],
            confirm: None,
        };
        let backend = TestBackend::new(40, 12);
        let mut terminal = Terminal::new(backend).unwrap();
        let _ = terminal.draw(|f| {
            let area = ratatui::layout::Rect {
                x: 0,
                y: 0,
                width: 40,
                height: 12,
            };
            draw_form(f, area, &mut form, true, true);
        });
        let buf = terminal.backend().buffer().clone();
        let mut inner_lines: Vec<String> = Vec::new();
        for y in 1..(buf.area.height - 1) {
            let mut line = String::new();
            for x in 1..(buf.area.width - 1) {
                let cell = &buf[(x, y)];
                let ch = cell.symbol().chars().next().unwrap_or(' ');
                line.push(ch);
            }
            while line.ends_with(' ') {
                line.pop();
            }
            inner_lines.push(line);
        }
        let current_top = inner_lines
            .iter()
            .take(10)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
        let golden = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/golden/multiselect_editor.txt"
        ));
        assert_eq!(current_top.trim_end(), golden.trim_end());
    }

    #[test]
    fn fields_from_schema_maps_required_and_types() {
        use serde_json::json;
        let schema = json!({
            "required": ["age", "name"],
            "properties": {
                "name": {"type": "string", "title": "Full Name"},
                "age": {"type": "integer", "minimum": 0},
                "color": {"type": "string", "enum": ["red","green"]},
                "tags": {"type": "array", "items": {"enum": ["a","b"]}},
                "agree": {"type": "boolean"}
            }
        });
        let fields = super::fields_from_json_schema(&schema);
        assert_eq!(fields.len(), 5);
        let age = fields.iter().find(|f| f.name == "age").unwrap();
        assert!(age.required);
        match age.kind {
            super::FieldKind::Number {
                is_integer,
                minimum,
                ..
            } => {
                assert!(is_integer);
                assert_eq!(minimum, Some(0.0));
            }
            _ => panic!("age not number"),
        }
        let color = fields.iter().find(|f| f.name == "color").unwrap();
        match &color.kind {
            super::FieldKind::Select { options, .. } => {
                assert_eq!(options, &vec!["red", "green"]);
            }
            _ => panic!("color not select"),
        }
        let tags = fields.iter().find(|f| f.name == "tags").unwrap();
        match &tags.kind {
            super::FieldKind::MultiSelect { options, .. } => {
                assert_eq!(options, &vec!["a", "b"]);
            }
            _ => panic!("tags not multiselect"),
        }
        let agree = fields.iter().find(|f| f.name == "agree").unwrap();
        match agree.kind {
            super::FieldKind::Checkbox => {}
            _ => panic!("agree not checkbox"),
        }
    }
}
