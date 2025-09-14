use dawn_app::{run_dawn, WorldSyncMode};
use log::{error, info, trace};
use std::panic;
pub use wasm_bindgen::prelude::*;
use web_sys::console::debug;

build_info::build_info!(pub fn dawn_build_info);

fn panic_hook(info: &panic::PanicHookInfo) {
    // TODO: Display a dialog in the browser if possible.
    error!("Panic: {}", info);
}

pub struct WebLogger;

impl WebLogger {
    fn log(msg: &str) {
        web_sys::console::log_1(&msg.into());
    }

    fn error(msg: &str) {
        web_sys::console::error_1(&msg.into());
    }

    fn warn(msg: &str) {
        web_sys::console::warn_1(&msg.into());
    }
}

impl log::Log for WebLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::Level::Info
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            match record.level() {
                log::Level::Error => {
                    dawn_app::logging::format_colored(record.args(), record, |fmt| {
                        WebLogger::error(&fmt.to_string())
                    })
                }
                log::Level::Warn => {
                    dawn_app::logging::format_colored(record.args(), record, |fmt| {
                        WebLogger::warn(&fmt.to_string())
                    })
                }
                _ => dawn_app::logging::format_colored(record.args(), record, |fmt| {
                    WebLogger::log(&fmt.to_string())
                }),
            }
        }
    }

    fn flush(&self) {}
}

#[wasm_bindgen]
pub fn run() {
    WebLogger::log("Starting Dawn in WebAssembly...");

    let logger = WebLogger;
    if let Err(e) = log::set_boxed_logger(Box::new(logger)) {
        WebLogger::error(&format!("Failed to set logger: {}", e));
    }

    log::set_max_level(log::LevelFilter::Info);

    info!("Logger initialized");

    run_dawn(
        WorldSyncMode::SynchronizedWithMonitor,
        dawn_build_info().clone(),
        panic_hook,
    );
}
