use dawn_dac::writer::{write_from_directory, ChecksumAlgorithm, ReadMode, WriteConfig};
use std::path::PathBuf;

fn main() {
    let current_dir = std::env::current_dir().unwrap().join("assets");
    let output = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("assets.dac");
    let file = std::fs::File::create(&output).unwrap();
    let mut writer = std::io::BufWriter::new(file);
    
    write_from_directory(
        &mut writer,
        current_dir,
        WriteConfig {
            read_mode: ReadMode::Recursive,
            checksum_algorithm: ChecksumAlgorithm::Blake3,
            author: Some("Coestaris <vk_vm@ukr.net>".to_string()),
            description: Some("DAWN assets".to_string()),
            version: Some("0.1.0".to_string()),
            license: Some("MIT".to_string()),
        },
    )
    .unwrap();

    println!("cargo:rerun-if-changed=assets");
    println!("cargo:rustc-env=DAC_FILE={}", output.display());
}
