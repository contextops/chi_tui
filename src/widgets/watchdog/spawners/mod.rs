use std::collections::VecDeque;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use crate::widgets::watchdog::config::WatchdogConfig;

pub mod local;

pub trait Spawner: Send + Sync {
    fn run_with_retries(
        &self,
        lines_arc: &Arc<Mutex<VecDeque<String>>>,
        cmdline: &str,
        cfg: &WatchdogConfig,
        idx: Option<usize>,
        stop: &Arc<AtomicBool>,
    ) -> bool;
}

pub use local::LocalSpawner;
