use super::*;
use serde_json::json;

#[test]
fn progress_and_done_update_state() {
    let mut st = AppState::default();
    // Progress updates status
    let _ = update(
        &mut st,
        AppMsg::StreamProgress {
            text: Some("Working".into()),
            percent: Some(12.5),
        },
    );
    assert_eq!(st.status_text.as_deref(), Some("Working"));
    assert_eq!(st.status_percent, Some(12.5));

    // Done with result moves to JSON view and clears status
    let _ = update(
        &mut st,
        AppMsg::StreamDone {
            result: Some(json!({"ok": true})),
            err: None,
        },
    );
    assert!(st.status_text.is_none());
    assert!(st.status_percent.is_none());
    assert!(st.last_error.is_none());
    assert!(st
        .last_json_pretty
        .as_ref()
        .unwrap()
        .contains("\"ok\": true"));
    assert!(matches!(st.view, crate::ui::View::Json));
}

#[test]
fn pane_yaml_effect_builds_expected_effects() {
    use crate::ui::PanelPane;

    // json_viewer with cmd
    let v_cmd = json!({"type":"json_viewer","cmd":"example-app list-items"});
    match super::pane_yaml_effect(PanelPane::A, &v_cmd) {
        Some(Effect::LoadPanelCmd { pane, cmdline }) => {
            assert!(matches!(pane, PanelPane::A));
            assert_eq!(cmdline, "example-app list-items");
        }
        _ => panic!("expected LoadPanelCmd for json_viewer cmd"),
    }

    // json_viewer with yaml
    let v_yaml = json!({"type":"json_viewer","yaml":"config/panel_b.yaml"});
    match super::pane_yaml_effect(PanelPane::B, &v_yaml) {
        Some(Effect::LoadPanelYaml { pane, path }) => {
            assert!(matches!(pane, PanelPane::B));
            assert_eq!(path, "config/panel_b.yaml");
        }
        _ => panic!("expected LoadPanelYaml for json_viewer yaml"),
    }

    // menu with spec
    let v_menu = json!({"type":"menu","spec":"config/nav.yaml"});
    match super::pane_yaml_effect(PanelPane::B, &v_menu) {
        Some(Effect::LoadPanelYaml { pane, path }) => {
            assert!(matches!(pane, PanelPane::B));
            assert_eq!(path, "config/nav.yaml");
        }
        _ => panic!("expected LoadPanelYaml for menu spec"),
    }

    // panel: defer -> None
    let v_panel = json!({"type":"panel"});
    assert!(super::pane_yaml_effect(PanelPane::A, &v_panel).is_none());

    // unknown: None
    let v_unknown = json!({"type":"unknown"});
    assert!(super::pane_yaml_effect(PanelPane::A, &v_unknown).is_none());
}

#[test]
fn loaded_submit_form_maps_nested_error_locations() {
    use crate::ui::{
        PaneContent, PaneData, PanelLayout, PanelPane, PanelRatio, PanelState as UiPanelState,
    };
    // Prepare AppState with a form containing a 'username' field
    let mut st = AppState::default();
    st.panel = Some(UiPanelState {
        layout: PanelLayout::Vertical,
        ratio: PanelRatio::Half,
        a: PaneData::default(),
        b: PaneData::default(),
        b_content: PaneContent::Widget(Box::new(crate::widgets::form_widget::FormWidget::new(
            crate::widgets::form::FormState {
                title: "T".into(),
                fields: vec![crate::widgets::form::FormField {
                    name: "username".into(),
                    label: "Username".into(),
                    required: true,
                    kind: crate::widgets::form::FieldKind::Text,
                    value: crate::widgets::form::FieldValue::Text(String::new()),
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
                }],
                selected: 0,
                editing: false,
                message: None,
                submit_cmd: Some("x".into()),
                disabled: false,
                dirty: false,
                initial: vec![],
                confirm: None,
            },
        ))),
        b_history: Vec::new(),
    });
    // Simulate error envelope with nested loc ["payload","username"]
    let env = json!({
        "ok": false,
        "type": "error",
        "data": { "details": { "errors": [ {"loc": ["payload", "username"], "msg": "Too short"} ] } }
    });
    let _ = update(
        &mut st,
        AppMsg::LoadedSubmitForm {
            pane: PanelPane::B,
            outcome: Ok(LoadOutcome::Fallback(env)),
        },
    );
    if let Some(ps) = &st.panel {
        if let PaneContent::Widget(w) = &ps.b_content {
            if let Some(fw) = w
                .as_any()
                .downcast_ref::<crate::widgets::form_widget::FormWidget>()
            {
                assert_eq!(fw.form.fields[0].error.as_deref(), Some("Too short"));
            } else {
                panic!("expected form widget");
            }
        } else {
            panic!("expected widget");
        }
    } else {
        panic!("missing panel");
    }
}

#[test]
fn validate_form_yaml_reports_field_index_and_name() {
    let v = json!({
        "type": "form",
        "title": "T",
        "fields": [
            {"name": "choice", "type": "select"}
        ]
    });
    let err = super::validate_form_yaml(&v).unwrap_err();
    assert!(err.contains("fields[0]"));
    assert!(err.contains("choice"));
}
