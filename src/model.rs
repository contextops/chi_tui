use serde::Deserialize;
use serde_json::Value as JsonValue;

#[derive(Debug, Deserialize, Clone, Default)]
pub struct MenuItem {
    pub id: String,
    pub title: String,
    #[serde(default)]
    #[allow(dead_code)]
    pub command: Option<String>,
    #[serde(default)]
    pub widget: Option<String>,
    // Optional custom title for Pane B widget header (non-panel widgets)
    #[serde(default)]
    pub pane_b_title: Option<String>,
    // Markdown: optional path to file (when widget == "markdown")
    #[serde(default)]
    pub path: Option<String>,
    // Markdown: optional inline content (when widget == "markdown")
    #[serde(default)]
    pub content: Option<String>,
    // Watchdog: optional list of commands (when widget == "watchdog")
    #[serde(default)]
    pub commands: Option<Vec<String>>,
    // Watchdog: optional external detection/kill commands
    #[serde(default)]
    pub external_check_cmd: Option<String>,
    #[serde(default)]
    pub external_kill_cmd: Option<String>,
    #[serde(default)]
    pub sequential: Option<bool>,
    #[serde(default)]
    pub auto_restart: Option<bool>,
    #[serde(default)]
    pub max_retries: Option<u32>,
    #[serde(default)]
    pub restart_delay_ms: Option<u32>,
    #[serde(default)]
    pub stop_on_failure: Option<bool>,
    #[serde(default)]
    pub allowed_exit_codes: Option<Vec<i32>>,
    #[serde(default)]
    pub on_panic_exit_cmd: Option<String>,
    #[serde(default)]
    pub unwrap: Option<String>,
    #[serde(default)]
    pub initial_text: Option<String>,
    #[serde(default)]
    pub auto_expand: Option<bool>,
    #[serde(default)]
    pub expand_on_enter: Option<bool>,
    // Force command to run in streaming mode (even inside Panel view)
    #[serde(default)]
    pub stream: Option<bool>,
    // Static hierarchical children (for nested menus)
    #[serde(default)]
    pub children: Option<Vec<JsonValue>>, // children defined inline in YAML
    // Panel widget configuration
    #[serde(default)]
    pub panel_layout: Option<String>, // horizontal|vertical
    #[serde(default)]
    pub panel_size: Option<String>, // "1:1" | "1:3" | "3:1"
    #[serde(default)]
    pub pane_a_cmd: Option<String>,
    #[serde(default)]
    pub pane_b_cmd: Option<String>,
    #[serde(default)]
    pub pane_a_yaml: Option<String>,
    #[serde(default)]
    pub pane_b_yaml: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    pub modal: Option<bool>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct HorizontalMenuItem {
    #[allow(dead_code)]
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub config: Option<String>, // Path to YAML config to load when selected
}

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    #[serde(default)]
    #[allow(dead_code)]
    pub header: Option<String>,
    #[serde(default)]
    pub logo: Option<String>,
    // Optional: auto-enter a menu item by id when this screen loads
    #[serde(default)]
    pub auto_enter: Option<String>,
    // Optional: allow closing panel view with Esc. Default: true.
    #[serde(default = "default_true")]
    pub can_close: bool,
    #[serde(default)]
    pub horizontal_menu: Vec<HorizontalMenuItem>,
    pub menu: Vec<MenuItem>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            header: Some("CHI TUI".to_string()),
            logo: None,
            auto_enter: None,
            can_close: true,
            horizontal_menu: vec![],
            menu: vec![],
        }
    }
}

fn default_true() -> bool {
    true
}

#[allow(dead_code)]
pub(crate) fn validate_app_config(cfg: &AppConfig) -> Result<(), String> {
    use std::collections::HashSet;
    let mut ids = HashSet::new();
    for (i, m) in cfg.menu.iter().enumerate() {
        if !ids.insert(&m.id) {
            return Err(format!("duplicate menu id: '{}' at index {}", m.id, i));
        }
        if let Some(w) = &m.widget {
            match w.as_str() {
                "panel" => {
                    let any = m.pane_a_cmd.is_some()
                        || m.pane_b_cmd.is_some()
                        || m.pane_a_yaml.is_some()
                        || m.pane_b_yaml.is_some();
                    if !any {
                        return Err(format!(
                            "panel '{}' must specify at least one of pane_a/b cmd/yaml",
                            m.id
                        ));
                    }
                    for (which, p) in [
                        ("pane_a_yaml", &m.pane_a_yaml),
                        ("pane_b_yaml", &m.pane_b_yaml),
                    ] {
                        if let Some(path) = p {
                            let pb = std::path::PathBuf::from(path);
                            let full = if pb.is_absolute() {
                                pb
                            } else {
                                let mut base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
                                base.push(path);
                                base
                            };
                            if !full.exists() {
                                return Err(format!(
                                    "panel '{}' {} refers to missing file: {}",
                                    m.id, which, path
                                ));
                            }
                        }
                    }
                }
                "lazy_items" | "autoload_items" => {
                    if m.command.as_deref().unwrap_or("").is_empty() {
                        return Err(format!("menu '{}' requires 'command' for {}", m.id, w));
                    }
                }
                "markdown" => {
                    let has_path = m.path.as_deref().map(|s| !s.is_empty()).unwrap_or(false);
                    let has_content = m.content.as_deref().map(|s| !s.is_empty()).unwrap_or(false);
                    if !(has_path || has_content) {
                        return Err(format!(
                            "menu '{}' requires either 'path' or 'content' for markdown",
                            m.id
                        ));
                    }
                }
                "watchdog" => {
                    let allows_external = m.external_check_cmd.is_some();
                    if m.commands.as_ref().map(|v| v.is_empty()).unwrap_or(true) && !allows_external
                    {
                        return Err(format!(
                            "menu '{}' requires non-empty 'commands' for watchdog",
                            m.id
                        ));
                    }
                    if let Some(v) = &m.max_retries {
                        if *v > 1000 {
                            return Err(format!("menu '{}' watchdog max_retries too large", m.id));
                        }
                    }
                }
                _ => {}
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_detects_duplicate_ids() {
        let cfg = AppConfig {
            header: None,
            menu: vec![
                MenuItem {
                    id: "a".into(),
                    title: "A".into(),
                    ..Default::default()
                },
                MenuItem {
                    id: "a".into(),
                    title: "B".into(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        let err = validate_app_config(&cfg).unwrap_err();
        assert!(err.contains("duplicate menu id"));
    }

    #[test]
    fn validate_panel_requires_content() {
        let cfg = AppConfig {
            header: None,
            menu: vec![MenuItem {
                id: "p".into(),
                title: "P".into(),
                widget: Some("panel".into()),
                ..Default::default()
            }],
            ..Default::default()
        };
        let err = validate_app_config(&cfg).unwrap_err();
        assert!(err.contains("must specify at least one"));
    }
}
