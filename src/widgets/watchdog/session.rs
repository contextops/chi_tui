use super::config::WatchdogConfig;
use super::detectors::{CommandDetector, Detector};
use super::killers::{CommandKiller, Killer};
use super::spawners::{LocalSpawner, Spawner};
use super::util::{push_line, run_cmd_quiet};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

pub struct CmdLog {
    pub cmd: String,
    pub output: Arc<Mutex<VecDeque<String>>>,
}

struct Worker {
    // stop flag observed inside the run loop; when set, the run loop kills the child
    stop: Arc<AtomicBool>,
    // thread handle for parallel mode; None in sequential mode
    handle: Option<std::thread::JoinHandle<()>>,
}

pub struct WatchdogSession {
    pub cmds: Vec<CmdLog>,
    pub cfg: WatchdogConfig,
    pub started: bool,
    // per command worker state
    workers: Vec<Worker>,
    // Orchestrator thread for sequential mode
    seq_handle: Option<std::thread::JoinHandle<()>>,
    // External mode flags
    pub external: bool,
    pub external_running: bool,
    external_stop: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    external_handle: Option<std::thread::JoinHandle<()>>,
    detector: Option<Box<dyn Detector + Send + Sync>>,
    killer: Option<Box<dyn Killer + Send + Sync>>,
    spawner: std::sync::Arc<dyn Spawner + Send + Sync>,
}

pub type WatchdogSessionRef = Arc<Mutex<WatchdogSession>>;

impl WatchdogSession {
    pub fn create(commands: Vec<String>, cfg: WatchdogConfig) -> WatchdogSessionRef {
        let mut cmds: Vec<CmdLog> = Vec::new();
        for raw in commands.iter() {
            let cmd = raw.clone();
            let log = CmdLog {
                cmd,
                output: Arc::new(Mutex::new(VecDeque::new())),
            };
            cmds.push(log);
        }
        let workers: Vec<Worker> = (0..cmds.len())
            .map(|_| Worker {
                stop: Arc::new(AtomicBool::new(false)),
                handle: None,
            })
            .collect();
        let session = Arc::new(Mutex::new(WatchdogSession {
            cmds,
            cfg,
            started: false,
            workers,
            seq_handle: None,
            external: false,
            external_running: false,
            external_stop: None,
            external_handle: None,
            detector: None,
            killer: None,
            spawner: std::sync::Arc::new(LocalSpawner::new()),
        }));
        {
            let mut s = session.lock().unwrap();
            // Seed and start workers
            for c in &s.cmds {
                push_line(&c.output, format!("[start] {}", c.cmd));
            }
            // External mode: if configured, do not spawn processes; start external detector loop
            if s.cfg.external_check_cmd.is_some() {
                s.external = true;
                // Replace the seed line with external notice for clarity
                for c in &s.cmds {
                    push_line(
                        &c.output,
                        "[external mode] will not spawn commands".to_string(),
                    );
                }
                let check_cmd = s.cfg.external_check_cmd.clone().unwrap();
                s.detector = Some(Box::new(CommandDetector::new(check_cmd.clone())));
                if let Some(kill_cmd) = s.cfg.external_kill_cmd.clone() {
                    s.killer = Some(Box::new(CommandKiller::new(kill_cmd)));
                }
                let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
                s.external_stop = Some(stop.clone());
                let sess_clone = Arc::clone(&session);
                s.external_handle = Some(std::thread::spawn(move || {
                    // Poll every 1000ms
                    let mut last: Option<bool> = None;
                    loop {
                        if stop.load(std::sync::atomic::Ordering::SeqCst) {
                            break;
                        }
                        let running = matches!(run_cmd_quiet(&check_cmd), Some(0));
                        if let Ok(mut g) = sess_clone.lock() {
                            g.external_running = running;
                            if last.map(|v| v != running).unwrap_or(true) {
                                // status changed; append note to panes
                                for c in &g.cmds {
                                    if running {
                                        push_line(
                                            &c.output,
                                            "[external] running (detected)".to_string(),
                                        );
                                    } else {
                                        push_line(&c.output, "[external] not running".to_string());
                                    }
                                }
                                last = Some(running);
                            }
                        }
                        std::thread::sleep(std::time::Duration::from_millis(1000));
                    }
                }));
            } else {
                s.start_locked();
            }
        }
        session
    }

    pub fn start(&mut self) {
        if self.started {
            return;
        }
        self.start_locked();
    }

    fn start_locked(&mut self) {
        self.started = true;
        // reset stop flags
        for w in &mut self.workers {
            w.stop.store(false, Ordering::SeqCst);
        }
        if self.cfg.sequential {
            self.spawn_sequential();
        } else {
            self.spawn_parallel();
        }
    }

    pub fn stop_all(&mut self) {
        // Request stop and kill running child processes
        for w in &mut self.workers {
            w.stop.store(true, Ordering::SeqCst);
        }
        // Join threads
        if let Some(h) = self.seq_handle.take() {
            let _ = h.join();
        }
        for w in &mut self.workers {
            if let Some(h) = w.handle.take() {
                let _ = h.join();
            }
        }
        // Stop external detector if any
        if let Some(stop) = &self.external_stop {
            stop.store(true, Ordering::SeqCst);
        }
        if let Some(h) = self.external_handle.take() {
            let _ = h.join();
        }
        self.started = false;
    }

    pub fn clear_outputs(&mut self) {
        for c in &mut self.cmds {
            if let Ok(mut q) = c.output.lock() {
                q.clear();
            }
        }
    }

    pub fn restart_all(&mut self, clear: bool) {
        if self.external {
            // In external mode, restart is not applicable. Just recheck status and notify.
            if clear {
                self.clear_outputs();
            }
            for c in &self.cmds {
                push_line(
                    &c.output,
                    "[external mode] restart not supported".to_string(),
                );
            }
        } else {
            self.stop_all();
            if clear {
                self.clear_outputs();
            }
            // Seed after clear for visibility
            for c in &self.cmds {
                push_line(&c.output, format!("[start] {}", c.cmd));
            }
            self.start_locked();
        }
    }

    pub fn kill_external(&mut self) -> bool {
        if !self.external {
            return false;
        }
        if let Some(k) = &self.killer {
            k.kill();
            true
        } else if let Some(cmd) = &self.cfg.external_kill_cmd {
            let _ = run_cmd_quiet(cmd);
            true
        } else {
            false
        }
    }

    fn spawn_parallel(&mut self) {
        // spawn one thread per command, each with retries
        for (idx, cmd) in self.cmds.iter().enumerate() {
            let lines_arc = Arc::clone(&cmd.output);
            let cfg = self.cfg.clone();
            let stop = self.workers[idx].stop.clone();
            let raw = cmd.cmd.clone();
            let spawner = self.spawner.clone();
            self.workers[idx].handle = Some(thread::spawn(move || {
                let _ = spawner.run_with_retries(&lines_arc, &raw, &cfg, None, &stop);
            }));
        }
    }

    fn spawn_sequential(&mut self) {
        let buffers: Vec<Arc<Mutex<VecDeque<String>>>> =
            self.cmds.iter().map(|c| Arc::clone(&c.output)).collect();
        let raw_cmds: Vec<String> = self.cmds.iter().map(|c| c.cmd.clone()).collect();
        let cfg = self.cfg.clone();
        // Take stop flags per worker
        let stops: Vec<Arc<AtomicBool>> = self.workers.iter().map(|w| w.stop.clone()).collect();
        let spawner = self.spawner.clone();
        self.seq_handle = Some(thread::spawn(move || {
            for (idx, raw) in raw_cmds.into_iter().enumerate() {
                let lines_arc = Arc::clone(&buffers[idx]);
                let stop = &stops[idx];
                let ok = spawner.run_with_retries(&lines_arc, &raw, &cfg, Some(idx), stop);
                if stop.load(Ordering::SeqCst) {
                    // stop requested: abort remaining
                    break;
                }
                if !ok && cfg.stop_on_failure {
                    for buf in buffers.iter().skip(idx + 1) {
                        push_line(buf, "[aborted by stop_on_failure]".to_string());
                    }
                    break;
                }
            }
        }));
    }
}
