use crate::widgets::watchdog::util::run_cmd_quiet;

#[allow(dead_code)]
pub trait Detector: Send + Sync {
    fn is_running(&self) -> bool;
}

#[allow(dead_code)]
pub struct CommandDetector {
    cmd: String,
}

impl CommandDetector {
    pub fn new(cmd: String) -> Self {
        Self { cmd }
    }
}

impl Detector for CommandDetector {
    fn is_running(&self) -> bool {
        matches!(run_cmd_quiet(&self.cmd), Some(0))
    }
}
