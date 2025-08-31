use build_info::VersionControl;
use fern::FormatCallback;
use log::{Level, LevelFilter};
use std::path::{Path, PathBuf};
use std::ptr::addr_of_mut;
use std::sync::OnceLock;
use std::time::{Instant, SystemTime};
use std::{fmt, mem};
/* Use a simple format instead of something like strftime,
 * to avoid unnecessary complexity, and to not extend the
 * dependency tree with a crate that provides it. */
#[allow(unused_imports)]
pub fn format_system_time(system_time: SystemTime) -> Option<String> {
    /* Get tm-like representation of the current time */
    let duration = system_time.duration_since(std::time::UNIX_EPOCH).ok()?;

    let tm = unsafe {
        let datetime = libc::time_t::try_from(duration.as_secs()).ok()?;
        let mut ret = mem::zeroed();
        #[cfg(windows)]
        {
            libc::localtime_s(addr_of_mut!(ret), &datetime);
        }
        #[cfg(unix)]
        {
            libc::localtime_r(&datetime, addr_of_mut!(ret));
        }
        ret
    };

    /* Format:
     * YYYY.MM.DD HH:MM:SS.{ms} */
    Some(format!(
        "{:04}.{:02}.{:02} {:02}:{:02}:{:02}.{:03}",
        tm.tm_year + 1900,
        tm.tm_mon + 1,
        tm.tm_mday,
        tm.tm_hour,
        tm.tm_min,
        tm.tm_sec,
        duration.subsec_millis()
    ))
}

fn log_build_info() {
    build_info::build_info!(fn build_info);
    let bi = build_info();

    log::info!("Current time: {}", format_system_time(SystemTime::now()).unwrap());
    log::info!("Build Information:");
    log::info!("  Timestamp: {}", bi.timestamp);
    log::info!("  Profile: {}", bi.profile);
    log::info!("  Optimizations: {}", bi.optimization_level);
    log::info!("  Crate info: {}", bi.crate_info);
    log::info!("  Target: {}", bi.target);
    log::info!("  Compiler: {}", bi.compiler);
    if let Some(VersionControl::Git(git)) = &bi.version_control {
        log::info!("  VCS (Git) Information:");
        log::info!("    Commit: {} ({})", git.commit_id, git.commit_timestamp);
        log::info!("    Is dirty: {}", git.dirty);
        log::info!("    Refs: {:?}, {:?}", git.branch, git.tags);
    }
}

// Store the start time of the application
// Used for logging elapsed time
static START_TIME: OnceLock<Instant> = OnceLock::new();

fn format<const COLORED: bool>(
    callback: FormatCallback,
    message: &fmt::Arguments,
    record: &log::Record,
) {
    let (red, yellow, green, blue, magenta, cyan, white, reset) = if COLORED {
        (
            "\x1B[31m", // Red
            "\x1B[33m", // Yellow
            "\x1B[32m", // Green
            "\x1B[34m", // Blue
            "\x1B[35m", // Magenta
            "\x1B[36m", // Cyan
            "\x1B[37m", // White
            "\x1B[0m",  // Reset
        )
    } else {
        ("", "", "", "", "", "", "", "")
    };

    let elapsed = START_TIME.get().map(|start| start.elapsed()).unwrap();

    // Keep only the file name, not the full path since that can be very long
    // and filename is really additional info anyway
    let file = Path::new(record.file().unwrap_or("unknown"));
    let base = file.file_name().unwrap_or_default().to_string_lossy();
    let location = format!("{}:{}", base, record.line().unwrap_or(0));

    callback.finish(format_args!(
        "[{cyan}{:^10.3}{reset}][{magenta}{:^30}{reset}][{yellow}{:^10}{reset}][{}{:>5}{reset}]: {}",
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
    ));
}

pub fn setup_logging(level: LevelFilter, file_logging: Option<PathBuf>, colored: bool) {
    START_TIME.set(Instant::now()).ok();

    let mut dispatch = fern::Dispatch::new().level(level).chain(std::io::stdout());

    if colored {
        dispatch = dispatch.format(format::<true>);
    } else {
        dispatch = dispatch.format(format::<false>);
    }

    if let Some(path) = file_logging {
        dispatch = dispatch.chain(fern::log_file(path).unwrap());
    }

    dispatch.apply().unwrap();

    log_build_info();
}
