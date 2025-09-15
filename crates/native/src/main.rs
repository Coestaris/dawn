// Do not display a console window on Windows
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use dawn_app::assets::reader::ReaderBackend;
use dawn_assets::ir::IRAsset;
use dawn_assets::AssetID;
use dawn_dac::reader::{read_asset, read_manifest};
use log::{error, LevelFilter};
use std::backtrace::BacktraceStatus;
use std::fs::File;
use std::io::BufReader;
use std::panic;
use std::path::PathBuf;
use std::sync::Arc;

build_info::build_info!(pub fn dawn_build_info);

pub fn setup_logging(level: LevelFilter, file_logging: Option<PathBuf>, colored: bool) {
    let mut dispatch = fern::Dispatch::new().level(level).chain(std::io::stdout());

    if colored {
        dispatch = dispatch
            .format(|cb, args, r| dawn_app::logging::format_colored(args, r, |fmt| cb.finish(fmt)));
    } else {
        dispatch =
            dispatch.format(|cb, args, r| dawn_app::logging::format(args, r, |fmt| cb.finish(fmt)));
    }

    if let Some(path) = file_logging {
        dispatch = dispatch.chain(fern::log_file(path).unwrap());
    }

    dispatch.apply().unwrap();
}

pub fn panic_hook(info: &panic::PanicHookInfo) {
    // For development, it's more convenient to see the panic messages in the console.
    #[cfg(not(debug_assertions))]
    {
        use native_dialog;
        native_dialog::DialogBuilder::message()
            .set_level(native_dialog::MessageLevel::Error)
            .set_title("Application Error")
            .set_text(&format!(
                "The application has encountered a fatal error:\n\n{}\n\nSee the log file for more details.\nApplication will now exit.",
                info
            ))
            .alert()
            .show()
            .map_err(|e| {
                error!("Failed to show panic message dialog: {}", e);
            });
    }

    error!("Panic: {}", info);

    // Print the backtrace if possible.
    let capture = std::backtrace::Backtrace::capture();
    if let BacktraceStatus::Captured = capture.status() {
        error!("Backtrace:\n{:?}", capture);
    }
}

struct Reader {
    path: PathBuf,
}

impl Reader {
    fn get_patch() -> PathBuf {
        // Try to find file with the same name in the current directory
        let path = std::env::current_dir().unwrap().join("assets.dac");
        if path.exists() {
            path
        } else {
            let exe_dir = std::env::current_exe()
                .unwrap()
                .parent()
                .unwrap()
                .to_path_buf();
            let path = exe_dir.join("assets.dac");
            if path.exists() {
                path
            } else {
                panic!("DAC file not found. Please ensure 'assets.dac' is present in the current directory or the executable directory.");
            }
        }
    }

    fn new() -> Self {
        Self {
            path: Self::get_patch(),
        }
    }
}

impl ReaderBackend for Reader {
    fn enumerate(&self) -> Result<dawn_dac::Manifest, anyhow::Error> {
        let file = File::open(self.path.as_path())?;
        let mut reader = BufReader::new(file);
        let err = read_manifest(&mut reader)?;
        Ok(err)
    }

    fn load(&self, aid: AssetID) -> Result<IRAsset, anyhow::Error> {
        let file = File::open(self.path.as_path())?;
        let mut reader = BufReader::new(file);
        let err = read_asset(&mut reader, aid.clone())?;
        Ok(err)
    }
}

fn main() {
    use dawn_app::{run_dawn, WorldSyncMode};

    // Disable colors in the release builds to not consume extra resources.
    // It also makes the log files much more readable.
    #[cfg(not(debug_assertions))]
    setup_logging(log::LevelFilter::Info, Some("dawn_log".into()), false);
    #[cfg(debug_assertions)]
    setup_logging(log::LevelFilter::Info, None, true);

    run_dawn(
        Arc::new(Reader::new()),
        WorldSyncMode::SynchronizedWithMonitor,
        dawn_build_info().clone(),
        Box::new(panic_hook),
    );
}
