// Do not display a console window on Windows
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use build_info::BuildInfo;
use log::{error, LevelFilter};
use std::backtrace::BacktraceStatus;
use std::panic;
use std::path::PathBuf;
use web_time::Instant;

build_info::build_info!(pub fn dawn_build_info);

pub fn setup_logging(level: LevelFilter, file_logging: Option<PathBuf>, colored: bool) {
    let mut dispatch = fern::Dispatch::new().level(level).chain(std::io::stdout());

    if colored {
        dispatch = dispatch
            .format(|cb, args, r| dawn_app::logging::format_colored(args, r, |fmt| cb.finish(fmt)));
    } else {
        dispatch = dispatch
            .format(|cb, args, r| dawn_app::logging::format(args, r, |fmt| cb.finish(fmt)));
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

fn main() {
    use dawn_app::{run_dawn, WorldSyncMode};

    // Disable colors in the release builds to not consume extra resources.
    // It also makes the log files much more readable.
    #[cfg(not(debug_assertions))]
    setup_logging(log::LevelFilter::Info, Some("dawn_log".into()), false);
    #[cfg(debug_assertions)]
    setup_logging(log::LevelFilter::Info, None, true);

    run_dawn(
        WorldSyncMode::SynchronizedWithMonitor,
        dawn_build_info().clone(),
        Box::new(panic_hook),
    );
}
