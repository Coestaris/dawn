use dawn_app::assets::reader::ReaderBackend;
use dawn_app::{run_dawn, WorldSyncMode};
use dawn_assets::ir::IRAsset;
use dawn_assets::AssetID;
use dawn_dac::reader::{read_asset, read_manifest};
use dawn_dac::Manifest;
use log::info;
use serde::Deserialize;
use std::io::Cursor;
use std::panic;
use std::sync::Arc;
pub use wasm_bindgen::prelude::*;
use web_sys::console::debug;

// Raise error if compiled not for wasm32
#[cfg(not(target_arch = "wasm32"))]
compile_error!("This crate should only be compiled for the wasm32-unknown target.");

build_info::build_info!(pub fn dawn_build_info);

fn panic_hook(info: &panic::PanicHookInfo) {
    // TODO: Display a dialog in the browser if possible.
    WebLogger::error(format!("Panic: {}", info).as_str());

    // Print the backtrace if possible.
    let capture = std::backtrace::Backtrace::capture();
    if let std::backtrace::BacktraceStatus::Captured = capture.status() {
        WebLogger::error(format!("Backtrace:\n{:?}", capture).as_str());
    }
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

struct WebReader {
    resources: Resources,
}

#[derive(Deserialize)]
struct WebResource {
    #[serde(with = "serde_bytes")]
    content: Vec<u8>,
    hash: String,
    id: String,
    name: String,
    size: u64,
}

#[derive(Deserialize)]
struct Resources(Vec<WebResource>);

impl WebReader {
    pub fn new(resources: JsValue) -> Self {
        let resources: Resources = serde_wasm_bindgen::from_value(resources).unwrap();
        Self { resources }
    }
}

impl ReaderBackend for WebReader {
    fn enumerate(&self) -> Result<Manifest, anyhow::Error> {
        let resource = self.resources.0.iter().next().unwrap();
        let mut reader = Cursor::new(&resource.content);
        let err = read_manifest(&mut reader)?;
        Ok(err)
    }

    fn load(&self, aid: AssetID) -> Result<IRAsset, anyhow::Error> {
        let resource = self.resources.0.iter().next().unwrap();
        let mut reader = Cursor::new(&resource.content);
        let err = read_asset(&mut reader, aid.clone())?;
        Ok(err)
    }
}

#[wasm_bindgen]
// Takes JS dictionary with resources list
pub fn run(resources: JsValue) {
    // Bootstrap the panic hook
    // The app will override it later,
    // but we want to catch panics as early as possible
    panic::set_hook(Box::new(panic_hook));

    WebLogger::log("Starting Dawn in WebAssembly...");

    let logger = WebLogger;
    if let Err(e) = log::set_boxed_logger(Box::new(logger)) {
        WebLogger::error(&format!("Failed to set logger: {}", e));
    }

    log::set_max_level(log::LevelFilter::Info);

    info!("Logger initialized");

    run_dawn(
        Arc::new(WebReader::new(resources)),
        WorldSyncMode::SynchronizedWithMonitor,
        dawn_build_info().clone(),
        panic_hook,
    );
}
