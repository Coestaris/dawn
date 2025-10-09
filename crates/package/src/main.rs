use crate::lib::{package, Compression};
use clap::Parser;
use std::path::PathBuf;

mod lib;

#[derive(clap::Parser)]
struct Args {
    #[clap(long, short = 'i')]
    assets_dir: PathBuf,
    #[clap(long, short = 'o')]
    output_file: PathBuf,
    #[clap(long, default_value = "true")]
    compress: bool,
    #[clap(long, default_value = "")]
    cache_dir: PathBuf,
}

struct Logger;

impl log::Log for Logger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            println!("{} - {}", record.level(), record.args());
        }
    }

    fn flush(&self) {}
}

fn main() {
    // Setup logging
    log::set_boxed_logger(Box::new(Logger)).unwrap();
    log::set_max_level(log::LevelFilter::Debug);

    let args = Args::parse();

    let cache_dir = if args.cache_dir.as_os_str().is_empty() {
        dirs::cache_dir().unwrap().join("dawn")
    } else {
        args.cache_dir.clone()
    };

    package(
        &args.assets_dir,
        &args.output_file,
        cache_dir.as_path(),
        if args.compress {
            Compression::Default
        } else {
            Compression::None
        },
    )
    .unwrap();
}
