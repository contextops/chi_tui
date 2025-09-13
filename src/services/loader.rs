use crate::model::MenuItem;
use crate::services::cli_runner::run_cmdline_to_json;
use anyhow::{anyhow, Result};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

pub enum Loaded {
    Items(Vec<JsonValue>),
    ItemsWithPagination {
        items: Vec<JsonValue>,
        pagination: JsonValue,
    },
    Fallback(JsonValue),
}

pub fn get_by_path<'a>(v: &'a JsonValue, path: &str) -> Option<&'a JsonValue> {
    let mut cur = v;
    for seg in path.split('.') {
        cur = cur.get(seg)?;
    }
    Some(cur)
}

// Load dynamic select/multiselect options from a CLI command, with optional unwrap
// unwrap formats supported:
// - None: defaults to data.items; array of strings or objects with id/title/name
// - "data.items": same as above
// - "data.items[].id/title": iterate array at data.items and map value from id and label from title
static OPTIONS_CACHE: OnceLock<Mutex<HashMap<String, (Instant, serde_json::Value)>>> =
    OnceLock::new();

fn options_cache() -> &'static Mutex<HashMap<String, (Instant, serde_json::Value)>> {
    OPTIONS_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
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

pub fn spawn_load_options_cmd(
    cmdline: String,
    unwrap: Option<String>,
    key: String,
    force: bool,
    tx: Sender<crate::ui::LoadMsg>,
) {
    thread::spawn(move || {
        let outcome = (|| -> Result<crate::ui::LoadOutcome, String> {
            let cache_key = format!("{}|{}", cmdline, unwrap.clone().unwrap_or_default());
            let ttl = options_ttl();
            // Try cache hit if not forced and TTL enabled
            if !force {
                if let Some(ttl) = ttl {
                    if let Ok(map) = options_cache().lock() {
                        if let Some((ts, v)) = map.get(&cache_key) {
                            if ts.elapsed() <= ttl {
                                return Ok(crate::ui::LoadOutcome::Fallback(v.clone()));
                            }
                        }
                    }
                }
            }
            // Fetch fresh
            let v = run_cmdline_to_json(&cmdline).map_err(|e| format!("{e}"))?;
            let pairs = parse_options_from_json(&v, unwrap.as_deref());
            let label_value_pairs: Vec<serde_json::Value> = pairs
                .into_iter()
                .map(|(l, r)| serde_json::json!({"label": l, "value": r}))
                .collect();
            let out = serde_json::json!({"options": label_value_pairs});
            // Store in cache if TTL enabled
            if ttl.is_some() {
                if let Ok(mut map) = options_cache().lock() {
                    map.insert(cache_key, (Instant::now(), out.clone()));
                }
            }
            Ok(crate::ui::LoadOutcome::Fallback(out))
        })();
        let _ = tx.send(crate::ui::LoadMsg {
            key,
            outcome,
            kind: crate::ui::LoadKind::FormOptions,
        });
    });
}

pub(crate) fn parse_options_from_json(
    v: &JsonValue,
    unwrap: Option<&str>,
) -> Vec<(String, String)> {
    let uw = unwrap.unwrap_or("data.items");
    let mut out: Vec<(String, String)> = Vec::new();
    if let Some(idx) = uw.find("[]") {
        let base = &uw[..idx];
        let rest = uw[idx + 2..].trim_start_matches('.');
        let (val_path, lbl_path) = if rest.is_empty() {
            ("id", "title")
        } else if let Some(slash) = rest.find('/') {
            (&rest[..slash], &rest[slash + 1..])
        } else {
            (rest, rest)
        };
        if let Some(arr) = get_by_path(v, base).and_then(|x| x.as_array()) {
            for item in arr {
                let val = get_by_path(item, val_path)
                    .and_then(|x| x.as_str().map(|s| s.to_string()))
                    .or_else(|| {
                        get_by_path(item, val_path).and_then(|x| x.as_i64().map(|n| n.to_string()))
                    })
                    .or_else(|| {
                        get_by_path(item, val_path).and_then(|x| x.as_f64().map(|f| f.to_string()))
                    })
                    .unwrap_or_else(|| item.to_string());
                let lbl = get_by_path(item, lbl_path)
                    .and_then(|x| x.as_str().map(|s| s.to_string()))
                    .or_else(|| {
                        get_by_path(item, lbl_path).and_then(|x| x.as_i64().map(|n| n.to_string()))
                    })
                    .or_else(|| {
                        get_by_path(item, lbl_path).and_then(|x| x.as_f64().map(|f| f.to_string()))
                    })
                    .unwrap_or_else(|| val.clone());
                out.push((lbl, val));
            }
        }
        return out;
    }
    if let Some(arr) = get_by_path(v, uw)
        .or_else(|| v.get("data").and_then(|d| d.get("items")))
        .and_then(|x| x.as_array())
    {
        for item in arr {
            if let Some(s) = item.as_str() {
                out.push((s.to_string(), s.to_string()));
            } else if let Some(obj) = item.as_object() {
                let val = obj
                    .get("id")
                    .and_then(|x| x.as_str())
                    .map(|s| s.to_string())
                    .or_else(|| {
                        obj.get("value")
                            .and_then(|x| x.as_str())
                            .map(|s| s.to_string())
                    })
                    .unwrap_or_else(|| item.to_string());
                let lbl = obj
                    .get("title")
                    .and_then(|x| x.as_str())
                    .map(|s| s.to_string())
                    .or_else(|| {
                        obj.get("name")
                            .and_then(|x| x.as_str())
                            .map(|s| s.to_string())
                    })
                    .unwrap_or_else(|| val.clone());
                out.push((lbl, val));
            }
        }
    }
    out
}

pub fn load_lazy_children_cmd(mi: &MenuItem) -> Result<Loaded> {
    let cmdline = mi
        .command
        .as_ref()
        .ok_or_else(|| anyhow!("No command configured for '{}'.", mi.title))?;
    let v = run_cmdline_to_json(cmdline)?;
    let target = if let Some(path) = mi.unwrap.as_deref() {
        get_by_path(&v, path)
    } else {
        v.get("data").and_then(|d| d.get("items"))
    };

    // Check for pagination metadata
    let pagination = v.get("data").and_then(|d| d.get("pagination"));

    if let Some(arr) = target.and_then(|x| x.as_array()) {
        if let Some(pagination_data) = pagination {
            Ok(Loaded::ItemsWithPagination {
                items: arr.clone(),
                pagination: pagination_data.clone(),
            })
        } else {
            Ok(Loaded::Items(arr.clone()))
        }
    } else {
        Ok(Loaded::Fallback(v))
    }
}

pub fn load_lazy_children_value_cmd(val: &JsonValue) -> Result<Loaded> {
    let cmdline = val
        .get("command")
        .and_then(|s| s.as_str())
        .ok_or_else(|| anyhow!("No command configured for this node"))?;
    let v = run_cmdline_to_json(cmdline)?;
    let target = if let Some(path) = val.get("unwrap").and_then(|s| s.as_str()) {
        get_by_path(&v, path)
    } else {
        v.get("data").and_then(|d| d.get("items"))
    };

    // Check for pagination metadata
    let pagination = v.get("data").and_then(|d| d.get("pagination"));

    if let Some(arr) = target.and_then(|x| x.as_array()) {
        if let Some(pagination_data) = pagination {
            Ok(Loaded::ItemsWithPagination {
                items: arr.clone(),
                pagination: pagination_data.clone(),
            })
        } else {
            Ok(Loaded::Items(arr.clone()))
        }
    } else {
        Ok(Loaded::Fallback(v))
    }
}

// Panel helpers: load panel content (cmd or yaml) and send via LoadMsg
pub fn spawn_load_panel_cmd(
    cmdline: String,
    kind: crate::ui::LoadKind,
    tx: Sender<crate::ui::LoadMsg>,
) {
    thread::spawn(move || {
        let outcome: Result<crate::ui::LoadOutcome, String> = match run_cmdline_to_json(&cmdline) {
            Ok(v) => Ok(crate::ui::LoadOutcome::Fallback(v)),
            Err(e) => Err(format!("{e}")),
        };
        let key = match kind {
            crate::ui::LoadKind::PanelA => "panel:A",
            crate::ui::LoadKind::PanelB => "panel:B",
            _ => "panel:?",
        };
        let _ = tx.send(crate::ui::LoadMsg {
            key: key.to_string(),
            outcome,
            kind,
        });
    });
}

#[cfg(test)]
mod loader_tests;

pub fn spawn_load_panel_yaml(
    path: String,
    kind: crate::ui::LoadKind,
    tx: Sender<crate::ui::LoadMsg>,
) {
    thread::spawn(move || {
        let outcome = (|| -> Result<crate::ui::LoadOutcome, String> {
            let full_path = {
                let pb = PathBuf::from(&path);
                if pb.is_absolute() {
                    pb
                } else if let Ok(dir) = std::env::var("CHI_TUI_CONFIG_DIR") {
                    PathBuf::from(&dir).join(&path)
                } else {
                    // As a last resort, try CWD
                    std::env::current_dir()
                        .unwrap_or_else(|_| PathBuf::from("."))
                        .join(&path)
                }
            };
            let s = std::fs::read_to_string(&full_path)
                .map_err(|e| format!("reading {full_path:?}: {e}"))?;
            let v: serde_json::Value = serde_yaml::from_str(&s).map_err(|e| {
                if let Some(loc) = e.location() {
                    format!("{}:{}:{}: {}", path, loc.line(), loc.column(), e)
                } else {
                    format!("{path}: {e}")
                }
            })?;
            Ok(crate::ui::LoadOutcome::Fallback(v))
        })();
        let key = match kind {
            crate::ui::LoadKind::PanelA => "panel:A",
            crate::ui::LoadKind::PanelB => "panel:B",
            _ => "panel:?",
        };
        let _ = tx.send(crate::ui::LoadMsg {
            key: key.to_string(),
            outcome,
            kind,
        });
    });
}

// Submit a form: run command once and try to parse either stdout (success envelope)
// or stderr (error envelope). Send the JSON back as Fallback so UI can decide.
pub fn spawn_submit_form(
    cmdline: String,
    kind: crate::ui::LoadKind,
    tx: Sender<crate::ui::LoadMsg>,
) {
    thread::spawn(move || {
        let outcome = (|| -> Result<crate::ui::LoadOutcome, String> {
            let parts =
                shlex::split(&cmdline).ok_or_else(|| "Failed to parse command line".to_string())?;
            if parts.is_empty() {
                return Err("Empty command".into());
            }
            let program = &parts[0];
            let args = &parts[1..];
            let output = std::process::Command::new(program)
                .args(args)
                .env("CHI_TUI_JSON", "1")
                .output()
                .map_err(|e| format!("spawn: {e}"))?;
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout).to_string();
                let v: JsonValue =
                    serde_json::from_str(&text).map_err(|e| format!("parse json: {e}"))?;
                Ok(crate::ui::LoadOutcome::Fallback(v))
            } else {
                // Try parse stderr as JSON error envelope; fallback to plain text
                let err_text = String::from_utf8_lossy(&output.stderr).to_string();
                if let Ok(v) = serde_json::from_str::<JsonValue>(&err_text) {
                    Ok(crate::ui::LoadOutcome::Fallback(v))
                } else {
                    Err(format!("Command failed: {cmdline}\n{err_text}"))
                }
            }
        })();
        let _ = tx.send(crate::ui::LoadMsg {
            key: "panel:B".into(),
            outcome,
            kind,
        });
    });
}

// Async wrappers used by autoload to fetch children off-thread and report back
pub fn spawn_load_for_menu(mi: MenuItem, key: String, tx: Sender<crate::ui::LoadMsg>) {
    thread::spawn(move || {
        let outcome: Result<crate::ui::LoadOutcome, String> = match load_lazy_children_cmd(&mi) {
            Ok(Loaded::Items(arr)) => Ok(crate::ui::LoadOutcome::Items(arr)),
            Ok(Loaded::ItemsWithPagination { items, pagination }) => {
                Ok(crate::ui::LoadOutcome::ItemsWithPagination { items, pagination })
            }
            Ok(Loaded::Fallback(v)) => Ok(crate::ui::LoadOutcome::Fallback(v)),
            Err(e) => Err(format!("{e}")),
        };
        let _ = tx.send(crate::ui::LoadMsg {
            key,
            outcome,
            kind: crate::ui::LoadKind::Menu,
        });
    });
}

pub fn spawn_load_for_value(val: serde_json::Value, key: String, tx: Sender<crate::ui::LoadMsg>) {
    thread::spawn(move || {
        let outcome: Result<crate::ui::LoadOutcome, String> =
            match load_lazy_children_value_cmd(&val) {
                Ok(Loaded::Items(arr)) => Ok(crate::ui::LoadOutcome::Items(arr)),
                Ok(Loaded::ItemsWithPagination { items, pagination }) => {
                    Ok(crate::ui::LoadOutcome::ItemsWithPagination { items, pagination })
                }
                Ok(Loaded::Fallback(v)) => Ok(crate::ui::LoadOutcome::Fallback(v)),
                Err(e) => Err(format!("{e}")),
            };
        let _ = tx.send(crate::ui::LoadMsg {
            key,
            outcome,
            kind: crate::ui::LoadKind::Child,
        });
    });
}
