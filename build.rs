use dawn_dac::{ChecksumAlgorithm, CompressionLevel, ReadMode};
use dawn_dacgen::config::WriteConfig;
use dawn_dacgen::write_from_directory;
use dirs::cache_dir;
use std::path::PathBuf;
use winresource::VersionInfo;

fn main() {
    let build_info = build_info_build::build_script().build();

    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico");

        //  MAJOR << 48 | MINOR << 32 | PATCH << 16 | RELEASE)
        macro_rules! version {
            ($major:expr, $minor:expr, $patch:expr, $release:expr) => {
                (($major as u64) << 48)
                    | (($minor as u64) << 32)
                    | (($patch as u64) << 16)
                    | ($release as u64)
            };
        }

        let version_code = version!(
            build_info.crate_info.version.major,
            build_info.crate_info.version.minor,
            build_info.crate_info.version.patch,
            0
        );
        res.set_version_info(VersionInfo::FILEVERSION, version_code);
        res.set_version_info(VersionInfo::PRODUCTVERSION, version_code);
        res.compile().unwrap();
    }

    let current_dir = std::env::current_dir().unwrap().join("assets");
    let output = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("assets.dac");
    let file = std::fs::File::create(&output).unwrap();
    let mut writer = std::io::BufWriter::new(file);

    // Make compression none in debug mode for faster builds
    let compression_level = if std::env::var("PROFILE").unwrap() == "release" {
        CompressionLevel::Default
    } else {
        CompressionLevel::None
    };

    write_from_directory(
        &mut writer,
        current_dir,
        WriteConfig {
            read_mode: ReadMode::Recursive,
            checksum_algorithm: ChecksumAlgorithm::Blake3,
            compression_level,
            cache_dir: cache_dir().unwrap().join("dawn_cache"),
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
