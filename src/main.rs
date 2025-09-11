// Do not display a console window on Windows
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crate::logging::setup_logging;
use crate::run::run_dawn;
use log::error;
use std::panic;

#[cfg(feature = "devtools")]
pub mod devtools;
pub mod logging;
pub mod rendering;
mod run;
pub mod world;

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
enum WorldSyncMode {
    SynchronizedWithMonitor,
    FixedTickRate(usize),
}

const WORLD_SYNC_MODE: WorldSyncMode = WorldSyncMode::SynchronizedWithMonitor;

// #[cfg(target_os = "linux")]
// const WORLD_SYNC_MODE: WorldSyncMode = WorldSyncMode::FixedTickRate(60);
// #[cfg(not(target_os = "linux"))]
// const WORLD_SYNC_MODE: WorldSyncMode = WorldSyncMode::SynchronizedWithMonitor;

pub fn panic_hook(info: &panic::PanicHookInfo) {
    // For development, it's more convenient to see the panic messages in the console.
    #[cfg(not(debug_assertions))]
    {}

    error!("Panic: {}", info);
}

fn main() {
    // Disable colors in the release builds to not consume extra resources.
    // It also makes the log files much more readable.
    #[cfg(not(debug_assertions))]
    setup_logging(log::LevelFilter::Info, Some("dawn_log".into()), false);

    #[cfg(debug_assertions)]
    setup_logging(log::LevelFilter::Info, None, true);

    run_dawn(WORLD_SYNC_MODE);
}
