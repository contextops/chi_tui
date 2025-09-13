pub mod config;
pub mod detectors;
pub mod killers;
pub mod output;
pub mod session;
pub mod spawners;
pub mod util;
pub mod widget;
pub use output::stats::StatsAggregator;

// Re-exports to preserve the existing public API
#[allow(unused_imports)]
pub use config::{WatchdogConfig, WatchdogStatSpec};
#[allow(unused_imports)]
pub use session::{CmdLog, WatchdogSession, WatchdogSessionRef};
#[allow(unused_imports)]
pub use spawners::{LocalSpawner, Spawner};
pub use widget::WatchdogWidget;
