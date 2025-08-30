use ansi_term::Color::{Blue, Cyan, Green, Red, Yellow};
use build_info::VersionControl;
use log::{Level, LevelFilter};
use std::mem;
use std::path::PathBuf;
use std::ptr::addr_of_mut;
use std::time::SystemTime;
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

pub fn setup_logging(level: LevelFilter, file_logging: Option<PathBuf>, colored: bool) {
    let mut dispatch = fern::Dispatch::new().level(level).chain(std::io::stdout());

    if colored {
        dispatch = dispatch.format(|out, message, record| {
            out.finish(format_args!(
                "[{}][{:>19}][{:>14}]: {} [{}:{}]",
                Cyan.paint(format_system_time(SystemTime::now()).unwrap_or("unknown".to_string())),
                Yellow
                    .paint(std::thread::current().name().unwrap_or("main"))
                    .to_string(),
                match record.level() {
                    Level::Error => Red.paint(record.level().to_string()).to_string(),
                    Level::Warn => Yellow.paint(record.level().to_string()).to_string(),
                    Level::Info => Green.paint(record.level().to_string()).to_string(),
                    Level::Debug => Blue.paint(record.level().to_string()).to_string(),
                    Level::Trace => Cyan.paint(record.level().to_string()).to_string(),
                },
                message,
                Green.paint(record.file().unwrap_or("unknown")),
                Green.paint(record.line().unwrap_or(0).to_string())
            ));
        })
    } else {
        dispatch = dispatch.format(|out, message, record| {
            out.finish(format_args!(
                "[{}][{:>19}][{:>14}]: {} [{}:{}]",
                format_system_time(SystemTime::now()).unwrap_or("unknown".to_string()),
                std::thread::current().name().unwrap_or("main"),
                record.level(),
                message,
                record.file().unwrap_or("unknown"),
                record.line().unwrap_or(0)
            ));
        });
    }

    if let Some(path) = file_logging {
        dispatch = dispatch.chain(fern::log_file(path).unwrap());
    }

    dispatch.apply().unwrap();

    log_build_info();
}
