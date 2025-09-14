fn main() {
    let build_info = build_info_build::build_script().build();

    #[cfg(feature = "build_assets")]
    {
        use dawn_dac::{ChecksumAlgorithm, CompressionLevel, ReadMode, Version};
        use dawn_dacgen::config::WriteConfig;
        use dawn_dacgen::write_from_directory;
        use dirs::cache_dir;
        use std::path::PathBuf;

        let current_dir = std::env::current_dir()
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("assets");

        let mut target_dir: PathBuf = std::env::var_os("CARGO_MANIFEST_DIR").unwrap().into();
        target_dir.push("www");
        target_dir.push("pkg");
        let output = target_dir.join("assets.dac");

        if !target_dir.exists() {
            std::fs::create_dir_all(&target_dir).unwrap();
        }

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
                version: Some(Version::new(
                    build_info.crate_info.version.major as u16,
                    build_info.crate_info.version.minor as u16,
                    build_info.crate_info.version.patch as u16,
                    None,
                )),
                license: Some("MIT".to_string()),
            },
        )
        .unwrap();

        println!("cargo:rerun-if-changed=../../assets");
    }
}
