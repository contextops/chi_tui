use serde::{Deserialize, Serialize};

pub const MAX_LINES_PER_CMD: usize = 5000;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WatchdogStatSpec {
    pub label: String,
    pub regexp: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WatchdogConfig {
    pub sequential: bool,
    pub auto_restart: bool,
    pub max_retries: usize,
    pub restart_delay_ms: u64,
    pub allowed_exit_codes: Vec<i32>,
    pub stop_on_failure: bool, // only meaningful for sequential
    pub on_panic_exit_cmd: Option<String>,
    pub stats: Vec<WatchdogStatSpec>,
    // External mode: do not spawn commands; detect and manage an externally-started process
    // If set, the session will poll this command periodically; exit code 0 => running
    pub external_check_cmd: Option<String>,
    // Optional command to terminate the external process
    pub external_kill_cmd: Option<String>,
}
