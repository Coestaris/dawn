use dawn_yarc::writer::write_from_directory;
use dawn_yarc::{ChecksumAlgorithm, Compression, ReadMode, WriteOptions};
use std::path::PathBuf;

fn main() {
    let current_dir = std::env::current_dir().unwrap().join("assets");
    let target_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("demo_graphics.yarc");
    write_from_directory(
        current_dir,
        WriteOptions {
            compression: Compression::Gzip,
            read_mode: ReadMode::Recursive,
            checksum_algorithm: ChecksumAlgorithm::Md5,
            author: Some("Coestaris <vk_vm@ukr.net>".to_string()),
            description: Some("DAWN assets".to_string()),
            version: Some("0.1.0".to_string()),
            license: Some("MIT".to_string()),
        },
        target_dir.clone(),
    )
    .unwrap();

    println!("cargo:rerun-if-changed=assets");
    println!("cargo:rustc-env=YARC_FILE={}", target_dir.display());
}
