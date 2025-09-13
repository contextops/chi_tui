use crate::app::Effect;
use serde_json::Value as JsonValue;

/// Return a normalized, lowercased widget `type`, if present.
fn spec_type_normalized(v: &JsonValue) -> Option<String> {
    let t = v.get("type").and_then(|s| s.as_str())?.to_ascii_lowercase();
    Some(match t.as_str() {
        "json-viewer" => "json_viewer".to_string(),
        "markdown-viewer" => "markdown".to_string(),
        other => other.to_string(),
    })
}

/// Produce a normalized spec JSON, folding aliases like `json-viewer` -> `json_viewer`.
pub fn normalize_spec(v: &JsonValue) -> Option<JsonValue> {
    let t = spec_type_normalized(v)?;
    let mut obj = v.as_object()?.clone();
    obj.insert("type".to_string(), JsonValue::String(t));
    Some(JsonValue::Object(obj))
}

pub fn resolve_widget_effect(pane: crate::ui::PanelPane, v: &JsonValue) -> Option<Effect> {
    let v = normalize_spec(v)?;
    let t = v
        .get("type")
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .to_string();
    match t.as_str() {
        "json_viewer" => v
            .get("cmd")
            .and_then(|s| s.as_str())
            .map(|cmd| Effect::LoadPanelCmd {
                pane,
                cmdline: cmd.to_string(),
            })
            .or_else(|| {
                v.get("yaml")
                    .and_then(|s| s.as_str())
                    .map(|path| Effect::LoadPanelYaml {
                        pane,
                        path: path.to_string(),
                    })
            }),
        "menu" => v
            .get("spec")
            .and_then(|s| s.as_str())
            .map(|path| Effect::LoadPanelYaml {
                pane,
                path: path.to_string(),
            }),
        "panel" => None,
        _ => None,
    }
}

#[allow(dead_code)]
pub fn resolve_widget(v: &JsonValue) -> Option<Box<dyn crate::widgets::Widget>> {
    let t = spec_type_normalized(v).unwrap_or_default();
    match t.as_str() {
        "panel" => {
            // Build a PanelWidget from inlined spec (synchronous small helper)
            let layout = v.get("layout").and_then(|s| s.as_str());
            let ratio = v.get("size").and_then(|s| s.as_str());
            let mut nested = crate::ui::PanelState {
                layout: crate::ui::parse_panel_layout(layout),
                ratio: crate::ui::parse_panel_ratio(ratio),
                ..Default::default()
            };
            let load_into = |sub: &serde_json::Value, target: &mut crate::ui::PaneData| {
                if let Some(cmd) = sub.get("cmd").and_then(|s| s.as_str()) {
                    match crate::services::cli_runner::run_cmdline_to_json(cmd) {
                        Ok(j) => {
                            target.last_error = None;
                            target.last_json_pretty = Some(
                                serde_json::to_string_pretty(&j).unwrap_or_else(|_| j.to_string()),
                            );
                        }
                        Err(e) => {
                            target.last_error = Some(format!("{e}"));
                            target.last_json_pretty = None;
                        }
                    }
                } else if let Some(path) = sub.get("yaml").and_then(|s| s.as_str()) {
                    let full_path = {
                        let pb = std::path::PathBuf::from(path);
                        if pb.is_absolute() {
                            pb
                        } else if let Ok(dir) = std::env::var("CHI_TUI_CONFIG_DIR") {
                            std::path::PathBuf::from(dir).join(path)
                        } else {
                            std::env::current_dir()
                                .unwrap_or_else(|_| std::path::PathBuf::from("."))
                                .join(path)
                        }
                    };
                    if let Ok(s) = std::fs::read_to_string(&full_path) {
                        match serde_yaml::from_str::<serde_json::Value>(&s) {
                            Ok(j) => {
                                target.last_error = None;
                                target.last_json_pretty = Some(
                                    serde_json::to_string_pretty(&j)
                                        .unwrap_or_else(|_| j.to_string()),
                                );
                            }
                            Err(e) => {
                                target.last_error = Some(format!("{e}"));
                                target.last_json_pretty = None;
                            }
                        }
                    } else {
                        target.last_error = Some(format!("missing file: {path}"));
                        target.last_json_pretty = None;
                    }
                }
            };
            if let Some(a) = v.get("a").and_then(|x| x.as_object()) {
                load_into(&JsonValue::Object(a.clone()), &mut nested.a);
            }
            if let Some(b) = v.get("b").and_then(|x| x.as_object()) {
                load_into(&JsonValue::Object(b.clone()), &mut nested.b);
            }
            let title_a = v
                .get("title_a")
                .and_then(|s| s.as_str())
                .unwrap_or("Pane B.A");
            let title_b = v
                .get("title_b")
                .and_then(|s| s.as_str())
                .unwrap_or("Pane B.B");
            Some(Box::new(
                crate::widgets::panel::PanelWidget::from_panel_state_with_titles(
                    nested, title_a, title_b,
                ),
            ))
        }
        _ => None,
    }
}

/// Build a concrete widget for a given pane from a spec JSON, when possible.
/// Known: `menu` (from AppConfig spec or path), `json_viewer` (placeholder widget).
pub fn resolve_widget_for_pane(
    pane: crate::ui::PanelPane,
    v: &JsonValue,
) -> Option<Box<dyn crate::widgets::Widget>> {
    let t = spec_type_normalized(v)?;
    match t.as_str() {
        "menu" => {
            // Two variants: inline AppConfig under `config`, or external YAML via `spec` path
            if let Some(cfg_v) = v.get("config") {
                if let Ok(cfg) = serde_json::from_value::<crate::model::AppConfig>(cfg_v.clone()) {
                    let default_title = match pane {
                        crate::ui::PanelPane::A => "Pane A — Menu".to_string(),
                        crate::ui::PanelPane::B => "Pane B — Menu".to_string(),
                    };
                    let title = v
                        .get("title")
                        .and_then(|s| s.as_str())
                        .map(|s| s.to_string())
                        .unwrap_or(default_title);
                    return Some(Box::new(crate::widgets::menu::MenuWidget::from_config(
                        title, cfg,
                    )));
                }
            }
            if let Some(path) = v.get("spec").and_then(|s| s.as_str()) {
                let pb = std::path::PathBuf::from(path);
                let full_path = if pb.is_absolute() {
                    pb
                } else if let Ok(dir) = std::env::var("CHI_TUI_CONFIG_DIR") {
                    std::path::PathBuf::from(dir).join(path)
                } else {
                    std::env::current_dir()
                        .unwrap_or_else(|_| std::path::PathBuf::from("."))
                        .join(path)
                };
                if let Ok(s) = std::fs::read_to_string(&full_path) {
                    if let Ok(cfg_v) = serde_yaml::from_str::<serde_json::Value>(&s) {
                        if let Ok(cfg) =
                            serde_json::from_value::<crate::model::AppConfig>(cfg_v.clone())
                        {
                            let default_title = match pane {
                                crate::ui::PanelPane::A => "Pane A — Menu".to_string(),
                                crate::ui::PanelPane::B => "Pane B — Menu".to_string(),
                            };
                            let title = v
                                .get("title")
                                .and_then(|s| s.as_str())
                                .map(|s| s.to_string())
                                .unwrap_or(default_title);
                            return Some(Box::new(crate::widgets::menu::MenuWidget::from_config(
                                title, cfg,
                            )));
                        }
                    }
                }
            }
            None
        }
        "json_viewer" => {
            let default_title = match pane {
                crate::ui::PanelPane::A => "Pane A".to_string(),
                crate::ui::PanelPane::B => "Pane B".to_string(),
            };
            let title = v
                .get("title")
                .and_then(|s| s.as_str())
                .map(|s| s.to_string())
                .unwrap_or(default_title);
            Some(Box::new(
                crate::widgets::json_viewer::JsonViewerWidget::from_text(title, ""),
            ))
        }
        "markdown" => {
            let default_title = match pane {
                crate::ui::PanelPane::A => "Pane A — Markdown".to_string(),
                crate::ui::PanelPane::B => "Pane B — Markdown".to_string(),
            };
            let title = v
                .get("title")
                .and_then(|s| s.as_str())
                .map(|s| s.to_string())
                .unwrap_or(default_title);
            // Support either direct `path` or inline `text`
            if let Some(path) = v.get("path").and_then(|s| s.as_str()) {
                // Resolve relative to CHI_TUI_CONFIG_DIR (or CWD as last resort)
                let pb = std::path::PathBuf::from(path);
                let full = if pb.is_absolute() {
                    pb
                } else if let Ok(dir) = std::env::var("CHI_TUI_CONFIG_DIR") {
                    std::path::PathBuf::from(dir).join(path)
                } else {
                    std::env::current_dir()
                        .unwrap_or_else(|_| std::path::PathBuf::from("."))
                        .join(path)
                };
                return Some(Box::new(
                    crate::widgets::markdown::MarkdownWidget::from_path(title, &full),
                ));
            }
            if let Some(txt) = v.get("text").and_then(|s| s.as_str()) {
                return Some(Box::new(
                    crate::widgets::markdown::MarkdownWidget::from_text(title, txt),
                ));
            }
            None
        }
        "watchdog" => {
            let default_title = match pane {
                crate::ui::PanelPane::A => "Pane A — Watchdog".to_string(),
                crate::ui::PanelPane::B => "Pane B — Watchdog".to_string(),
            };
            let title = v
                .get("title")
                .and_then(|s| s.as_str())
                .map(|s| s.to_string())
                .unwrap_or(default_title);
            let cmds: Vec<String> = v
                .get("commands")
                .and_then(|a| a.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|x| x.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            let sequential = v
                .get("sequential")
                .and_then(|b| b.as_bool())
                .unwrap_or(false);
            let auto_restart = v
                .get("auto_restart")
                .and_then(|b| b.as_bool())
                .unwrap_or(false);
            let max_retries = v.get("max_retries").and_then(|n| n.as_u64()).unwrap_or(0) as usize;
            let restart_delay_ms = v
                .get("restart_delay_ms")
                .and_then(|n| n.as_u64())
                .unwrap_or(1000);
            let stop_on_failure = v
                .get("stop_on_failure")
                .and_then(|b| b.as_bool())
                .unwrap_or(false);
            let allowed_exit_codes: Vec<i32> = v
                .get("allowed_exit_codes")
                .and_then(|a| a.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|x| x.as_i64().map(|i| i as i32))
                        .collect()
                })
                .unwrap_or_else(|| vec![0]);
            let on_panic_exit_cmd = v
                .get("on_panic_exit_cmd")
                .and_then(|s| s.as_str())
                .map(|s| s.to_string());
            let external_check_cmd = v
                .get("external_check_cmd")
                .and_then(|s| s.as_str())
                .map(|s| s.to_string());
            let external_kill_cmd = v
                .get("external_kill_cmd")
                .and_then(|s| s.as_str())
                .map(|s| s.to_string());
            // Allow external-only watchdog when `external_check_cmd` is provided
            if cmds.is_empty() && external_check_cmd.is_none() {
                return None;
            }
            let stats = v
                .get("stats")
                .and_then(|a| a.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|x| {
                            let label = x
                                .get("label")
                                .and_then(|s| s.as_str())
                                .unwrap_or("")
                                .to_string();
                            let regexp = x
                                .get("regexp")
                                .and_then(|s| s.as_str())
                                .unwrap_or("")
                                .to_string();
                            if !label.is_empty() && !regexp.is_empty() {
                                Some(crate::widgets::watchdog::WatchdogStatSpec { label, regexp })
                            } else {
                                None
                            }
                        })
                        .collect()
                })
                .unwrap_or_default();
            let cfg = crate::widgets::watchdog::WatchdogConfig {
                sequential,
                auto_restart,
                max_retries,
                restart_delay_ms,
                allowed_exit_codes,
                stop_on_failure,
                on_panic_exit_cmd,
                stats,
                external_check_cmd,
                external_kill_cmd,
            };
            Some(Box::new(crate::widgets::watchdog::WatchdogWidget::new(
                title, cmds, cfg,
            )))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::PanelPane;
    use serde_json::json;

    #[test]
    fn resolves_json_viewer_cmd_and_yaml() {
        let v_cmd = json!({"type":"json_viewer","cmd":"example-app list-items"});
        match resolve_widget_effect(PanelPane::A, &v_cmd) {
            Some(Effect::LoadPanelCmd { pane, cmdline }) => {
                assert!(matches!(pane, PanelPane::A));
                assert_eq!(cmdline, "example-app list-items");
            }
            _ => panic!("expected LoadPanelCmd"),
        }
        let v_yaml = json!({"type":"json_viewer","yaml":"config/panel_b.yaml"});
        match resolve_widget_effect(PanelPane::B, &v_yaml) {
            Some(Effect::LoadPanelYaml { pane, path }) => {
                assert!(matches!(pane, PanelPane::B));
                assert_eq!(path, "config/panel_b.yaml");
            }
            _ => panic!("expected LoadPanelYaml"),
        }
    }

    #[test]
    fn resolves_panel_widget_from_inline_spec() {
        let v = json!({
            "type": "panel",
            "layout": "horizontal",
            "size": "1:1",
            "a": { "yaml": "config/nav.yaml" },
            "b": { "yaml": "config/nav.yaml" }
        });
        let w = resolve_widget(&v).expect("expected widget");
        assert!(w
            .as_any()
            .downcast_ref::<crate::widgets::panel::PanelWidget>()
            .is_some());
    }

    #[test]
    fn normalize_converts_json_viewer_dash_to_snake() {
        let v = json!({"type": "json-viewer", "cmd": "echo"});
        let n = normalize_spec(&v).expect("normalized");
        assert_eq!(n.get("type").and_then(|s| s.as_str()), Some("json_viewer"));
    }

    #[test]
    fn resolves_menu_widget_from_inline_config() {
        let v = json!({
            "type": "menu",
            "config": {
                "header": "Test",
                "menu": [ {"id": "welcome", "title": "Welcome"} ]
            }
        });
        let w = resolve_widget_for_pane(PanelPane::B, &v).expect("expected widget");
        assert!(w
            .as_any()
            .downcast_ref::<crate::widgets::menu::MenuWidget>()
            .is_some());
    }

    #[test]
    fn resolves_json_viewer_placeholder_widget() {
        let v = json!({"type": "json_viewer", "cmd": "example-app list-items"});
        let w = resolve_widget_for_pane(PanelPane::B, &v).expect("expected widget");
        assert!(w
            .as_any()
            .downcast_ref::<crate::widgets::json_viewer::JsonViewerWidget>()
            .is_some());
    }
}
