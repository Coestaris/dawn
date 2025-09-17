// Do not display a console window on Windows
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use dawn_app::assets::reader::ReaderBackend;
use dawn_assets::ir::IRAsset;
use dawn_assets::AssetID;
use dawn_dac::reader::{read_asset, read_manifest};
use log::{error, LevelFilter};
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
}

struct Reader {
    path: PathBuf,
}

impl Reader {
    fn get_patch() -> PathBuf {
        const FILENAME: &str = "assets.dac";

        // Directories to search for the DAC file (in order)
        let dirs = [
            // Near the executable
            std::env::current_exe()
                .unwrap()
                .parent()
                .unwrap()
                .to_path_buf(),
            // Current working directory
            std::env::current_dir().unwrap(),
        ];

        for dir in &dirs {
            let candidate = dir.join(FILENAME);
            if candidate.exists() {
                return candidate;
            }
        }

        panic!(
            "Assets file '{}' not found. Searched in: {:?}. Consider putting it next to the executable or running the application from the directory containing the assets file.",
            FILENAME, dirs
        );
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

    // Bootstrap panic hook. The app will override it later,
    // but we want to catch panics as early as possible.
    panic::set_hook(Box::new(panic_hook));

    // Disable colors in the release builds to not consume extra resources.
    // It also makes the log files much more readable.
    #[cfg(not(debug_assertions))]
    setup_logging(log::LevelFilter::Info, Some("dawn.log".into()), false);
    #[cfg(debug_assertions)]
    setup_logging(log::LevelFilter::Info, None, true);

    run_dawn(
        Arc::new(Reader::new()),
        WorldSyncMode::SynchronizedWithMonitor,
        dawn_build_info().clone(),
        Box::new(panic_hook),
    );
}
