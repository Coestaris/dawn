use std::path::PathBuf;

fn main() {
    let build_info = build_info_build::build_script().build();

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
            build_info.crate_info.version.major,
            build_info.crate_info.version.minor,
            build_info.crate_info.version.patch,
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
        let profile = std::env::var("PROFILE").unwrap();
        let assets = std::env::current_dir()
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("assets");
        let mut target_dir: PathBuf = std::env::var_os("CARGO_MANIFEST_DIR").unwrap().into();
        target_dir = target_dir.parent().unwrap().parent().unwrap().into();
        target_dir.push("target");
        target_dir.push(profile.clone()); // "debug" or "release"
        let output = target_dir.join("assets.dac");

        package(
            &assets,
            &output,
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
