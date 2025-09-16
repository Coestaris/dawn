use std::path::Path;

build_info::build_info!(pub fn dawn_build_info);

pub enum Compression {
    None,
    Default,
    Fast,
    Best,
}

pub fn package(
    assets_dir: &Path,
    output_file: &Path,
    compression: Compression,
) -> Result<(), String> {
    use dawn_dac::{ChecksumAlgorithm, CompressionLevel, ReadMode, Version};
    use dawn_dacgen::config::WriteConfig;
    use dawn_dacgen::write_from_directory;
    let file = std::fs::File::create(output_file).unwrap();
    let mut writer = std::io::BufWriter::new(file);

    // Make compression none in debug mode for faster builds
    let compression_level = match compression {
        Compression::None => CompressionLevel::None,
        Compression::Default => CompressionLevel::Default,
        Compression::Fast => CompressionLevel::Fast,
        Compression::Best => CompressionLevel::Best,
    };

    let build_info = dawn_build_info();

    write_from_directory(
        &mut writer,
        assets_dir.to_path_buf(),
        WriteConfig {
            read_mode: ReadMode::Recursive,
            checksum_algorithm: ChecksumAlgorithm::Blake3,
            compression_level,
            cache_dir: dirs::cache_dir().unwrap().join("dawn"),
            author: Some("Coestaris".to_string()),
            description: Some("DAWN assets".to_string()),
            version: Some(Version::new(
                build_info.crate_info.version.major as u16,
                build_info.crate_info.version.minor as u16,
                build_info.crate_info.version.patch as u16,
                None,
            )),
            license: Some("MIT".to_string()),
        },
    )
    .map_err(|e| format!("Failed to write package: {}", e))?;
    Ok(())
}

fn main() {}
