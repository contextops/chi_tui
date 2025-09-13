use crate::widgets::watchdog::output::RingBufferSink;
use regex::Regex;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

pub fn push_line(buf: &Arc<Mutex<VecDeque<String>>>, s: String) {
    let sink = RingBufferSink::new(Arc::clone(buf));
    sink.push_line(s);
}

pub fn expand_vars(s: &str) -> String {
    // matches ${VAR}
    let re = Regex::new(r"\$\{([A-Z0-9_]+)\}").unwrap();
    let env_map: HashMap<String, String> = std::env::vars().collect();
    re.replace_all(s, |caps: &regex::Captures| {
        let key = &caps[1];
        if key == "APP_BIN" {
            if let Ok(v) = std::env::var("CHI_APP_BIN") {
                return v;
            }
            return "example-app".to_string();
        }
        env_map.get(key).cloned().unwrap_or_default()
    })
    .to_string()
}

// Execute a command line quietly (no captured stdout/stderr), returning exit code.
// Returns None on spawn error or if the process had no exit code.
pub fn run_cmd_quiet(cmdline: &str) -> Option<i32> {
    let expanded = expand_vars(cmdline);
    let parts = shlex::split(&expanded).unwrap_or_default();
    if parts.is_empty() {
        return None;
    }
    let program = &parts[0];
    let args = &parts[1..];
    match Command::new(program)
        .args(args)
        .env("CHI_TUI_JSON", "1")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
    {
        Ok(status) => status.code(),
        Err(_) => None,
    }
}
