use ansi_term::Color::{Blue, Cyan, Green, Red, Yellow};
use log::{Level, Log, Metadata, Record};
use std::mem;
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

pub struct CommonLogger;

impl Log for CommonLogger {
    fn enabled(&self, _: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            fn colored_level(level: Level) -> ansi_term::Colour {
                match level {
                    Level::Error => Red,
                    Level::Warn => Yellow,
                    Level::Info => Green,
                    Level::Debug => Blue,
                    Level::Trace => Cyan,
                }
            }

            let formatted_date =
                format_system_time(SystemTime::now()).unwrap_or("unknown".to_string());

            println!(
                "[{}][{:>19}][{:>14}]: {} [{}:{}]",
                Cyan.paint(formatted_date),
                Yellow
                    .paint(std::thread::current().name().unwrap_or("main"))
                    .to_string(),
                colored_level(record.level())
                    .paint(record.level().to_string())
                    .to_string(),
                record.args(),
                Green.paint(record.file().unwrap_or("unknown")),
                Green.paint(record.line().unwrap_or(0).to_string())
            );
        }
    }

    fn flush(&self) {}
}
