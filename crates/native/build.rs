fn main() {
    let _build_info = build_info_build::build_script().build();

    // Set up Windows resources (icon, version info) in release builds
    #[cfg(target_os = "windows")] // TODO: Cross compilation?
    if std::env::var("PROFILE").unwrap() == "release"
        && std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows"
    {
        #[cfg(target_os = "windows")]
        use winresource::VersionInfo;

        let mut res = winresource::WindowsResource::new();
        res.set_icon("../../assets/icon.ico");

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
            _build_info.crate_info.version.major,
            _build_info.crate_info.version.minor,
            _build_info.crate_info.version.patch,
            0
        );
        res.set_version_info(VersionInfo::FILEVERSION, version_code);
        res.set_version_info(VersionInfo::PRODUCTVERSION, version_code);
        res.set_manifest(include_str!("../../assets/app.manifest"));
        res.compile().unwrap();
    }

    #[cfg(feature = "build_assets")]
    {
        use dawn_package::package;
        use dawn_package::Compression;
        use std::path::PathBuf;

        let profile = std::env::var("PROFILE").unwrap(); // "debug" or "release"
        let mut root_dir: PathBuf = std::env::var_os("CARGO_MANIFEST_DIR").unwrap().into();
        root_dir = root_dir.parent().unwrap().parent().unwrap().to_path_buf();

        let target_dir = root_dir.join("target");

        let assets = root_dir.join("assets");
        let cache_dir = target_dir.join("dac_cache");
        let output_file = target_dir.join(&profile).join("assets.dac");

        package(
            &assets,
            &output_file,
            &cache_dir,
            if profile == "release" {
                Compression::Default
            } else {
                Compression::None
            },
        )
        .unwrap();

        println!("cargo:rerun-if-changed=../../assets");
    }
}
