use crate::{
    BuildPlatform,
    build::{self, Profile},
    command,
    dependencies::{self, OpenXRLoadersSelection},
};
use alvr_filesystem as afs;
use std::{fs, path::Path};
use xshell::{Shell, cmd};

pub enum ReleaseFlavor {
    GitHub,
    MetaStore,
    PicoStore,
}

pub fn include_licenses(root_path: &Path) {
    let sh = Shell::new().unwrap();

    // Add licenses
    let licenses_dir = root_path.join("licenses");
    sh.create_dir(&licenses_dir).unwrap();
    sh.copy_file(
        afs::workspace_dir().join("LICENSE"),
        licenses_dir.join("ALVR.txt"),
    )
    .unwrap();
    sh.copy_file(
        afs::crate_dir("server_openvr").join("LICENSE-Valve"),
        licenses_dir.join("Valve.txt"),
    )
    .unwrap();

    // Gather licenses with cargo about
    cmd!(sh, "cargo install cargo-about --version 0.6.4")
        .run()
        .unwrap();
    let licenses_template = afs::crate_dir("xtask").join("licenses_template.hbs");
    let licenses_content = cmd!(sh, "cargo about generate {licenses_template}")
        .read()
        .unwrap();
    sh.write_file(licenses_dir.join("dependencies.html"), licenses_content)
        .unwrap();
}

pub fn package_streamer(
    platform: Option<BuildPlatform>,
    enable_nvenc: bool,
    gpl: bool,
    root: Option<String>,
) {
    let sh = Shell::new().unwrap();

    dependencies::prepare_server_deps(platform, enable_nvenc);

    build::build_streamer(
        Profile::Distribution,
        gpl,
        root,
        crate::CommonBuildFlags {
            locked: true,
            ..Default::default()
        },
        false,
    );

    include_licenses(&afs::streamer_build_dir());

    command::targz(&sh, &afs::streamer_build_dir()).unwrap();
}

pub fn package_launcher() {
    let sh = Shell::new().unwrap();

    sh.remove_path(afs::launcher_build_dir()).ok();

    build::build_launcher(
        Profile::Distribution,
        crate::CommonBuildFlags {
            locked: true,
            ..Default::default()
        },
    );

    include_licenses(&afs::launcher_build_dir());

    command::targz(&sh, &afs::launcher_build_dir()).unwrap();
}

pub fn replace_client_openxr_manifest(from_pattern: &str, to: &str) {
    let manifest_path = afs::crate_dir("client_openxr").join("Cargo.toml");
    let manifest_string = fs::read_to_string(&manifest_path)
        .unwrap()
        .replace(from_pattern, to);

    fs::write(manifest_path, manifest_string).unwrap();
}

pub fn package_client_openxr(flavor: ReleaseFlavor) {
    fs::remove_dir_all(afs::deps_dir().join("android_openxr")).ok();

    let openxr_selection = match flavor {
        ReleaseFlavor::GitHub => OpenXRLoadersSelection::All,
        ReleaseFlavor::MetaStore => OpenXRLoadersSelection::OnlyGeneric,
        ReleaseFlavor::PicoStore => OpenXRLoadersSelection::OnlyPico,
    };

    dependencies::android::build_deps(false, openxr_selection);

    if !matches!(flavor, ReleaseFlavor::GitHub) {
        replace_client_openxr_manifest(
            r#"package = "alvr.client.stable""#,
            r#"package = "alvr.client""#,
        );
    }

    if matches!(flavor, ReleaseFlavor::MetaStore) {
        replace_client_openxr_manifest(r#"value = "all""#, r#"value = "quest2|questpro|quest3""#);
    }

    build::build_android_client(Profile::Distribution);
}

pub fn package_client_lib(link_stdcpp: bool, all_targets: bool) {
    let sh = Shell::new().unwrap();

    build::build_android_client_core_lib(Profile::Distribution, link_stdcpp, all_targets);

    command::zip(&sh, &afs::build_dir().join("client_core")).unwrap();
}
