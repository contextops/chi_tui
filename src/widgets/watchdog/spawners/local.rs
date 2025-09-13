use std::collections::VecDeque;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::widgets::watchdog::config::WatchdogConfig;
use crate::widgets::watchdog::util::{expand_vars, push_line};

use super::Spawner;

#[derive(Default)]
pub struct LocalSpawner;

impl LocalSpawner {
    pub fn new() -> Self {
        Self
    }
}

impl Spawner for LocalSpawner {
    fn run_with_retries(
        &self,
        lines_arc: &Arc<Mutex<VecDeque<String>>>,
        cmdline: &str,
        cfg: &WatchdogConfig,
        _idx: Option<usize>,
        stop: &Arc<AtomicBool>,
    ) -> bool {
        let mut attempt = 0usize;
        loop {
            if stop.load(Ordering::SeqCst) {
                // Aborted before start
                push_line(lines_arc, "[stopped]".to_string());
                return false;
            }
            let status_code_opt = run_once(lines_arc, cmdline, stop);
            let mut success = false;
            if let Some(code) = status_code_opt {
                success =
                    cfg.allowed_exit_codes.is_empty() || cfg.allowed_exit_codes.contains(&code);
            }
            if success {
                push_line(lines_arc, "[done]".to_string());
                return true;
            }
            // failure path
            if stop.load(Ordering::SeqCst) {
                push_line(lines_arc, "[stopped]".to_string());
                return false;
            }
            if cfg.auto_restart && attempt < cfg.max_retries {
                let next = attempt + 1;
                let of = cfg.max_retries;
                push_line(
                    lines_arc,
                    format!(
                        "[retry {next}/{of} in {delay}ms]",
                        delay = cfg.restart_delay_ms
                    ),
                );
                let sleep_ms = cfg.restart_delay_ms;
                let mut waited = 0u64;
                while waited < sleep_ms {
                    if stop.load(Ordering::SeqCst) {
                        push_line(lines_arc, "[stopped]".to_string());
                        return false;
                    }
                    let step = 50;
                    thread::sleep(Duration::from_millis(step));
                    waited += step;
                }
                attempt = next;
                continue;
            } else {
                push_line(lines_arc, "[panic: retries exhausted]".to_string());
                if let Some(hook) = &cfg.on_panic_exit_cmd {
                    push_line(lines_arc, format!("[panic hook] running: {hook}"));
                    let _ = run_once(lines_arc, hook, stop);
                }
                return false;
            }
        }
    }
}

fn run_once(
    lines_arc: &Arc<Mutex<VecDeque<String>>>,
    cmdline: &str,
    stop: &Arc<AtomicBool>,
) -> Option<i32> {
    let expanded = expand_vars(cmdline);
    let parts = shlex::split(&expanded).unwrap_or_default();
    if parts.is_empty() {
        push_line(lines_arc, "[error] empty command".to_string());
        return None;
    }
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
            push_line(lines_arc, format!("[spawn error] {e}"));
            return None;
        }
    };
    // Concurrently read stdout and stderr
    let mut handles: Vec<std::thread::JoinHandle<()>> = Vec::new();
    if let Some(stdout) = child.stdout.take() {
        let out_buf = Arc::clone(lines_arc);
        handles.push(thread::spawn(move || {
            use std::io::{BufRead, BufReader};
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                push_line(&out_buf, line);
            }
        }));
    }
    if let Some(stderr) = child.stderr.take() {
        let err_buf = Arc::clone(lines_arc);
        handles.push(thread::spawn(move || {
            use std::io::{BufRead, BufReader};
            let reader = BufReader::new(stderr);
            for line in reader.lines().map_while(Result::ok) {
                push_line(&err_buf, format!("[stderr] {line}"));
            }
        }));
    }

    // Wait for child but stay responsive to stop
    loop {
        if stop.load(Ordering::SeqCst) {
            // Try to kill the child; ignore errors
            let _ = child.kill();
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                // Ensure readers are finished
                for h in handles {
                    let _ = h.join();
                }
                return status.code();
            }
            Ok(None) => {
                thread::sleep(Duration::from_millis(50));
                continue;
            }
            Err(_) => {
                // Failed to wait; assume None without crashing
                thread::sleep(Duration::from_millis(50));
            }
        }
    }
}
