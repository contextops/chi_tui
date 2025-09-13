use anyhow::{anyhow, Context, Result};
use regex::Regex;
use serde_json::Value as JsonValue;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::mpsc::Sender;
use std::thread;
use std::{collections::HashMap, env};

fn expand_cmdline_env(cmdline: &str) -> String {
    // Expand ${VAR} from environment; special-case ${APP_BIN}
    // -> CHI_APP_BIN (quoted if contains whitespace) or default "example-app"
    let re = Regex::new(r"\$\{([A-Z0-9_]+)\}").unwrap();
    let env_map: HashMap<String, String> = env::vars().collect();
    re.replace_all(cmdline, |caps: &regex::Captures| {
        let key = &caps[1];
        if key == "APP_BIN" {
            if let Some(v) = env_map.get("CHI_APP_BIN") {
                // Quote if contains whitespace to keep it a single arg in shlex::split
                let needs_quote = v.chars().any(|c| c.is_whitespace());
                if needs_quote {
                    let escaped = v.replace('"', "\\\"");
                    return format!("\"{escaped}\"");
                }
                return v.to_string();
            }
            return "example-app".to_string();
        }
        env_map.get(key).cloned().unwrap_or_default()
    })
    .to_string()
}

pub fn run_cmdline_to_json(cmdline: &str) -> Result<JsonValue> {
    let expanded = expand_cmdline_env(cmdline);
    let parts = shlex::split(&expanded).ok_or_else(|| anyhow!("Failed to parse command line"))?;
    if parts.is_empty() {
        return Err(anyhow!("Empty command line"));
    }
    let program = &parts[0];
    let args = &parts[1..];
    let output = Command::new(program)
        .args(args)
        .env("CHI_TUI_JSON", "1")
        .output()
        .with_context(|| format!("spawning {expanded}"))?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(anyhow!("Command failed: {}\n{}", cmdline, err));
    }
    let text = String::from_utf8_lossy(&output.stdout).to_string();
    let v: JsonValue = serde_json::from_str(&text).with_context(|| "parsing command JSON")?;
    Ok(v)
}

pub fn spawn_streaming_cmd(cmdline: String, tx: Sender<crate::ui::ProgressEvent>) {
    thread::spawn(move || {
        let expanded = expand_cmdline_env(&cmdline);
        let parts = match shlex::split(&expanded) {
            Some(p) if !p.is_empty() => p,
            _ => {
                let _ = tx.send(crate::ui::ProgressEvent {
                    text: None,
                    percent: None,
                    done: true,
                    result: None,
                    err: Some("Failed to parse command line".to_string()),
                });
                return;
            }
        };
        let program = &parts[0];
        let args = &parts[1..];
        let mut child = match Command::new(program)
            .args(args)
            .env("CHI_TUI_JSON", "1")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                let _ = tx.send(crate::ui::ProgressEvent {
                    text: None,
                    percent: None,
                    done: true,
                    result: None,
                    err: Some(format!("{e}")),
                });
                return;
            }
        };

        // Drop stderr to avoid blocking
        drop(child.stderr.take());

        let mut final_result: Option<JsonValue> = None;
        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                let l = line.trim();
                if l.is_empty() {
                    continue;
                }
                if let Ok(v) = serde_json::from_str::<JsonValue>(l) {
                    let typ = v.get("type").and_then(|s| s.as_str()).unwrap_or("result");
                    if typ == "progress" {
                        let data = v.get("data").cloned().unwrap_or(JsonValue::Null);
                        let mut text = data
                            .get("message")
                            .and_then(|s| s.as_str())
                            .unwrap_or("")
                            .to_string();
                        if let Some(stage) = data.get("stage").and_then(|s| s.as_str()) {
                            if !stage.is_empty() {
                                if text.is_empty() {
                                    text = stage.to_string();
                                } else {
                                    text = format!("{stage} — {text}");
                                }
                            }
                        }
                        let percent = data.get("percent").and_then(|p| p.as_f64());
                        let _ = tx.send(crate::ui::ProgressEvent {
                            text: if text.is_empty() { None } else { Some(text) },
                            percent,
                            done: false,
                            result: None,
                            err: None,
                        });
                    } else {
                        final_result = Some(v);
                        break;
                    }
                }
            }
        }

        let status = child.wait();
        let success = status.as_ref().map(|s| s.success()).unwrap_or(false);
        if let Some(v) = final_result {
            let _ = tx.send(crate::ui::ProgressEvent {
                text: None,
                percent: None,
                done: true,
                result: Some(v),
                err: None,
            });
        } else if !success {
            let _ = tx.send(crate::ui::ProgressEvent {
                text: None,
                percent: None,
                done: true,
                result: None,
                err: Some(format!("Command failed: {cmdline}")),
            });
        } else {
            let _ = tx.send(crate::ui::ProgressEvent {
                text: None,
                percent: None,
                done: true,
                result: Some(JsonValue::Null),
                err: None,
            });
        }
    });
}
