use crate::model::MenuItem;
use crate::ui::{AppState, LoadOutcome};
use serde_json::Value as JsonValue;
use std::time::Instant;

pub enum AppMsg {
    EnterMenu(MenuItem),
    EnterChild {
        key: String,
        val: JsonValue,
    },
    RefreshMenu(MenuItem),
    RefreshChild {
        key: String,
        val: JsonValue,
    },
    LoadedMenu {
        key: String,
        outcome: Result<LoadOutcome, String>,
    },
    LoadedChild {
        key: String,
        outcome: Result<LoadOutcome, String>,
    },
    LoadedPanel {
        pane: super::ui::PanelPane,
        outcome: Result<LoadOutcome, String>,
    },
    LoadedNested {
        subpane: super::ui::PanelPane,
        outcome: Result<LoadOutcome, String>,
    },
    LoadedSubmitForm {
        pane: super::ui::PanelPane,
        outcome: Result<LoadOutcome, String>,
    },
    LoadedFormOptions {
        key: String,
        outcome: Result<LoadOutcome, String>,
    },
    StreamProgress {
        text: Option<String>,
        percent: Option<f64>,
    },
    StreamDone {
        result: Option<JsonValue>,
        err: Option<String>,
    },
}

#[allow(clippy::large_enum_variant)]
pub enum Effect {
    LoadMenu {
        mi: MenuItem,
        key: String,
    },
    LoadChild {
        val: JsonValue,
        key: String,
    },
    RunStream {
        cmdline: String,
        title: String,
    },
    LoadPanelCmd {
        pane: super::ui::PanelPane,
        cmdline: String,
    },
    LoadPanelYaml {
        pane: super::ui::PanelPane,
        path: String,
    },
    SubmitForm {
        pane: super::ui::PanelPane,
        cmdline: String,
    },
    CancelForm {
        pane: super::ui::PanelPane,
    },
    LoadFormOptions {
        field: String,
        cmdline: String,
        unwrap: Option<String>,
        force: bool,
    },
    ShowToast {
        text: String,
        level: crate::ui::ToastLevel,
        seconds: u64,
    },
}

pub fn update(state: &mut AppState, msg: AppMsg) -> Vec<Effect> {
    use AppMsg::*;
    let mut effects: Vec<Effect> = Vec::new();
    match msg {
        EnterMenu(mi) => {
            // Support static hierarchical children: toggle expand/collapse and seed children map.
            let has_static_children = mi.children.as_ref().map(|v| !v.is_empty()).unwrap_or(false);
            if has_static_children && !super::ui::is_lazy(&mi) && !super::ui::is_autoload(&mi) {
                let key = crate::nav::keys::menu_key(&mi);
                if !state.children.contains_key(&key) {
                    state
                        .children
                        .insert(key.clone(), mi.children.clone().unwrap_or_default());
                }
                if state.expanded.contains(&key) {
                    state.expanded.remove(&key);
                } else {
                    state.expanded.insert(key);
                }
            }
            // Pane B (panel mode) intercept: handle menu items locally in Pane B
            if matches!(state.view, super::ui::View::Panel)
                && matches!(state.panel_focus, super::ui::PanelPane::B)
            {
                // If explicitly marked as streaming, bypass Pane B and run as global stream
                if mi.stream.unwrap_or(false) {
                    if let Some(cmdline) = mi.command.clone() {
                        effects.push(Effect::RunStream {
                            cmdline,
                            title: mi.title.clone(),
                        });
                        return effects;
                    }
                }
                // Update Pane B title override for upcoming content
                state.pane_b_title = mi.pane_b_title.clone();
                if super::ui::is_panel(&mi) {
                    // Build nested panel from MenuItem fields (synchronous fill)
                    let mut nested = super::ui::PanelState {
                        layout: super::ui::parse_panel_layout(mi.panel_layout.as_deref()),
                        ratio: super::ui::parse_panel_ratio(mi.panel_size.as_deref()),
                        ..Default::default()
                    };
                    // Fill A
                    if let Some(cmd) = mi.pane_a_cmd.clone() {
                        if let Ok(j) = crate::services::cli_runner::run_cmdline_to_json(&cmd) {
                            nested.a.last_error = None;
                            nested.a.last_json_pretty = Some(
                                serde_json::to_string_pretty(&j).unwrap_or_else(|_| j.to_string()),
                            );
                        }
                    } else if let Some(path) = mi.pane_a_yaml.clone() {
                        let full_path = {
                            let pb = std::path::PathBuf::from(&path);
                            if pb.is_absolute() {
                                pb
                            } else if let Ok(dir) = std::env::var("CHI_TUI_CONFIG_DIR") {
                                std::path::PathBuf::from(dir).join(&path)
                            } else {
                                std::env::current_dir()
                                    .unwrap_or_else(|_| std::path::PathBuf::from("."))
                                    .join(&path)
                            }
                        };
                        if let Ok(s) = std::fs::read_to_string(&full_path) {
                            if let Ok(j) = serde_yaml::from_str::<serde_json::Value>(&s) {
                                nested.a.last_error = None;
                                nested.a.last_json_pretty = Some(
                                    serde_json::to_string_pretty(&j)
                                        .unwrap_or_else(|_| j.to_string()),
                                );
                            }
                        }
                    }
                    // Fill B
                    if let Some(cmd) = mi.pane_b_cmd.clone() {
                        if let Ok(j) = crate::services::cli_runner::run_cmdline_to_json(&cmd) {
                            nested.b.last_error = None;
                            nested.b.last_json_pretty = Some(
                                serde_json::to_string_pretty(&j).unwrap_or_else(|_| j.to_string()),
                            );
                        }
                    } else if let Some(path) = mi.pane_b_yaml.clone() {
                        let full_path = {
                            let pb = std::path::PathBuf::from(&path);
                            if pb.is_absolute() {
                                pb
                            } else if let Ok(dir) = std::env::var("CHI_TUI_CONFIG_DIR") {
                                std::path::PathBuf::from(dir).join(&path)
                            } else {
                                std::env::current_dir()
                                    .unwrap_or_else(|_| std::path::PathBuf::from("."))
                                    .join(&path)
                            }
                        };
                        if let Ok(s) = std::fs::read_to_string(&full_path) {
                            if let Ok(j) = serde_yaml::from_str::<serde_json::Value>(&s) {
                                nested.b.last_error = None;
                                nested.b.last_json_pretty = Some(
                                    serde_json::to_string_pretty(&j)
                                        .unwrap_or_else(|_| j.to_string()),
                                );
                            }
                        }
                    }
                    if let Some(_ps) = &mut state.panel {
                        super::ui::pane_b_replace_with_widget(
                            state,
                            Box::new(crate::widgets::panel::PanelWidget::from_panel_state(nested)),
                            true,
                        );
                    }
                    return effects;
                }
                if super::ui::is_markdown(&mi) {
                    if let Some(_ps) = &mut state.panel {
                        let title = mi
                            .pane_b_title
                            .clone()
                            .unwrap_or_else(|| "Pane B — Markdown".to_string());
                        if let Some(text) = mi.content.clone() {
                            super::ui::pane_b_replace_with_widget(
                                state,
                                Box::new(crate::widgets::markdown::MarkdownWidget::from_text(
                                    title, &text,
                                )),
                                true,
                            );
                        } else if let Some(path) = mi.path.clone() {
                            let pb = std::path::PathBuf::from(&path);
                            let full = if pb.is_absolute() {
                                pb
                            } else if let Ok(dir) = std::env::var("CHI_TUI_CONFIG_DIR") {
                                std::path::PathBuf::from(dir).join(&path)
                            } else {
                                std::env::current_dir()
                                    .unwrap_or_else(|_| std::path::PathBuf::from("."))
                                    .join(&path)
                            };
                            super::ui::pane_b_replace_with_widget(
                                state,
                                Box::new(crate::widgets::markdown::MarkdownWidget::from_path(
                                    title, &full,
                                )),
                                true,
                            );
                        } else {
                            super::ui::pane_b_replace_with_widget(
                                state,
                                Box::new(crate::widgets::markdown::MarkdownWidget::from_text(
                                    title, "",
                                )),
                                true,
                            );
                        }
                    }
                    return effects;
                }
                if super::ui::is_watchdog(&mi) {
                    if let Some(_ps) = &mut state.panel {
                        let title = mi
                            .pane_b_title
                            .clone()
                            .unwrap_or_else(|| "Pane B — Watchdog".to_string());
                        let cmds = mi.commands.clone().unwrap_or_default();
                        let cfg = crate::widgets::watchdog::WatchdogConfig {
                            sequential: mi.sequential.unwrap_or(false),
                            auto_restart: mi.auto_restart.unwrap_or(false),
                            max_retries: mi.max_retries.unwrap_or(0) as usize,
                            restart_delay_ms: mi.restart_delay_ms.unwrap_or(1000) as u64,
                            allowed_exit_codes: mi
                                .allowed_exit_codes
                                .clone()
                                .unwrap_or_else(|| vec![0]),
                            stop_on_failure: mi.stop_on_failure.unwrap_or(false),
                            on_panic_exit_cmd: mi.on_panic_exit_cmd.clone(),
                            stats: vec![],
                            external_check_cmd: mi.external_check_cmd.clone(),
                            external_kill_cmd: mi.external_kill_cmd.clone(),
                        };
                        // Reuse or create a persistent watchdog session by menu key
                        let key = crate::nav::keys::menu_key(&mi);
                        let (session, reused) =
                            if let Some(s) = state.watchdog_sessions.get(&key).cloned() {
                                (s, true)
                            } else {
                                let s = crate::widgets::watchdog::WatchdogSession::create(
                                    cmds.clone(),
                                    crate::widgets::watchdog::WatchdogConfig {
                                        stats: cfg.stats.clone(),
                                        ..cfg
                                    },
                                );
                                state.watchdog_sessions.insert(key.clone(), s.clone());
                                (s, false)
                            };
                        if reused {
                            state.dbg(format!("watchdog: reusing session for {key}"));
                        } else {
                            state.dbg(format!("watchdog: creating session for {key}"));
                        }
                        super::ui::pane_b_replace_with_widget(
                            state,
                            Box::new(crate::widgets::watchdog::WatchdogWidget::from_session(
                                title, &session,
                            )),
                            true,
                        );
                    }
                    return effects;
                }
                if super::ui::is_lazy(&mi) || super::ui::is_autoload(&mi) {
                    match crate::services::loader::load_lazy_children_cmd(&mi) {
                        Ok(crate::services::loader::Loaded::Items(arr)) => {
                            if let Some(ps) = &mut state.panel {
                                ps.b.last_error = None;
                                let v = serde_json::Value::Array(arr);
                                let title = state
                                    .pane_b_title
                                    .clone()
                                    .unwrap_or_else(|| "Pane B".to_string());
                                super::ui::pane_b_replace_with_widget(
                                    state,
                                    Box::new(
                                        crate::widgets::result_viewer::ResultViewerWidget::new(
                                            title, v,
                                        ),
                                    ),
                                    true,
                                );
                            }
                            return effects;
                        }
                        Ok(crate::services::loader::Loaded::ItemsWithPagination {
                            items, ..
                        }) => {
                            // For now, just show items in panel B
                            if let Some(ps) = &mut state.panel {
                                ps.b.last_error = None;
                                let v = serde_json::Value::Array(items);
                                let title = state
                                    .pane_b_title
                                    .clone()
                                    .unwrap_or_else(|| "Pane B".to_string());
                                super::ui::pane_b_replace_with_widget(
                                    state,
                                    Box::new(
                                        crate::widgets::result_viewer::ResultViewerWidget::new(
                                            title, v,
                                        ),
                                    ),
                                    true,
                                );
                            }
                            return effects;
                        }
                        Ok(crate::services::loader::Loaded::Fallback(v)) => {
                            if let Some(ps) = &mut state.panel {
                                ps.b.last_error = None;
                                let title = state
                                    .pane_b_title
                                    .clone()
                                    .unwrap_or_else(|| "Pane B".to_string());
                                super::ui::pane_b_replace_with_widget(
                                    state,
                                    Box::new(
                                        crate::widgets::result_viewer::ResultViewerWidget::new(
                                            title, v,
                                        ),
                                    ),
                                    true,
                                );
                            }
                            return effects;
                        }
                        Err(e) => {
                            let msg = e.to_string();
                            if let Some(ps) = &mut state.panel {
                                ps.b.last_error = Some(msg.clone());
                                ps.b.last_json_pretty = None;
                            }
                            let title = state
                                .pane_b_title
                                .clone()
                                .unwrap_or_else(|| "Pane B".to_string());
                            super::ui::pane_b_replace_with_widget(
                                state,
                                Box::new(
                                    crate::widgets::json_viewer::JsonViewerWidget::from_error(
                                        title, msg,
                                    ),
                                ),
                                true,
                            );
                            return effects;
                        }
                    }
                }
                if let Some(cmdline) = mi.command.clone() {
                    if mi.stream.unwrap_or(false) {
                        effects.push(Effect::RunStream {
                            cmdline,
                            title: mi.title.clone(),
                        });
                    } else {
                        effects.push(Effect::LoadPanelCmd {
                            pane: super::ui::PanelPane::B,
                            cmdline,
                        });
                    }
                    return effects;
                }
            }
            if mi.id == "welcome" && mi.command.is_none() {
                state.view = super::ui::View::Welcome;
                return effects;
            }
            if super::ui::is_lazy(&mi) {
                let key = crate::nav::keys::menu_key(&mi);
                if !state.expanded.contains(&key) {
                    state.loading.insert(key.clone());
                    state.expanded.insert(key.clone());
                    effects.push(Effect::LoadMenu { mi, key });
                } else {
                    state.expanded.remove(&key);
                }
            } else if super::ui::is_autoload(&mi) {
                let key = crate::nav::keys::menu_key(&mi);
                if !state.expanded.contains(&key) {
                    if super::ui::expand_on_enter_menu(&mi) {
                        state.loading.insert(key.clone());
                        state.expanded.insert(key.clone());
                        effects.push(Effect::LoadMenu { mi, key });
                    } else {
                        state.expanded.insert(key);
                    }
                } else {
                    state.expanded.remove(&key);
                }
            } else if let Some(cmdline) = mi.command.clone() {
                if mi.stream.unwrap_or(false) {
                    let run_title = mi.title.clone();
                    state.status_text = Some(format!("Running: {run_title}"));
                    state.status_percent = None;
                    effects.push(Effect::RunStream {
                        cmdline,
                        title: run_title,
                    });
                } else if state.view == super::ui::View::Panel {
                    // In panel mode, route command output to Pane B (master-detail UX)
                    effects.push(Effect::LoadPanelCmd {
                        pane: super::ui::PanelPane::B,
                        cmdline,
                    });
                } else {
                    let run_title = mi.title.clone();
                    state.status_text = Some(format!("Running: {run_title}"));
                    state.status_percent = None;
                    effects.push(Effect::RunStream {
                        cmdline,
                        title: run_title,
                    });
                }
            } else if super::ui::is_panel(&mi) {
                // Initialize panel state from MenuItem
                let layout = super::ui::parse_panel_layout(mi.panel_layout.as_deref());
                let ratio = super::ui::parse_panel_ratio(mi.panel_size.as_deref());
                state.panel = Some(super::ui::PanelState {
                    layout,
                    ratio,
                    a: super::ui::PaneData::default(),
                    b: super::ui::PaneData::default(),
                    b_content: super::ui::PaneContent::Widget(Box::new(
                        crate::widgets::json_viewer::JsonViewerWidget::from_text("Pane B", ""),
                    )),
                    b_history: Vec::new(),
                });
                // Reset Pane B back history when opening a new panel
                state.pane_b_title_stack.clear();
                // Apply custom Pane B title if provided
                state.pane_b_title = mi.pane_b_title.clone();
                state.view = super::ui::View::Panel;
                // UX: new panel -> focus on B automatically
                state.panel_focus = super::ui::PanelPane::B;
                state.panel_nested_focus = super::ui::PanelPane::A;
                if let Some(cmd) = mi.pane_a_cmd.clone() {
                    effects.push(Effect::LoadPanelCmd {
                        pane: super::ui::PanelPane::A,
                        cmdline: cmd,
                    });
                }
                if let Some(cmd) = mi.pane_b_cmd.clone() {
                    effects.push(Effect::LoadPanelCmd {
                        pane: super::ui::PanelPane::B,
                        cmdline: cmd,
                    });
                }
                if let Some(path) = mi.pane_a_yaml.clone() {
                    effects.push(Effect::LoadPanelYaml {
                        pane: super::ui::PanelPane::A,
                        path,
                    });
                }
                if let Some(path) = mi.pane_b_yaml.clone() {
                    effects.push(Effect::LoadPanelYaml {
                        pane: super::ui::PanelPane::B,
                        path,
                    });
                }
            } else if super::ui::is_markdown(&mi) {
                // Build single-panel view with Markdown in Pane B
                state.panel = Some(super::ui::PanelState {
                    layout: super::ui::PanelLayout::Horizontal,
                    ratio: super::ui::PanelRatio::Half,
                    a: super::ui::PaneData::default(),
                    b: super::ui::PaneData::default(),
                    b_content: super::ui::PaneContent::Widget(Box::new(
                        crate::widgets::markdown::MarkdownWidget::from_text(
                            mi.pane_b_title
                                .clone()
                                .unwrap_or_else(|| "Pane B — Markdown".to_string()),
                            &mi.content.clone().unwrap_or_default(),
                        ),
                    )),
                    b_history: Vec::new(),
                });
                state.pane_b_title_stack.clear();
                state.view = super::ui::View::Panel;
                state.panel_focus = super::ui::PanelPane::B;
                state.panel_nested_focus = super::ui::PanelPane::A;
                if let Some(path) = mi.path.clone() {
                    let pb = std::path::PathBuf::from(&path);
                    let full = if pb.is_absolute() {
                        pb
                    } else if let Ok(dir) = std::env::var("CHI_TUI_CONFIG_DIR") {
                        std::path::PathBuf::from(dir).join(&path)
                    } else {
                        std::env::current_dir()
                            .unwrap_or_else(|_| std::path::PathBuf::from("."))
                            .join(&path)
                    };
                    if state.panel.is_some() {
                        super::ui::pane_b_replace_with_widget(
                            state,
                            Box::new(crate::widgets::markdown::MarkdownWidget::from_path(
                                mi.pane_b_title
                                    .clone()
                                    .unwrap_or_else(|| "Pane B — Markdown".to_string()),
                                &full,
                            )),
                            true,
                        );
                    }
                }
                return effects;
            } else if super::ui::is_watchdog(&mi) {
                state.panel = Some(super::ui::PanelState {
                    layout: super::ui::PanelLayout::Vertical,
                    ratio: super::ui::PanelRatio::Half,
                    a: super::ui::PaneData::default(),
                    b: super::ui::PaneData::default(),
                    b_content: super::ui::PaneContent::Widget(Box::new(
                        crate::widgets::json_viewer::JsonViewerWidget::from_text("Pane B", ""),
                    )),
                    b_history: Vec::new(),
                });
                state.pane_b_title_stack.clear();
                state.view = super::ui::View::Panel;
                state.panel_focus = super::ui::PanelPane::B;
                state.panel_nested_focus = super::ui::PanelPane::A;
                let cmds = mi.commands.clone().unwrap_or_default();
                let cfg = crate::widgets::watchdog::WatchdogConfig {
                    sequential: mi.sequential.unwrap_or(false),
                    auto_restart: mi.auto_restart.unwrap_or(false),
                    max_retries: mi.max_retries.unwrap_or(0) as usize,
                    restart_delay_ms: mi.restart_delay_ms.unwrap_or(1000) as u64,
                    allowed_exit_codes: mi.allowed_exit_codes.clone().unwrap_or_else(|| vec![0]),
                    stop_on_failure: mi.stop_on_failure.unwrap_or(false),
                    on_panic_exit_cmd: mi.on_panic_exit_cmd.clone(),
                    stats: vec![],
                    external_check_cmd: mi.external_check_cmd.clone(),
                    external_kill_cmd: mi.external_kill_cmd.clone(),
                };
                if state.panel.is_some() {
                    let key = crate::nav::keys::menu_key(&mi);
                    let (session, reused) =
                        if let Some(s) = state.watchdog_sessions.get(&key).cloned() {
                            (s, true)
                        } else {
                            let s = crate::widgets::watchdog::WatchdogSession::create(
                                cmds.clone(),
                                crate::widgets::watchdog::WatchdogConfig {
                                    stats: cfg.stats.clone(),
                                    ..cfg
                                },
                            );
                            state.watchdog_sessions.insert(key.clone(), s.clone());
                            (s, false)
                        };
                    if reused {
                        state.dbg(format!("watchdog: reusing session for {key}"));
                    } else {
                        state.dbg(format!("watchdog: creating session for {key}"));
                    }
                    super::ui::pane_b_replace_with_widget(
                        state,
                        Box::new(crate::widgets::watchdog::WatchdogWidget::from_session(
                            mi.pane_b_title
                                .clone()
                                .unwrap_or_else(|| "Pane B — Watchdog".to_string()),
                            &session,
                        )),
                        true,
                    );
                }
                return effects;
            }
        }
        EnterChild { key, val } => {
            // Toggle static nested children when a child node contains an inline 'children' array
            if let Some(arr) = val.get("children").and_then(|c| c.as_array()) {
                if !state.children.contains_key(&key) {
                    state.children.insert(key.clone(), arr.clone());
                }
                if state.expanded.contains(&key) {
                    state.expanded.remove(&key);
                } else {
                    state.expanded.insert(key.clone());
                }
                return effects;
            }
            // Check if this is a pagination control
            if val
                .get("__is_pagination")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                // This is a pagination item - reload the parent list with new page
                if let Some(cmd) = val.get("command").and_then(|c| c.as_str()) {
                    // Determine parent menu key. Child keys are formatted as
                    //   "menu:<parent_id>/<child_id_or_index>"
                    // so take the segment before the first '/'.
                    let parent_key = key
                        .split('/')
                        .next()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| key.clone());

                    // Find the parent menu item to reload it with new command
                    if let Some(parent_mi) = state
                        .config
                        .menu
                        .iter()
                        .find(|mi| crate::nav::keys::menu_key(mi) == parent_key)
                        .cloned()
                    {
                        // Create a modified menu item with the pagination command
                        let mut paginated_mi = parent_mi;
                        paginated_mi.command = Some(cmd.to_string());

                        // Clear existing children and reload
                        state.children.remove(&parent_key);
                        state.loading.insert(parent_key.clone());

                        // Load the new page using Effect
                        effects.push(Effect::LoadMenu {
                            mi: paginated_mi,
                            key: parent_key,
                        });
                    }
                }
                return effects;
            }

            // Check if this is an info item (page indicator)
            if val
                .get("__is_info")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                // Info items are not interactive - do nothing
                return effects;
            }

            if super::ui::is_lazy_value(&val)
                || (super::ui::is_autoload_value(&val) && super::ui::expand_on_enter_value(&val))
            {
                if !state.expanded.contains(&key) {
                    state.loading.insert(key.clone());
                    state.expanded.insert(key.clone());
                    effects.push(Effect::LoadChild { val, key });
                } else {
                    state.expanded.remove(&key);
                }
            // Check for widget hint in the item
            } else if let Some(widget_type) = val.get("widget").and_then(|w| w.as_str()) {
                match widget_type {
                    "markdown" => {
                        // Handle markdown widget for list items
                        if state.view != super::ui::View::Panel {
                            // Switch to panel view for markdown display
                            state.panel = Some(super::ui::PanelState {
                                layout: super::ui::PanelLayout::Horizontal,
                                ratio: super::ui::PanelRatio::Half,
                                a: super::ui::PaneData::default(),
                                b: super::ui::PaneData::default(),
                                b_content: super::ui::PaneContent::Widget(Box::new(
                                    crate::widgets::markdown::MarkdownWidget::from_text(
                                        super::ui::title_from_value(&val),
                                        "",
                                    ),
                                )),
                                b_history: Vec::new(),
                            });
                            state.view = super::ui::View::Panel;
                            state.panel_focus = super::ui::PanelPane::B;
                        }

                        // Load markdown content from path or content field
                        if let Some(_ps) = &mut state.panel {
                            let title = super::ui::title_from_value(&val);

                            if let Some(path) = val.get("path").and_then(|p| p.as_str()) {
                                // Load from file path
                                let pb = std::path::PathBuf::from(path);
                                let full = if pb.is_absolute() {
                                    pb
                                } else if let Ok(dir) = std::env::var("CHI_TUI_CONFIG_DIR") {
                                    // Config dir already points to .tui, so strip .tui/ from path if present
                                    let clean_path = path.strip_prefix(".tui/").unwrap_or(path);
                                    std::path::PathBuf::from(dir).join(clean_path)
                                } else {
                                    std::env::current_dir()
                                        .unwrap_or_else(|_| std::path::PathBuf::from("."))
                                        .join(path)
                                };
                                super::ui::pane_b_replace_with_widget(
                                    state,
                                    Box::new(crate::widgets::markdown::MarkdownWidget::from_path(
                                        title, &full,
                                    )),
                                    true,
                                );
                            } else if let Some(content) =
                                val.get("content").and_then(|c| c.as_str())
                            {
                                // Use inline content
                                super::ui::pane_b_replace_with_widget(
                                    state,
                                    Box::new(crate::widgets::markdown::MarkdownWidget::from_text(
                                        title, content,
                                    )),
                                    true,
                                );
                            } else if let Some(cmd) = val.get("command").and_then(|c| c.as_str()) {
                                // Fall back to command execution for content
                                effects.push(Effect::LoadPanelCmd {
                                    pane: super::ui::PanelPane::B,
                                    cmdline: cmd.to_string(),
                                });
                            }
                        }
                    }
                    "watchdog" => {
                        // Handle watchdog widget for list items
                        if let Some(commands) = val.get("commands").and_then(|c| c.as_array()) {
                            let cmds: Vec<String> = commands
                                .iter()
                                .filter_map(|v| v.as_str().map(String::from))
                                .collect();

                            if !cmds.is_empty() {
                                if state.view != super::ui::View::Panel {
                                    state.panel = Some(super::ui::PanelState {
                                        layout: super::ui::PanelLayout::Vertical,
                                        ratio: super::ui::PanelRatio::Half,
                                        a: super::ui::PaneData::default(),
                                        b: super::ui::PaneData::default(),
                                        b_content: super::ui::PaneContent::Widget(Box::new(
                                            crate::widgets::json_viewer::JsonViewerWidget::from_text("Watchdog", ""),
                                        )),
                                        b_history: Vec::new(),
                                    });
                                    state.view = super::ui::View::Panel;
                                    state.panel_focus = super::ui::PanelPane::B;
                                }

                                if let Some(_ps) = &mut state.panel {
                                    let title = super::ui::title_from_value(&val);
                                    let cfg = crate::widgets::watchdog::WatchdogConfig {
                                        sequential: val
                                            .get("sequential")
                                            .and_then(|v| v.as_bool())
                                            .unwrap_or(false),
                                        auto_restart: val
                                            .get("auto_restart")
                                            .and_then(|v| v.as_bool())
                                            .unwrap_or(false),
                                        max_retries: val
                                            .get("max_retries")
                                            .and_then(|v| v.as_u64())
                                            .unwrap_or(0)
                                            as usize,
                                        restart_delay_ms: val
                                            .get("restart_delay_ms")
                                            .and_then(|v| v.as_u64())
                                            .unwrap_or(1000),
                                        allowed_exit_codes: val
                                            .get("allowed_exit_codes")
                                            .and_then(|v| v.as_array())
                                            .map(|arr| {
                                                arr.iter()
                                                    .filter_map(|v| v.as_i64().map(|i| i as i32))
                                                    .collect()
                                            })
                                            .unwrap_or_else(|| vec![0]),
                                        stop_on_failure: val
                                            .get("stop_on_failure")
                                            .and_then(|v| v.as_bool())
                                            .unwrap_or(false),
                                        on_panic_exit_cmd: val
                                            .get("on_panic_exit_cmd")
                                            .and_then(|v| v.as_str())
                                            .map(String::from),
                                        stats: val
                                            .get("stats")
                                            .and_then(|a| a.as_array())
                                            .map(|arr| {
                                                arr.iter()
                                                    .filter_map(|x| {
                                                        let label = x.get("label").and_then(|s| s.as_str()).unwrap_or("").to_string();
                                                        let regexp = x.get("regexp").and_then(|s| s.as_str()).unwrap_or("").to_string();
                                                        if !label.is_empty() && !regexp.is_empty() {
                                                            Some(crate::widgets::watchdog::WatchdogStatSpec { label, regexp })
                                                        } else { None }
                                                    })
                                                    .collect()
                                            })
                                            .unwrap_or_default(),
                                        external_check_cmd: val
                                            .get("external_check_cmd")
                                            .and_then(|v| v.as_str())
                                            .map(String::from),
                                        external_kill_cmd: val
                                            .get("external_kill_cmd")
                                            .and_then(|v| v.as_str())
                                            .map(String::from),
                                    };
                                    // Use the child key to uniquely identify the session
                                    let sess_key = key.clone();
                                    let (session, reused) = if let Some(s) =
                                        state.watchdog_sessions.get(&sess_key).cloned()
                                    {
                                        (s, true)
                                    } else {
                                        let s = crate::widgets::watchdog::WatchdogSession::create(
                                            cmds.clone(),
                                            crate::widgets::watchdog::WatchdogConfig {
                                                stats: cfg.stats.clone(),
                                                ..cfg
                                            },
                                        );
                                        state.watchdog_sessions.insert(sess_key.clone(), s.clone());
                                        (s, false)
                                    };
                                    if reused {
                                        state.dbg(format!(
                                            "watchdog: reusing session for {sess_key}"
                                        ));
                                    } else {
                                        state.dbg(format!(
                                            "watchdog: creating session for {sess_key}"
                                        ));
                                    }
                                    super::ui::pane_b_replace_with_widget(
                                        state,
                                        Box::new(
                                            crate::widgets::watchdog::WatchdogWidget::from_session(
                                                title, &session,
                                            ),
                                        ),
                                        true,
                                    );
                                }
                            }
                        }
                    }
                    _ => {
                        // Unknown widget type, fall back to command or JSON display
                        if let Some(cmd) = val.get("command").and_then(|s| s.as_str()) {
                            if state.view == super::ui::View::Panel {
                                effects.push(Effect::LoadPanelCmd {
                                    pane: super::ui::PanelPane::B,
                                    cmdline: cmd.to_string(),
                                });
                            } else {
                                let title = super::ui::title_from_value(&val);
                                state.status_text = Some(format!("Running: {title}"));
                                state.status_percent = None;
                                effects.push(Effect::RunStream {
                                    cmdline: cmd.to_string(),
                                    title,
                                });
                            }
                        } else {
                            // Show as JSON
                            state.last_error = None;
                            state.last_json_pretty = Some(
                                serde_json::to_string_pretty(&val)
                                    .unwrap_or_else(|_| val.to_string()),
                            );
                            state.json_scroll_y = 0;
                            state.view = super::ui::View::Json;
                        }
                    }
                }
            } else if let Some(cmd) = val
                .get("command")
                .and_then(|s| s.as_str())
                .map(|s| s.to_string())
            {
                if state.view == super::ui::View::Panel {
                    effects.push(Effect::LoadPanelCmd {
                        pane: super::ui::PanelPane::B,
                        cmdline: cmd,
                    });
                } else {
                    let title = super::ui::title_from_value(&val);
                    state.status_text = Some(format!("Running: {title}"));
                    state.status_percent = None;
                    effects.push(Effect::RunStream {
                        cmdline: cmd,
                        title,
                    });
                }
            } else if state.view == super::ui::View::Panel {
                // Non-command leaf selected in panel mode: show in Pane B
                if let Some(ps) = &mut state.panel {
                    ps.b.last_error = None;
                    ps.b.last_json_pretty = Some(
                        serde_json::to_string_pretty(&val).unwrap_or_else(|_| val.to_string()),
                    );
                }
            } else {
                state.last_error = None;
                state.last_json_pretty =
                    Some(serde_json::to_string_pretty(&val).unwrap_or_else(|_| val.to_string()));
                state.json_scroll_y = 0;
                state.view = super::ui::View::Json;
            }
        }
        RefreshMenu(mi) => {
            if super::ui::is_lazy(&mi) || super::ui::is_autoload(&mi) {
                let key = crate::nav::keys::menu_key(&mi);
                state.loading.insert(key.clone());
                state.expanded.insert(key.clone());
                effects.push(Effect::LoadMenu { mi, key });
            } else if let Some(cmd) = mi.command.clone() {
                if state.view == super::ui::View::Panel {
                    effects.push(Effect::LoadPanelCmd {
                        pane: super::ui::PanelPane::B,
                        cmdline: cmd,
                    });
                } else {
                    let run_title = mi.title.clone();
                    state.status_text = Some(format!("Running: {run_title}"));
                    state.status_percent = None;
                    effects.push(Effect::RunStream {
                        cmdline: cmd,
                        title: run_title,
                    });
                }
            } else if super::ui::is_panel(&mi) {
                if let Some(cmd) = mi.pane_a_cmd.clone() {
                    effects.push(Effect::LoadPanelCmd {
                        pane: super::ui::PanelPane::A,
                        cmdline: cmd,
                    });
                }
                if let Some(cmd) = mi.pane_b_cmd.clone() {
                    effects.push(Effect::LoadPanelCmd {
                        pane: super::ui::PanelPane::B,
                        cmdline: cmd,
                    });
                }
                if let Some(path) = mi.pane_a_yaml.clone() {
                    effects.push(Effect::LoadPanelYaml {
                        pane: super::ui::PanelPane::A,
                        path,
                    });
                }
                if let Some(path) = mi.pane_b_yaml.clone() {
                    effects.push(Effect::LoadPanelYaml {
                        pane: super::ui::PanelPane::B,
                        path,
                    });
                }
            }
        }
        RefreshChild { key, val } => {
            if super::ui::is_lazy_value(&val) || super::ui::is_autoload_value(&val) {
                state.loading.insert(key.clone());
                state.expanded.insert(key.clone());
                effects.push(Effect::LoadChild { val, key });
            } else if let Some(cmd) = val
                .get("command")
                .and_then(|s| s.as_str())
                .map(|s| s.to_string())
            {
                let title = super::ui::title_from_value(&val);
                state.status_text = Some(format!("Running: {title}"));
                state.status_percent = None;
                effects.push(Effect::RunStream {
                    cmdline: cmd,
                    title,
                });
            }
        }
        LoadedMenu { key, outcome } => match outcome {
            Ok(LoadOutcome::Items(arr)) => {
                state.dbg(format!("loaded menu {} items", arr.len()));
                state.children.insert(key.clone(), arr);
                state.last_error = None;
                state.last_json_pretty = None;
                state.expanded.insert(key.clone());
                if let Some(children) = state.children.get(&key) {
                    for (ci, val) in children.iter().enumerate() {
                        if super::ui::is_autoload_value(val) && super::ui::auto_expand_value(val) {
                            let ckey = crate::nav::keys::child_key(&key, val, ci);
                            if !state.loading.contains(&ckey) && !state.children.contains_key(&ckey)
                            {
                                state.loading.insert(ckey.clone());
                                state.expanded.insert(ckey.clone());
                                effects.push(Effect::LoadChild {
                                    val: val.clone(),
                                    key: ckey,
                                });
                            }
                        }
                    }
                }
            }
            Ok(LoadOutcome::ItemsWithPagination { items, pagination }) => {
                let cur = pagination
                    .get("current_page")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let tot = pagination
                    .get("total_pages")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                state.dbg(format!(
                    "loaded menu page {}/{} ({} items)",
                    cur,
                    tot,
                    items.len()
                ));
                // Build paginated list with navigation items
                let mut paginated_items = Vec::new();

                // Add "Previous Page" if available
                if let Some(prev_cmd) = pagination.get("prev_page_cmd").and_then(|v| v.as_str()) {
                    let prev_item = serde_json::json!({
                        "id": "__prev_page__",
                        "title": format!("Previous Page ({})",
                            pagination.get("current_page").and_then(|v| v.as_i64()).map(|p| p - 1).unwrap_or(0)),
                        "command": prev_cmd,
                        "__is_pagination": true
                    });
                    paginated_items.push(prev_item);
                }

                // Add actual items
                paginated_items.extend(items);

                // Add "Next Page" if available
                if let Some(next_cmd) = pagination.get("next_page_cmd").and_then(|v| v.as_str()) {
                    let next_item = serde_json::json!({
                        "id": "__next_page__",
                        "title": format!("Next Page ({})",
                            pagination.get("current_page").and_then(|v| v.as_i64()).map(|p| p + 1).unwrap_or(2)),
                        "command": next_cmd,
                        "__is_pagination": true
                    });
                    paginated_items.push(next_item);
                }

                // Add page info at the bottom
                if let (Some(current), Some(total)) = (
                    pagination.get("current_page").and_then(|v| v.as_i64()),
                    pagination.get("total_pages").and_then(|v| v.as_i64()),
                ) {
                    let page_info = serde_json::json!({
                        "id": "__page_info__",
                        "title": format!("─────  Page {}/{} ({} items)  ─────",
                            current, total,
                            pagination.get("total_items").and_then(|v| v.as_i64()).unwrap_or(0)),
                        "__is_info": true
                    });
                    paginated_items.push(page_info);
                }

                state.children.insert(key.clone(), paginated_items);
                state.last_error = None;
                state.last_json_pretty = None;
                state.expanded.insert(key.clone());
            }
            Ok(LoadOutcome::Fallback(v)) => {
                state.last_error = None;
                state.last_json_pretty =
                    Some(serde_json::to_string_pretty(&v).unwrap_or_else(|_| v.to_string()));
            }
            Err(e) => {
                state.dbg(format!("load menu error: {e}"));
                state.last_error = Some(e);
                state.last_json_pretty = None;
            }
        },
        LoadedChild { key, outcome } => match outcome {
            Ok(LoadOutcome::Items(arr)) => {
                state.dbg(format!("loaded child {} items", arr.len()));
                state.children.insert(key.clone(), arr);
                state.last_error = None;
                state.last_json_pretty = None;
                state.expanded.insert(key);
            }
            Ok(LoadOutcome::ItemsWithPagination { items, pagination }) => {
                let cur = pagination
                    .get("current_page")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let tot = pagination
                    .get("total_pages")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                state.dbg(format!(
                    "loaded child page {}/{} ({} items)",
                    cur,
                    tot,
                    items.len()
                ));
                // Build paginated list with navigation items
                let mut paginated_items = Vec::new();

                // Add "Previous Page" if available
                if let Some(prev_cmd) = pagination.get("prev_page_cmd").and_then(|v| v.as_str()) {
                    let prev_item = serde_json::json!({
                        "id": "__prev_page__",
                        "title": format!("Previous Page ({})",
                            pagination.get("current_page").and_then(|v| v.as_i64()).map(|p| p - 1).unwrap_or(0)),
                        "command": prev_cmd,
                        "__is_pagination": true
                    });
                    paginated_items.push(prev_item);
                }

                // Add actual items
                paginated_items.extend(items);

                // Add "Next Page" if available
                if let Some(next_cmd) = pagination.get("next_page_cmd").and_then(|v| v.as_str()) {
                    let next_item = serde_json::json!({
                        "id": "__next_page__",
                        "title": format!("Next Page ({})",
                            pagination.get("current_page").and_then(|v| v.as_i64()).map(|p| p + 1).unwrap_or(2)),
                        "command": next_cmd,
                        "__is_pagination": true
                    });
                    paginated_items.push(next_item);
                }

                // Add page info at the bottom
                if let (Some(current), Some(total)) = (
                    pagination.get("current_page").and_then(|v| v.as_i64()),
                    pagination.get("total_pages").and_then(|v| v.as_i64()),
                ) {
                    let page_info = serde_json::json!({
                        "id": "__page_info__",
                        "title": format!("─────  Page {}/{} ({} items)  ─────",
                            current, total,
                            pagination.get("total_items").and_then(|v| v.as_i64()).unwrap_or(0)),
                        "__is_info": true
                    });
                    paginated_items.push(page_info);
                }

                state.children.insert(key.clone(), paginated_items);
                state.last_error = None;
                state.last_json_pretty = None;
                state.expanded.insert(key);
            }
            Ok(LoadOutcome::Fallback(v)) => {
                state.dbg("loaded fallback JSON".to_string());
                state.last_error = None;
                state.last_json_pretty =
                    Some(serde_json::to_string_pretty(&v).unwrap_or_else(|_| v.to_string()));
            }
            Err(e) => {
                state.dbg(format!("load child error: {e}"));
                state.last_error = Some(e);
                state.last_json_pretty = None;
            }
        },
        LoadedPanel { pane, outcome } => {
            if let (super::ui::PanelPane::B, Some(ps)) = (pane, &state.panel) {
                if let super::ui::PaneContent::Widget(w) = &ps.b_content {
                    if w.as_any()
                        .downcast_ref::<crate::widgets::panel::PanelWidget>()
                        .is_some()
                    {
                        // Ignore outer Pane B loads when a nested panel is active
                        return effects;
                    }
                }
            }
            match outcome {
                Ok(LoadOutcome::Items(vs)) => {
                    // Show result using pretty ResultViewer in Pane B
                    let v = JsonValue::Array(vs);
                    if let Some(ps) = &mut state.panel {
                        match pane {
                            super::ui::PanelPane::A => {
                                ps.a.last_error = None;
                                ps.a.last_json_pretty = Some(
                                    serde_json::to_string_pretty(&v)
                                        .unwrap_or_else(|_| v.to_string()),
                                );
                            }
                            super::ui::PanelPane::B => {
                                ps.b.last_error = None;
                                ps.b.last_json_pretty = Some(
                                    serde_json::to_string_pretty(&v)
                                        .unwrap_or_else(|_| v.to_string()),
                                );
                                let title = state
                                    .pane_b_title
                                    .clone()
                                    .unwrap_or_else(|| "Pane B".to_string());
                                super::ui::pane_b_replace_with_widget(
                                    state,
                                    Box::new(
                                        crate::widgets::result_viewer::ResultViewerWidget::new(
                                            title, v,
                                        ),
                                    ),
                                    true,
                                );
                            }
                        }
                    }
                }
                Ok(LoadOutcome::ItemsWithPagination { items, .. }) => {
                    // Show items using pretty ResultViewer in Pane B
                    let v = JsonValue::Array(items);
                    if let Some(ps) = &mut state.panel {
                        match pane {
                            super::ui::PanelPane::A => {
                                ps.a.last_error = None;
                                ps.a.last_json_pretty = Some(
                                    serde_json::to_string_pretty(&v)
                                        .unwrap_or_else(|_| v.to_string()),
                                );
                            }
                            super::ui::PanelPane::B => {
                                ps.b.last_error = None;
                                ps.b.last_json_pretty = Some(
                                    serde_json::to_string_pretty(&v)
                                        .unwrap_or_else(|_| v.to_string()),
                                );
                                let title = state
                                    .pane_b_title
                                    .clone()
                                    .unwrap_or_else(|| "Pane B".to_string());
                                super::ui::pane_b_replace_with_widget(
                                    state,
                                    Box::new(
                                        crate::widgets::result_viewer::ResultViewerWidget::new(
                                            title, v,
                                        ),
                                    ),
                                    true,
                                );
                            }
                        }
                    }
                }
                Ok(LoadOutcome::Fallback(v)) => {
                    // Try to interpret YAML-as-widget spec; if recognized, schedule appropriate loads
                    if let Some(eff) = pane_yaml_effect(pane, &v) {
                        effects.push(eff);
                    } else if apply_pane_loaded_yaml(pane, &v, state) {
                        // handled
                    } else if let Some(ps) = &mut state.panel {
                        match pane {
                            super::ui::PanelPane::A => {
                                ps.a.last_error = None;
                                ps.a.last_json_pretty = Some(
                                    serde_json::to_string_pretty(&v)
                                        .unwrap_or_else(|_| v.to_string()),
                                );
                            }
                            super::ui::PanelPane::B => {
                                ps.b.last_error = None;
                                ps.b.last_json_pretty = Some(
                                    serde_json::to_string_pretty(&v)
                                        .unwrap_or_else(|_| v.to_string()),
                                );
                                let title = state
                                    .pane_b_title
                                    .clone()
                                    .unwrap_or_else(|| "Pane B".to_string());
                                super::ui::pane_b_replace_with_widget(
                                    state,
                                    Box::new(
                                        crate::widgets::result_viewer::ResultViewerWidget::new(
                                            title, v,
                                        ),
                                    ),
                                    true,
                                );
                            }
                        }
                    }
                }
                Err(e) => {
                    if let Some(ps) = &mut state.panel {
                        match pane {
                            super::ui::PanelPane::A => {
                                ps.a.last_error = Some(e);
                                ps.a.last_json_pretty = None;
                            }
                            super::ui::PanelPane::B => {
                                ps.b.last_error = Some(e);
                                ps.b.last_json_pretty = None;
                            }
                        }
                    }
                }
            }
        }
        LoadedNested { subpane, outcome } => {
            // Derive parent menu key (top-level menu item) to key nested watchdog sessions
            let parent_key_opt = {
                let nodes = crate::nav::flatten::flatten_nodes(state);
                if let Some(crate::ui::FlatNode::Menu { idx, depth }) = nodes.get(state.selected) {
                    if *depth == 0 {
                        let mi = &state.config.menu[*idx];
                        Some(crate::nav::keys::menu_key(mi))
                    } else {
                        None
                    }
                } else {
                    None
                }
            };
            if let Some(ps) = &mut state.panel {
                if let super::ui::PaneContent::Widget(ref mut w) = ps.b_content {
                    if let Some(pw) = w
                        .as_any_mut()
                        .downcast_mut::<crate::widgets::panel::PanelWidget>()
                    {
                        match outcome {
                            Ok(LoadOutcome::Items(vs)) => {
                                let v = JsonValue::Array(vs);
                                let txt = serde_json::to_string_pretty(&v)
                                    .unwrap_or_else(|_| v.to_string());
                                pw.set_subpane_text(subpane, txt);
                            }
                            Ok(LoadOutcome::ItemsWithPagination { items, .. }) => {
                                let v = JsonValue::Array(items);
                                let txt = serde_json::to_string_pretty(&v)
                                    .unwrap_or_else(|_| v.to_string());
                                pw.set_subpane_text(subpane, txt);
                            }
                            Ok(LoadOutcome::Fallback(v)) => {
                                // 1) Special-case watchdog: reuse session per nested subpane
                                let is_watchdog_spec = v
                                    .get("type")
                                    .and_then(|s| s.as_str())
                                    .map(|t| t.eq_ignore_ascii_case("watchdog"))
                                    .unwrap_or_else(|| {
                                        v.get("widget")
                                            .and_then(|s| s.as_str())
                                            .map(|t| t.eq_ignore_ascii_case("watchdog"))
                                            .unwrap_or(false)
                                    });
                                if is_watchdog_spec {
                                    let default_title = match subpane {
                                        super::ui::PanelPane::A => {
                                            "Pane B.A — Watchdog".to_string()
                                        }
                                        super::ui::PanelPane::B => {
                                            "Pane B.B — Watchdog".to_string()
                                        }
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
                                    let max_retries =
                                        v.get("max_retries").and_then(|n| n.as_u64()).unwrap_or(0)
                                            as usize;
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
                                        external_check_cmd: v
                                            .get("external_check_cmd")
                                            .and_then(|s| s.as_str())
                                            .map(|s| s.to_string()),
                                        external_kill_cmd: v
                                            .get("external_kill_cmd")
                                            .and_then(|s| s.as_str())
                                            .map(|s| s.to_string()),
                                    };
                                    if let Some(parent_key) = parent_key_opt {
                                        let sess_key = format!("{parent_key}/nested:{subpane:?}");
                                        let (session, _reused) = if let Some(s) =
                                            state.watchdog_sessions.get(&sess_key).cloned()
                                        {
                                            (s, true)
                                        } else {
                                            let s =
                                                crate::widgets::watchdog::WatchdogSession::create(
                                                    cmds.clone(),
                                                    cfg.clone(),
                                                );
                                            state
                                                .watchdog_sessions
                                                .insert(sess_key.clone(), s.clone());
                                            (s, false)
                                        };
                                        // Debug logging removed here to avoid borrowing conflicts while Panel B is mutably borrowed
                                        let ww =
                                            crate::widgets::watchdog::WatchdogWidget::from_session(
                                                title, &session,
                                            );
                                        pw.set_subpane_widget(subpane, Box::new(ww));
                                        // handled watchdog; do not fall back to generic path
                                        return effects;
                                    }
                                } else {
                                    // 2) Other widget types via registry; if not recognized, fall back to text
                                    if let Some(w) =
                                        crate::chi_core::registry::resolve_widget_for_pane(
                                            subpane, &v,
                                        )
                                    {
                                        pw.set_subpane_widget(subpane, w);
                                    } else {
                                        let txt = serde_json::to_string_pretty(&v)
                                            .unwrap_or_else(|_| v.to_string());
                                        pw.set_subpane_text(subpane, txt);
                                    }
                                }
                            }
                            Err(e) => {
                                pw.set_subpane_error(subpane, e);
                            }
                        }
                    }
                }
            }
        }
        LoadedSubmitForm { pane, outcome } => {
            // Clear submitting status
            state.status_text = None;
            state.status_percent = None;
            // If we are in Form view, update inline errors or show result JSON
            match outcome {
                Ok(LoadOutcome::Fallback(v)) => {
                    let is_error = v
                        .get("ok")
                        .and_then(|b| b.as_bool())
                        .map(|b| !b)
                        .unwrap_or_else(|| v.get("type").and_then(|s| s.as_str()) == Some("error"));
                    if is_error {
                        if let Some(ps) = &mut state.panel {
                            if let super::ui::PaneContent::Widget(ref mut w) = ps.b_content {
                                if let Some(fw) =
                                    w.as_any_mut()
                                        .downcast_mut::<crate::widgets::form_widget::FormWidget>()
                                {
                                    let form = &mut fw.form;
                                    let mut any = false;
                                    let details = v.get("data").and_then(|d| d.get("details"));
                                    if let Some(errs) = details
                                        .and_then(|d| d.get("errors"))
                                        .and_then(|x| x.as_array())
                                    {
                                        for e in errs {
                                            let loc_arr = e.get("loc").and_then(|x| x.as_array());
                                            let last_name = loc_arr.and_then(|a| {
                                                a.iter().rev().find_map(|v| v.as_str())
                                            });
                                            let msg = e
                                                .get("msg")
                                                .and_then(|s| s.as_str())
                                                .unwrap_or("Invalid value");
                                            if let Some(name) = last_name {
                                                if let Some(ff) =
                                                    form.fields.iter_mut().find(|f| f.name == name)
                                                {
                                                    ff.error = Some(msg.to_string());
                                                    any = true;
                                                }
                                            }
                                        }
                                    }
                                    form.disabled = false;
                                    form.message = Some(if any {
                                        "Please fix the highlighted errors".into()
                                    } else {
                                        v.get("data")
                                            .and_then(|d| d.get("message"))
                                            .and_then(|s| s.as_str())
                                            .unwrap_or("Submit failed")
                                            .to_string()
                                    });
                                }
                            }
                        }
                        // Toast error message
                        let msg = v
                            .get("data")
                            .and_then(|d| d.get("message"))
                            .and_then(|s| s.as_str())
                            .unwrap_or("Submit failed")
                            .to_string();
                        return vec![Effect::ShowToast {
                            text: msg,
                            level: crate::ui::ToastLevel::Error,
                            seconds: 3,
                        }];
                    } else {
                        // Show result in Pane B using pretty ResultViewer
                        if let Some(ps) = &mut state.panel {
                            match pane {
                                super::ui::PanelPane::A => {
                                    ps.a.last_error = None;
                                    ps.a.last_json_pretty = Some(
                                        serde_json::to_string_pretty(&v)
                                            .unwrap_or_else(|_| v.to_string()),
                                    );
                                }
                                super::ui::PanelPane::B => {
                                    ps.b.last_error = None;
                                    ps.b.last_json_pretty = Some(
                                        serde_json::to_string_pretty(&v)
                                            .unwrap_or_else(|_| v.to_string()),
                                    );
                                    super::ui::pane_b_replace_with_widget(
                                        state,
                                        Box::new(
                                            crate::widgets::result_viewer::ResultViewerWidget::new(
                                                "Pane B", v,
                                            ),
                                        ),
                                        true,
                                    );
                                }
                            }
                        }
                        // Toast success when submitting into Pane B
                        if matches!(pane, super::ui::PanelPane::B) {
                            return vec![Effect::ShowToast {
                                text: "Saved".into(),
                                level: crate::ui::ToastLevel::Success,
                                seconds: 2,
                            }];
                        }
                    }
                }
                Ok(LoadOutcome::Items(vs)) => {
                    // Unlikely for submit; still render nicely
                    let v = serde_json::Value::Array(vs);
                    if let Some(ps) = &mut state.panel {
                        match pane {
                            super::ui::PanelPane::A => {
                                ps.a.last_json_pretty = Some(
                                    serde_json::to_string_pretty(&v)
                                        .unwrap_or_else(|_| v.to_string()),
                                )
                            }
                            super::ui::PanelPane::B => {
                                ps.b.last_json_pretty = Some(
                                    serde_json::to_string_pretty(&v)
                                        .unwrap_or_else(|_| v.to_string()),
                                );
                                super::ui::pane_b_replace_with_widget(
                                    state,
                                    Box::new(
                                        crate::widgets::result_viewer::ResultViewerWidget::new(
                                            "Pane B", v,
                                        ),
                                    ),
                                    true,
                                );
                            }
                        }
                    }
                }
                Ok(LoadOutcome::ItemsWithPagination { items, .. }) => {
                    // Unlikely for submit; still render nicely
                    let v = serde_json::Value::Array(items);
                    if let Some(ps) = &mut state.panel {
                        match pane {
                            super::ui::PanelPane::A => {
                                ps.a.last_json_pretty = Some(
                                    serde_json::to_string_pretty(&v)
                                        .unwrap_or_else(|_| v.to_string()),
                                )
                            }
                            super::ui::PanelPane::B => {
                                ps.b.last_json_pretty = Some(
                                    serde_json::to_string_pretty(&v)
                                        .unwrap_or_else(|_| v.to_string()),
                                );
                                super::ui::pane_b_replace_with_widget(
                                    state,
                                    Box::new(
                                        crate::widgets::result_viewer::ResultViewerWidget::new(
                                            "Pane B", v,
                                        ),
                                    ),
                                    true,
                                );
                            }
                        }
                    }
                }
                Err(e) => {
                    if let Some(ps) = &mut state.panel {
                        if let super::ui::PaneContent::Widget(ref mut w) = ps.b_content {
                            if let Some(fw) = w
                                .as_any_mut()
                                .downcast_mut::<crate::widgets::form_widget::FormWidget>()
                            {
                                fw.form.disabled = false;
                                fw.form.message = Some(e);
                            }
                        }
                    }
                    return vec![Effect::ShowToast {
                        text: "Submit failed".into(),
                        level: crate::ui::ToastLevel::Error,
                        seconds: 3,
                    }];
                }
            }
        }
        LoadedFormOptions { key, outcome } => {
            // Clear any transient status like "Refreshing options"
            state.status_text = None;
            state.status_percent = None;
            // key format: "form:opt:<field_name>"
            let field_name = key.strip_prefix("form:opt:").unwrap_or(&key).to_string();
            match outcome {
                Ok(LoadOutcome::Fallback(v)) => {
                    let opts = v
                        .get("options")
                        .and_then(|x| x.as_array())
                        .cloned()
                        .unwrap_or_default();
                    let mut labels: Vec<String> = Vec::new();
                    let mut values: Vec<String> = Vec::new();
                    for o in opts {
                        if let Some(obj) = o.as_object() {
                            let label = obj
                                .get("label")
                                .and_then(|s| s.as_str())
                                .unwrap_or("")
                                .to_string();
                            let value = obj
                                .get("value")
                                .and_then(|s| s.as_str())
                                .unwrap_or("")
                                .to_string();
                            if !value.is_empty() {
                                labels.push(if label.is_empty() {
                                    value.clone()
                                } else {
                                    label
                                });
                                values.push(value);
                            }
                        }
                    }
                    if let Some(ps) = &mut state.panel {
                        if let super::ui::PaneContent::Widget(ref mut w) = ps.b_content {
                            if let Some(fw) = w
                                .as_any_mut()
                                .downcast_mut::<crate::widgets::form_widget::FormWidget>()
                            {
                                if let Some(fld) =
                                    fw.form.fields.iter_mut().find(|f| f.name == field_name)
                                {
                                    match &mut fld.kind {
                                        crate::widgets::form::FieldKind::Select {
                                            options,
                                            values: vals,
                                            cursor,
                                            selected,
                                            offset,
                                        } => {
                                            *options = labels;
                                            *vals = values;
                                            *cursor = 0;
                                            *selected = 0;
                                            *offset = 0;
                                            fld.error = None;
                                            fld.dyn_loaded = true;
                                            fld.dyn_loaded_at = Some(Instant::now());
                                        }
                                        crate::widgets::form::FieldKind::MultiSelect {
                                            options,
                                            values: vals,
                                            cursor,
                                            selected,
                                            offset,
                                        } => {
                                            *options = labels.clone();
                                            *vals = values;
                                            *cursor = 0;
                                            *offset = 0;
                                            *selected = vec![false; options.len()];
                                            fld.error = None;
                                            fld.dyn_loaded = true;
                                            fld.dyn_loaded_at = Some(Instant::now());
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }
                }
                Ok(LoadOutcome::Items(_)) | Ok(LoadOutcome::ItemsWithPagination { .. }) => {
                    // Not used for form options; ignore
                }
                Err(e) => {
                    if let Some(ps) = &mut state.panel {
                        if let super::ui::PaneContent::Widget(ref mut w) = ps.b_content {
                            if let Some(fw) = w
                                .as_any_mut()
                                .downcast_mut::<crate::widgets::form_widget::FormWidget>()
                            {
                                if let Some(fld) =
                                    fw.form.fields.iter_mut().find(|f| f.name == field_name)
                                {
                                    fld.error = Some(e);
                                }
                            }
                        }
                    }
                }
            }
        }
        StreamProgress { text, percent } => {
            state.status_text = text;
            state.status_percent = percent;
            // Restart animation when progress starts
            if state.animations_enabled {
                state.animation_start_tick = state.tick;
            }
        }
        StreamDone { result, err } => {
            state.status_text = None;
            state.status_percent = None;
            if let Some(e) = err {
                state.dbg(format!("stream error: {e}"));
                state.last_error = Some(e);
                state.last_json_pretty = None;
                state.json_scroll_y = 0;
                state.view = super::ui::View::Json;
                state.json_viewer = None;
            } else if let Some(v) = result {
                state.dbg("stream done".to_string());
                state.last_error = None;
                state.last_json_pretty =
                    Some(serde_json::to_string_pretty(&v).unwrap_or_else(|_| v.to_string()));
                // Seed pretty JSON viewer for global results
                state.json_viewer = Some(crate::widgets::result_viewer::ResultViewerWidget::new(
                    "JSON Output",
                    v,
                ));
                state.json_scroll_y = 0;
                state.view = super::ui::View::Json;
            }
        }
    }
    effects
}

fn pane_yaml_effect(pane: super::ui::PanelPane, v: &JsonValue) -> Option<Effect> {
    // Route through the widget registry for known specs
    if let Some(eff) = crate::chi_core::registry::resolve_widget_effect(pane, v) {
        return Some(eff);
    }
    // Future: support custom widget specs here
    None
}

fn apply_pane_loaded_yaml(pane: super::ui::PanelPane, v: &JsonValue, state: &mut AppState) -> bool {
    // Special-case: Watchdog spec inside Panel YAML should reuse an existing session
    // based on the current top-level menu key so that closing and re-entering the
    // panel re-attaches instead of restarting the processes.
    if matches!(pane, super::ui::PanelPane::B) {
        let is_watchdog_spec = v
            .get("type")
            .and_then(|s| s.as_str())
            .map(|t| t.eq_ignore_ascii_case("watchdog"))
            .unwrap_or_else(|| {
                v.get("widget")
                    .and_then(|s| s.as_str())
                    .map(|t| t.eq_ignore_ascii_case("watchdog"))
                    .unwrap_or(false)
            });
        if is_watchdog_spec {
            if let Some(_ps) = &mut state.panel {
                // Build title + config (align with registry logic)
                let default_title = "Pane B — Watchdog".to_string();
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
                let max_retries =
                    v.get("max_retries").and_then(|n| n.as_u64()).unwrap_or(0) as usize;
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
                                    Some(crate::widgets::watchdog::WatchdogStatSpec {
                                        label,
                                        regexp,
                                    })
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
                    external_check_cmd: v
                        .get("external_check_cmd")
                        .and_then(|s| s.as_str())
                        .map(|s| s.to_string()),
                    external_kill_cmd: v
                        .get("external_kill_cmd")
                        .and_then(|s| s.as_str())
                        .map(|s| s.to_string()),
                };

                // Determine parent menu key for session reuse
                let parent_key_opt = {
                    let nodes = crate::nav::flatten::flatten_nodes(state);
                    if let Some(crate::ui::FlatNode::Menu { idx, depth }) =
                        nodes.get(state.selected)
                    {
                        if *depth == 0 {
                            let mi = &state.config.menu[*idx];
                            Some(crate::nav::keys::menu_key(mi))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                };
                if let Some(parent_key) = parent_key_opt {
                    // Reuse if present; else create and register
                    let (session, reused) =
                        if let Some(s) = state.watchdog_sessions.get(&parent_key).cloned() {
                            (s, true)
                        } else {
                            let s = crate::widgets::watchdog::WatchdogSession::create(
                                cmds.clone(),
                                cfg.clone(),
                            );
                            state
                                .watchdog_sessions
                                .insert(parent_key.clone(), s.clone());
                            (s, false)
                        };
                    if reused {
                        state.dbg(format!("watchdog(panel): reusing session for {parent_key}"));
                    } else {
                        state.dbg(format!(
                            "watchdog(panel): creating session for {parent_key}"
                        ));
                    }
                    // Attach widget to the session so processes are not restarted
                    super::ui::pane_b_replace_with_widget(
                        state,
                        Box::new(crate::widgets::watchdog::WatchdogWidget::from_session(
                            title, &session,
                        )),
                        true,
                    );
                    return true;
                }
            }
        }
    }

    // Prefer registry-based widget resolution (menu/json_viewer), then fall back.
    if let Some(_ps) = &mut state.panel {
        if let Some(w) = crate::chi_core::registry::resolve_widget_for_pane(pane, v) {
            // If the resolved widget is a Watchdog, register its session under the
            // current top-level menu key so the left menu can show "running...".
            // This also enables re-attachment semantics in the future.
            if let Some(ww) = w
                .as_any()
                .downcast_ref::<crate::widgets::watchdog::WatchdogWidget>()
            {
                // Compute the parent menu key from current selection (top-level Menu node)
                let parent_key_opt = {
                    let nodes = crate::nav::flatten::flatten_nodes(state);
                    if let Some(crate::ui::FlatNode::Menu { idx, .. }) = nodes.get(state.selected) {
                        let mi = &state.config.menu[*idx];
                        Some(crate::nav::keys::menu_key(mi))
                    } else {
                        None
                    }
                };
                if let Some(parent_key) = parent_key_opt {
                    let sess = ww.session_ref();
                    // Keep existing session if any; otherwise register
                    state
                        .watchdog_sessions
                        .entry(parent_key)
                        .or_insert_with(|| sess);
                }
            }
            if let super::ui::PanelPane::B = pane {
                super::ui::pane_b_replace_with_widget(state, w, true);
            }
            return true;
        }
    }
    // 1) Try to interpret as AppConfig menu
    if let Ok(cfg) = serde_json::from_value::<crate::model::AppConfig>(v.clone()) {
        if state.panel.is_some() && matches!(pane, super::ui::PanelPane::B) {
            super::ui::pane_b_replace_with_widget(
                state,
                Box::new(crate::widgets::menu::MenuWidget::from_config(
                    "Pane B — Menu",
                    cfg,
                )),
                true,
            );
        }
        return true;
    }
    // 2) Try to interpret as nested panel spec
    if v.get("type")
        .and_then(|s| s.as_str())
        .map(|s| s.eq_ignore_ascii_case("panel"))
        .unwrap_or(false)
    {
        // Build nested PanelState
        let layout = v.get("layout").and_then(|s| s.as_str());
        let ratio = v.get("size").and_then(|s| s.as_str());
        let mut nested = super::ui::PanelState {
            layout: super::ui::parse_panel_layout(layout),
            ratio: super::ui::parse_panel_ratio(ratio),
            ..Default::default()
        };

        // Helper to load immediate content for sub-pane (sync; minimal MVP)
        let load_into = |sub: &serde_json::Value, target: &mut super::ui::PaneData| {
            let mut txt_lines = String::new();
            if let Some(cmd) = sub.get("cmd").and_then(|s| s.as_str()) {
                match crate::services::cli_runner::run_cmdline_to_json(cmd) {
                    Ok(j) => {
                        txt_lines =
                            serde_json::to_string_pretty(&j).unwrap_or_else(|_| j.to_string());
                    }
                    Err(e) => {
                        target.last_error = Some(format!("{e}"));
                    }
                }
            } else if let Some(path) = sub.get("yaml").and_then(|s| s.as_str()) {
                let pb = std::path::PathBuf::from(path);
                let full_path = if pb.is_absolute() {
                    pb
                } else {
                    let mut base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
                    base.push(path);
                    base
                };
                match std::fs::read_to_string(&full_path) {
                    Ok(s) => match serde_yaml::from_str::<serde_json::Value>(&s) {
                        Ok(j) => {
                            txt_lines =
                                serde_json::to_string_pretty(&j).unwrap_or_else(|_| j.to_string());
                        }
                        Err(e) => {
                            target.last_error = Some(format!("{e}"));
                        }
                    },
                    Err(e) => {
                        target.last_error = Some(format!("{e}"));
                    }
                }
            }
            if !txt_lines.is_empty() {
                target.last_error = None;
                target.last_json_pretty = Some(txt_lines);
            }
        };
        if let Some(a) = v.get("a").and_then(|x| x.as_object()) {
            load_into(&JsonValue::Object(a.clone()), &mut nested.a);
        }
        if let Some(b) = v.get("b").and_then(|x| x.as_object()) {
            load_into(&JsonValue::Object(b.clone()), &mut nested.b);
        }
        if state.panel.is_some() && matches!(pane, super::ui::PanelPane::B) {
            super::ui::pane_b_replace_with_widget(
                state,
                Box::new(crate::widgets::panel::PanelWidget::from_panel_state(nested)),
                true,
            );
            // After placing the nested panel, try to resolve inline specs for subpanes
            // Pre-compute parent menu key (top-level) to key nested watchdog sessions
            let parent_key_opt = {
                let nodes = crate::nav::flatten::flatten_nodes(state);
                if let Some(crate::ui::FlatNode::Menu { idx, depth }) = nodes.get(state.selected) {
                    if *depth == 0 {
                        let mi = &state.config.menu[*idx];
                        Some(crate::nav::keys::menu_key(mi))
                    } else {
                        None
                    }
                } else {
                    None
                }
            };
            if let Some(ps) = &mut state.panel {
                if let super::ui::PaneContent::Widget(ref mut w) = ps.b_content {
                    if let Some(pw) = w
                        .as_any_mut()
                        .downcast_mut::<crate::widgets::panel::PanelWidget>()
                    {
                        // helper to resolve and set a subpane widget
                        let mut resolve_sub =
                            |subpane: super::ui::PanelPane, spec: &serde_json::Value| {
                                let is_watchdog = spec
                                    .get("type")
                                    .and_then(|s| s.as_str())
                                    .map(|t| t.eq_ignore_ascii_case("watchdog"))
                                    .unwrap_or_else(|| {
                                        spec.get("widget")
                                            .and_then(|s| s.as_str())
                                            .map(|t| t.eq_ignore_ascii_case("watchdog"))
                                            .unwrap_or(false)
                                    });
                                if is_watchdog {
                                    if let Some(parent_key) = &parent_key_opt {
                                        let sess_key = format!("{parent_key}/nested:{subpane:?}");
                                        let cmds: Vec<String> = spec
                                            .get("commands")
                                            .and_then(|a| a.as_array())
                                            .map(|arr| {
                                                arr.iter()
                                                    .filter_map(|x| {
                                                        x.as_str().map(|s| s.to_string())
                                                    })
                                                    .collect()
                                            })
                                            .unwrap_or_default();
                                        let sequential = spec
                                            .get("sequential")
                                            .and_then(|b| b.as_bool())
                                            .unwrap_or(false);
                                        let auto_restart = spec
                                            .get("auto_restart")
                                            .and_then(|b| b.as_bool())
                                            .unwrap_or(false);
                                        let max_retries = spec
                                            .get("max_retries")
                                            .and_then(|n| n.as_u64())
                                            .unwrap_or(0)
                                            as usize;
                                        let restart_delay_ms = spec
                                            .get("restart_delay_ms")
                                            .and_then(|n| n.as_u64())
                                            .unwrap_or(1000);
                                        let stop_on_failure = spec
                                            .get("stop_on_failure")
                                            .and_then(|b| b.as_bool())
                                            .unwrap_or(false);
                                        let allowed_exit_codes: Vec<i32> = spec
                                            .get("allowed_exit_codes")
                                            .and_then(|a| a.as_array())
                                            .map(|arr| {
                                                arr.iter()
                                                    .filter_map(|x| x.as_i64().map(|i| i as i32))
                                                    .collect()
                                            })
                                            .unwrap_or_else(|| vec![0]);
                                        let on_panic_exit_cmd = spec
                                            .get("on_panic_exit_cmd")
                                            .and_then(|s| s.as_str())
                                            .map(|s| s.to_string());
                                        let stats = spec
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
                                            external_check_cmd: v
                                                .get("external_check_cmd")
                                                .and_then(|s| s.as_str())
                                                .map(|s| s.to_string()),
                                            external_kill_cmd: v
                                                .get("external_kill_cmd")
                                                .and_then(|s| s.as_str())
                                                .map(|s| s.to_string()),
                                        };
                                        let (session, _reused) = if let Some(s) =
                                            state.watchdog_sessions.get(&sess_key).cloned()
                                        {
                                            (s, true)
                                        } else {
                                            let s =
                                                crate::widgets::watchdog::WatchdogSession::create(
                                                    cmds.clone(),
                                                    cfg.clone(),
                                                );
                                            state
                                                .watchdog_sessions
                                                .insert(sess_key.clone(), s.clone());
                                            (s, false)
                                        };
                                        // Debug logging removed here to avoid borrow conflicts while mutably borrowing panel widget
                                        let title = spec
                                            .get("title")
                                            .and_then(|s| s.as_str())
                                            .unwrap_or(match subpane {
                                                super::ui::PanelPane::A => "Pane B.A — Watchdog",
                                                super::ui::PanelPane::B => "Pane B.B — Watchdog",
                                            })
                                            .to_string();
                                        let ww =
                                            crate::widgets::watchdog::WatchdogWidget::from_session(
                                                title, &session,
                                            );
                                        pw.set_subpane_widget(subpane, Box::new(ww));
                                        return;
                                    }
                                }
                                if let Some(w) = crate::chi_core::registry::resolve_widget_for_pane(
                                    subpane, spec,
                                ) {
                                    pw.set_subpane_widget(subpane, w);
                                }
                            };
                        if let Some(a) = v.get("a") {
                            resolve_sub(super::ui::PanelPane::A, a);
                        }
                        if let Some(b) = v.get("b") {
                            resolve_sub(super::ui::PanelPane::B, b);
                        }
                    }
                }
            }
            // UX: when nested panel appears, ensure focus is on B
            state.panel_focus = super::ui::PanelPane::B;
        }
        return true;
    }
    // 3) Try to interpret as simple form spec
    if v.get("type")
        .and_then(|s| s.as_str())
        .map(|s| s.eq_ignore_ascii_case("form"))
        .unwrap_or(false)
    {
        // Basic YAML validation for common mistakes
        if let Err(e) = validate_form_yaml(v) {
            if let Some(ps) = &mut state.panel {
                if let super::ui::PanelPane::B = pane {
                    ps.b.last_error = Some(e);
                    ps.b.last_json_pretty = None;
                }
            }
            return true;
        }
        let title = v
            .get("title")
            .and_then(|s| s.as_str())
            .unwrap_or("Form")
            .to_string();
        // Detect command for submit
        // Priority: submit_cmd | submit.command | command
        let submit_cmd = v
            .get("submit_cmd")
            .and_then(|s| s.as_str())
            .map(|s| s.to_string())
            .or_else(|| {
                v.get("submit")
                    .and_then(|x| x.get("command"))
                    .and_then(|s| s.as_str())
                    .map(|s| s.to_string())
            })
            .or_else(|| {
                v.get("command")
                    .and_then(|s| s.as_str())
                    .map(|s| s.to_string())
            });
        let mut form = crate::widgets::form::FormState {
            title,
            submit_cmd,
            ..Default::default()
        };
        if let Some(fields) = v.get("fields").and_then(|x| x.as_array()) {
            for f in fields {
                if let Some(name) = f.get("name").and_then(|s| s.as_str()) {
                    let label = f
                        .get("label")
                        .and_then(|s| s.as_str())
                        .unwrap_or(name)
                        .to_string();
                    let required = f.get("required").and_then(|b| b.as_bool()).unwrap_or(false);
                    let t = f
                        .get("type")
                        .and_then(|s| s.as_str())
                        .unwrap_or("text")
                        .to_ascii_lowercase();
                    let kind = match t.as_str() {
                        "checkbox" | "bool" | "boolean" => {
                            crate::widgets::form::FieldKind::Checkbox
                        }
                        "select" => {
                            if let Some(opts_arr) = f.get("options").and_then(|x| x.as_array()) {
                                let opts: Vec<String> = opts_arr
                                    .iter()
                                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                    .collect();
                                crate::widgets::form::FieldKind::Select {
                                    options: opts.clone(),
                                    values: opts,
                                    cursor: 0,
                                    selected: 0,
                                    offset: 0,
                                }
                            } else {
                                // dynamic options via options_cmd
                                crate::widgets::form::FieldKind::Select {
                                    options: vec![],
                                    values: vec![],
                                    cursor: 0,
                                    selected: 0,
                                    offset: 0,
                                }
                            }
                        }
                        "multiselect" | "multi-select" => {
                            if let Some(opts_arr) = f.get("options").and_then(|x| x.as_array()) {
                                let opts: Vec<String> = opts_arr
                                    .iter()
                                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                    .collect();
                                let selected = vec![false; opts.len()];
                                crate::widgets::form::FieldKind::MultiSelect {
                                    options: opts.clone(),
                                    values: opts,
                                    cursor: 0,
                                    selected,
                                    offset: 0,
                                }
                            } else {
                                crate::widgets::form::FieldKind::MultiSelect {
                                    options: vec![],
                                    values: vec![],
                                    cursor: 0,
                                    selected: vec![],
                                    offset: 0,
                                }
                            }
                        }
                        "password" => crate::widgets::form::FieldKind::Password,
                        "textarea" => crate::widgets::form::FieldKind::TextArea {
                            edit_lines: 6,
                            offset: 0,
                        },
                        _ => crate::widgets::form::FieldKind::Text,
                    };
                    let value = match kind {
                        crate::widgets::form::FieldKind::Checkbox => {
                            let b = f.get("default").and_then(|x| x.as_bool()).unwrap_or(false);
                            crate::widgets::form::FieldValue::Bool(b)
                        }
                        crate::widgets::form::FieldKind::Number { .. } => {
                            let s = if let Some(v) = f.get("default").and_then(|x| x.as_i64()) {
                                v.to_string()
                            } else if let Some(v) = f.get("default").and_then(|x| x.as_f64()) {
                                if v.fract().abs() < 1e-12 {
                                    format!("{v:.0}")
                                } else {
                                    v.to_string()
                                }
                            } else {
                                f.get("default")
                                    .and_then(|x| x.as_str())
                                    .unwrap_or("")
                                    .to_string()
                            };
                            crate::widgets::form::FieldValue::Text(s)
                        }
                        crate::widgets::form::FieldKind::Array { .. } => {
                            let s = if let Some(arr) = f.get("default").and_then(|x| x.as_array()) {
                                let mut parts = Vec::new();
                                for v in arr {
                                    if let Some(t) = v.as_str() {
                                        parts.push(t.to_string());
                                    } else if let Some(i) = v.as_i64() {
                                        parts.push(i.to_string());
                                    } else if let Some(fl) = v.as_f64() {
                                        parts.push(if fl.fract().abs() < 1e-12 {
                                            format!("{fl:.0}")
                                        } else {
                                            fl.to_string()
                                        });
                                    }
                                }
                                parts.join(", ")
                            } else {
                                f.get("default")
                                    .and_then(|x| x.as_str())
                                    .unwrap_or("")
                                    .to_string()
                            };
                            crate::widgets::form::FieldValue::Text(s)
                        }
                        crate::widgets::form::FieldKind::Text
                        | crate::widgets::form::FieldKind::Password
                        | crate::widgets::form::FieldKind::TextArea { .. }
                        | crate::widgets::form::FieldKind::Select { .. }
                        | crate::widgets::form::FieldKind::MultiSelect { .. } => {
                            let s = f
                                .get("default")
                                .and_then(|x| x.as_str())
                                .unwrap_or("")
                                .to_string();
                            crate::widgets::form::FieldValue::Text(s)
                        }
                    };
                    let mut ff = crate::widgets::form::FormField {
                        name: name.to_string(),
                        label,
                        required,
                        kind,
                        value,
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
                    if let Some(cmd) = f.get("options_cmd").and_then(|s| s.as_str()) {
                        ff.dyn_options_cmd = Some(cmd.to_string());
                        ff.dyn_unwrap = f
                            .get("unwrap")
                            .and_then(|s| s.as_str())
                            .map(|s| s.to_string());
                    }
                    if let Some(maxl) = f.get("max_lines").and_then(|x| x.as_u64()) {
                        ff.textarea_max_lines = Some(maxl as usize);
                    }
                    form.fields.push(ff);
                }
            }
        } else if let Some(groups) = v.get("groups").and_then(|x| x.as_array()) {
            // Support grouped fields
            for g in groups {
                let group_title = g
                    .get("title")
                    .and_then(|s| s.as_str())
                    .unwrap_or("")
                    .to_string();
                if let Some(fields) = g.get("fields").and_then(|x| x.as_array()) {
                    for f in fields {
                        if let Some(name) = f.get("name").and_then(|s| s.as_str()) {
                            let label = f
                                .get("label")
                                .and_then(|s| s.as_str())
                                .unwrap_or(name)
                                .to_string();
                            let required =
                                f.get("required").and_then(|b| b.as_bool()).unwrap_or(false);
                            let t = f
                                .get("type")
                                .and_then(|s| s.as_str())
                                .unwrap_or("text")
                                .to_ascii_lowercase();
                            let kind = match t.as_str() {
                                "checkbox" | "bool" | "boolean" => {
                                    crate::widgets::form::FieldKind::Checkbox
                                }
                                "select" => {
                                    if let Some(opts_arr) =
                                        f.get("options").and_then(|x| x.as_array())
                                    {
                                        let opts: Vec<String> = opts_arr
                                            .iter()
                                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                            .collect();
                                        crate::widgets::form::FieldKind::Select {
                                            options: opts.clone(),
                                            values: opts,
                                            cursor: 0,
                                            selected: 0,
                                            offset: 0,
                                        }
                                    } else {
                                        crate::widgets::form::FieldKind::Select {
                                            options: vec![],
                                            values: vec![],
                                            cursor: 0,
                                            selected: 0,
                                            offset: 0,
                                        }
                                    }
                                }
                                "multiselect" | "multi-select" => {
                                    if let Some(opts_arr) =
                                        f.get("options").and_then(|x| x.as_array())
                                    {
                                        let opts: Vec<String> = opts_arr
                                            .iter()
                                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                            .collect();
                                        let selected = vec![false; opts.len()];
                                        crate::widgets::form::FieldKind::MultiSelect {
                                            options: opts.clone(),
                                            values: opts,
                                            cursor: 0,
                                            selected,
                                            offset: 0,
                                        }
                                    } else {
                                        crate::widgets::form::FieldKind::MultiSelect {
                                            options: vec![],
                                            values: vec![],
                                            cursor: 0,
                                            selected: vec![],
                                            offset: 0,
                                        }
                                    }
                                }
                                "password" => crate::widgets::form::FieldKind::Password,
                                "textarea" => crate::widgets::form::FieldKind::TextArea {
                                    edit_lines: 6,
                                    offset: 0,
                                },
                                _ => crate::widgets::form::FieldKind::Text,
                            };
                            let value = match kind {
                                crate::widgets::form::FieldKind::Checkbox => {
                                    let b =
                                        f.get("default").and_then(|x| x.as_bool()).unwrap_or(false);
                                    crate::widgets::form::FieldValue::Bool(b)
                                }
                                crate::widgets::form::FieldKind::Number { .. } => {
                                    let s = if let Some(v) =
                                        f.get("default").and_then(|x| x.as_i64())
                                    {
                                        v.to_string()
                                    } else if let Some(v) =
                                        f.get("default").and_then(|x| x.as_f64())
                                    {
                                        if v.fract().abs() < 1e-12 {
                                            format!("{v:.0}")
                                        } else {
                                            v.to_string()
                                        }
                                    } else {
                                        f.get("default")
                                            .and_then(|x| x.as_str())
                                            .unwrap_or("")
                                            .to_string()
                                    };
                                    crate::widgets::form::FieldValue::Text(s)
                                }
                                crate::widgets::form::FieldKind::Array { .. } => {
                                    let s = if let Some(arr) =
                                        f.get("default").and_then(|x| x.as_array())
                                    {
                                        let mut parts = Vec::new();
                                        for v in arr {
                                            if let Some(t) = v.as_str() {
                                                parts.push(t.to_string());
                                            } else if let Some(i) = v.as_i64() {
                                                parts.push(i.to_string());
                                            } else if let Some(fl) = v.as_f64() {
                                                parts.push(if fl.fract().abs() < 1e-12 {
                                                    format!("{fl:.0}")
                                                } else {
                                                    fl.to_string()
                                                });
                                            }
                                        }
                                        parts.join(", ")
                                    } else {
                                        f.get("default")
                                            .and_then(|x| x.as_str())
                                            .unwrap_or("")
                                            .to_string()
                                    };
                                    crate::widgets::form::FieldValue::Text(s)
                                }
                                _ => {
                                    let s = f
                                        .get("default")
                                        .and_then(|x| x.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    crate::widgets::form::FieldValue::Text(s)
                                }
                            };
                            let mut ff = crate::widgets::form::FormField {
                                name: name.to_string(),
                                label,
                                required,
                                kind,
                                value,
                                error: None,
                                text_min_len: None,
                                text_max_len: None,
                                text_pattern: None,
                                textarea_max_lines: None,
                                dyn_options_cmd: None,
                                dyn_unwrap: None,
                                dyn_loaded: false,
                                dyn_loaded_at: None,
                                group: if group_title.is_empty() {
                                    None
                                } else {
                                    Some(group_title.clone())
                                },
                                order: None,
                            };
                            if let Some(cmd) = f.get("options_cmd").and_then(|s| s.as_str()) {
                                ff.dyn_options_cmd = Some(cmd.to_string());
                                ff.dyn_unwrap = f
                                    .get("unwrap")
                                    .and_then(|s| s.as_str())
                                    .map(|s| s.to_string());
                            }
                            if let Some(maxl) = f.get("max_lines").and_then(|x| x.as_u64()) {
                                ff.textarea_max_lines = Some(maxl as usize);
                            }
                            form.fields.push(ff);
                        }
                    }
                }
            }
        }
        // If no fields were defined explicitly, attempt schema-driven mapping from CLI
        if form.fields.is_empty() {
            // Prefer explicit schema_cmd if provided, otherwise attempt to derive from submit_cmd
            if let Some(schema_cmd) = v
                .get("schema_cmd")
                .and_then(|s| s.as_str())
                .map(|s| s.to_string())
            {
                if let Ok(schema_env) =
                    crate::services::cli_runner::run_cmdline_to_json(&schema_cmd)
                {
                    // Try common shapes
                    let data = schema_env.get("data").cloned().unwrap_or(JsonValue::Null);
                    // 1) Direct input_schema
                    if let Some(inp) = data.get("input_schema") {
                        form.fields = crate::widgets::form::fields_from_json_schema(inp);
                    } else if let Some(commands) = data.get("commands").and_then(|x| x.as_array()) {
                        if let Some(first) = commands.first() {
                            if let Some(inp) = first.get("input_schema") {
                                form.fields = crate::widgets::form::fields_from_json_schema(inp);
                            }
                        }
                    }
                }
            } else if let Some(cmdline) = &form.submit_cmd {
                // Parse program and (heuristically) command token
                let parts = shlex::split(cmdline).unwrap_or_default();
                if parts.len() >= 2 {
                    let prog = &parts[0];
                    let cmd_name = &parts[1];
                    let schema_cmd = format!("{prog} schema");
                    if let Ok(schema_env) =
                        crate::services::cli_runner::run_cmdline_to_json(&schema_cmd)
                    {
                        // Expect envelope: { ok, data: { commands: [...] } }
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
                            .find(|c| c.get("name").and_then(|s| s.as_str()) == Some(cmd_name))
                        {
                            if let Some(inp) = spec.get("input_schema") {
                                // Build fields from the input schema via widget helper
                                form.fields = crate::widgets::form::fields_from_json_schema(inp);
                            }
                        }
                    }
                }
            }
        }
        // Apply overrides if present
        if let Some(ov) = v.get("overrides").and_then(|x| x.as_object()) {
            for (fname, o) in ov.iter() {
                if let Some(ff) = form.fields.iter_mut().find(|f| &f.name == fname) {
                    if let Some(lbl) = o.get("label").and_then(|s| s.as_str()) {
                        ff.label = lbl.to_string();
                    }
                    if let Some(req) = o.get("required").and_then(|b| b.as_bool()) {
                        ff.required = req;
                    }
                    if let Some(g) = o.get("group").and_then(|s| s.as_str()) {
                        ff.group = Some(g.to_string());
                    }
                    if let Some(ord) = o.get("order").and_then(|x| x.as_i64()) {
                        ff.order = Some(ord as i32);
                    }
                    if let Some(w) = o.get("widget").and_then(|s| s.as_str()) {
                        match w.to_ascii_lowercase().as_str() {
                            "checkbox" => ff.kind = crate::widgets::form::FieldKind::Checkbox,
                            "select" => {
                                let opts: Vec<String> = o
                                    .get("options")
                                    .and_then(|x| x.as_array())
                                    .map(|a| {
                                        a.iter()
                                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                            .collect()
                                    })
                                    .unwrap_or_default();
                                ff.kind = crate::widgets::form::FieldKind::Select {
                                    options: opts.clone(),
                                    values: opts,
                                    cursor: 0,
                                    selected: 0,
                                    offset: 0,
                                };
                            }
                            "password" => ff.kind = crate::widgets::form::FieldKind::Password,
                            "textarea" => {
                                let edit_lines = o
                                    .get("edit_lines")
                                    .and_then(|x| x.as_u64())
                                    .map(|x| x as usize)
                                    .unwrap_or(6);
                                ff.kind = crate::widgets::form::FieldKind::TextArea {
                                    edit_lines,
                                    offset: 0,
                                };
                            }
                            _ => ff.kind = crate::widgets::form::FieldKind::Text,
                        }
                    }
                    match ff.kind {
                        crate::widgets::form::FieldKind::Checkbox => {
                            if let Some(b) = o.get("default").and_then(|x| x.as_bool()) {
                                ff.value = crate::widgets::form::FieldValue::Bool(b);
                            }
                        }
                        crate::widgets::form::FieldKind::Select {
                            ref options,
                            ref mut selected,
                            ..
                        } => {
                            if let Some(s) = o.get("default").and_then(|x| x.as_str()) {
                                if let Some(idx) = options.iter().position(|v| v == s) {
                                    *selected = idx;
                                }
                                ff.value = crate::widgets::form::FieldValue::Text(s.to_string());
                            }
                        }
                        _ => {
                            if let Some(s) = o.get("default").and_then(|x| x.as_str()) {
                                ff.value = crate::widgets::form::FieldValue::Text(s.to_string());
                            }
                        }
                    }
                    // dynamic options overrides
                    if let Some(cmd) = o.get("options_cmd").and_then(|s| s.as_str()) {
                        ff.dyn_options_cmd = Some(cmd.to_string());
                        ff.dyn_unwrap = o
                            .get("unwrap")
                            .and_then(|s| s.as_str())
                            .map(|s| s.to_string());
                    }
                    if let Some(maxl) = o.get("max_lines").and_then(|x| x.as_u64()) {
                        ff.textarea_max_lines = Some(maxl as usize);
                    }
                }
            }
        }
        // Sort fields by group and order for deterministic layout
        form.fields.sort_by(|a, b| {
            let ga = a.group.as_deref().unwrap_or("");
            let gb = b.group.as_deref().unwrap_or("");
            if ga != gb {
                return ga.cmp(gb);
            }
            let oa = a.order.unwrap_or(i32::MAX);
            let ob = b.order.unwrap_or(i32::MAX);
            oa.cmp(&ob)
        });
        if state.panel.is_some() && matches!(pane, super::ui::PanelPane::B) {
            super::ui::pane_b_replace_with_widget(
                state,
                Box::new(crate::widgets::form_widget::FormWidget::new(form)),
                true,
            );
        }
        return true;
    }
    false
}

fn validate_form_yaml(v: &JsonValue) -> Result<(), String> {
    if !v
        .get("type")
        .and_then(|s| s.as_str())
        .map(|s| s.eq_ignore_ascii_case("form"))
        .unwrap_or(false)
    {
        return Err("type must be 'form'".into());
    }
    if let Some(fields) = v.get("fields") {
        let arr = fields
            .as_array()
            .ok_or_else(|| "fields must be an array".to_string())?;
        for (i, f) in arr.iter().enumerate() {
            let obj = f
                .as_object()
                .ok_or_else(|| format!("fields[{i}] must be an object"))?;
            let name = obj
                .get("name")
                .and_then(|s| s.as_str())
                .ok_or_else(|| format!("fields[{i}]: missing 'name'"))?;
            let t = obj
                .get("type")
                .and_then(|s| s.as_str())
                .unwrap_or("text")
                .to_ascii_lowercase();
            if t == "select" || t == "multiselect" || t == "multi-select" {
                let has_opts = obj.get("options").and_then(|x| x.as_array()).is_some();
                let has_cmd = obj.get("options_cmd").and_then(|s| s.as_str()).is_some();
                if !has_opts && !has_cmd {
                    return Err(format!("fields[{i}] (name='{name}'): select/multiselect requires 'options' or 'options_cmd'"));
                }
            }
        }
    }
    Ok(())
}

// Keep test module at the very end to satisfy clippy::items-after-test-module
#[cfg(test)]
mod tests;
