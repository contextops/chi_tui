use crate::widgets::watchdog::util::run_cmd_quiet;

pub trait Killer: Send + Sync {
    fn kill(&self);
}

pub struct CommandKiller {
    pub cmd: String,
}

impl CommandKiller {
    pub fn new(cmd: String) -> Self {
        Self { cmd }
    }
}

impl Killer for CommandKiller {
    fn kill(&self) {
        let _ = run_cmd_quiet(&self.cmd);
    }
}
