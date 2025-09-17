use build_info::{BuildInfo, VersionControl};
use log::{info, Level};
use std::fmt;
use std::path::Path;
use std::sync::OnceLock;
use web_time::{Instant, SystemTime};

pub fn format_system_time(system_time: SystemTime) -> Option<String> {
    let datetime: chrono::DateTime<chrono::Utc> = system_time.into();
    Some(datetime.format("%Y-%m-%d %H:%M:%S UTC").to_string())
}

pub fn print_build_info(bi: &BuildInfo) {
    info!(r" _______       ___   ____    __    ____ .__   __.");
    info!(r"|       \     /   \  \   \  /  \  /   / |  \ |  |");
    info!(r"|  .--.  |   /  ^  \  \   \/    \/   /  |   \|  |");
    info!(r"|  |  |  |  /  /_\  \  \            /   |  . `  |");
    info!(r"|  '--'  | /  _____  \  \    /\    /    |  |\   |");
    info!(r"|_______/ /__/     \__\  \__/  \__/     |__| \__|");
    info!(
        "Current time: {}",
        format_system_time(SystemTime::now()).unwrap()
    );
    info!("Build Information:");
    info!("  Version: {}", bi.crate_info.version);
    info!("  Features: {:?}", bi.crate_info.enabled_features);
    info!("  Timestamp: {}", bi.timestamp);
    info!("  Profile: {}", bi.profile);
    info!("  Optimizations: {}", bi.optimization_level);
    info!("  Target: {}", bi.target);
    info!("  Compiler: {}", bi.compiler);
    if let Some(VersionControl::Git(git)) = &bi.version_control {
        info!("  VCS (Git) Information:");
        info!("    Commit: {} ({})", git.commit_id, git.commit_timestamp);
        info!("    Is dirty: {}", git.dirty);
        info!("    Refs: {:?}, {:?}", git.branch, git.tags);
    }
}

// Store the start time of the application
// Used for logging elapsed time
pub static START_TIME: OnceLock<Instant> = OnceLock::new();

pub fn format_inner<'a, F, const COLORED: bool>(
    message: &'a fmt::Arguments<'a>,
    record: &'a log::Record<'a>,
    callback: F,
) where
    F: FnOnce(fmt::Arguments),
{
    let red: &'static str = if COLORED { "\x1B[31m" } else { "" }; // Red
    let yellow: &'static str = if COLORED { "\x1B[33m" } else { "" }; // Yellow
    let green: &'static str = if COLORED { "\x1B[32m" } else { "" }; // Green
    let blue: &'static str = if COLORED { "\x1B[34m" } else { "" }; // Blue
    let magenta: &'static str = if COLORED { "\x1B[35m" } else { "" }; // Magenta
    let cyan: &'static str = if COLORED { "\x1B[36m" } else { "" }; // Cyan
    let white: &'static str = if COLORED { "\x1B[37m" } else { "" }; // White
    let reset: &'static str = if COLORED { "\x1B[0m" } else { "" }; // Reset

    let elapsed = START_TIME
        .get()
        .map(|start| start.elapsed())
        .unwrap_or_default();

    // Keep only the file name, not the full path since that can be very long
    // and filename is really additional info anyway
    let file = Path::new(record.file().unwrap_or("unknown"));
    let base = file.file_name().unwrap_or_default().to_string_lossy();
    let location = format!("{}:{}", base, record.line().unwrap_or(0));

    callback(format_args!(
        "[{cyan}{:^10.3}{reset}][{magenta}{:^25}{reset}][{yellow}{:^10}{reset}][{}{:>5}{reset}]: {}",
        elapsed.as_secs_f32(),
        location,
        std::thread::current().name().unwrap_or("main"),
        match record.level() {
            Level::Error => red,
            Level::Warn => yellow,
            Level::Info => green,
            Level::Debug => blue,
            Level::Trace => white,
        },
        record.level(),
        message,
    ))
}

pub fn format<'a, F>(message: &'a fmt::Arguments<'a>, record: &'a log::Record<'a>, callback: F)
where
    F: FnOnce(fmt::Arguments),
{
    format_inner::<F, false>(message, record, callback);
}

pub fn format_colored<'a, F>(
    message: &'a fmt::Arguments<'a>,
    record: &'a log::Record<'a>,
    callback: F,
) where
    F: FnOnce(fmt::Arguments),
{
    format_inner::<F, true>(message, record, callback);
}
