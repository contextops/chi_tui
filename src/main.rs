mod app;
mod chi_core;
mod model;
mod nav;
mod services;
mod theme;
mod ui;
mod visuals;
mod widgets;

use anyhow::Result;

fn main() -> Result<()> {
    ui::run()
}
