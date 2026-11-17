// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use crate::platform::{PLATFORMS, ROOT_PACKAGE};

/// The last version whose platform packages reached the registry under
/// bare names. npm never reuses a `name@version`, so the scope cannot
/// be applied to it afterwards.
const UNSCOPED_THROUGH: &str = "0.0.4";

/// What this channel publishes a release as, held against the spelling
/// `kernel::Release` decodes - the tag conversion itself is that type's
/// and is tested there. This is the seam: the job publishes
/// `npm_version()`, and `status --check` reads the registry back through
/// `from_npm_version`, so a release only stays findable while those two
/// agree.
#[test]
fn the_published_version_is_the_one_a_running_binary_decodes() {
    // The tag this tree would cut, so moving `MATURITY` leaves it true.
    let tag = format!("v0.0.4-{}-260911", kernel::release::MATURITY.titled());
    let cut = kernel::Release::from_tag(&tag, "0.0.4").expect("a release tag of this project");
    assert_eq!(cut.npm_version(), "0.0.4-pre.260911");
    assert_eq!(
        kernel::Release::from_npm_version(&cut.npm_version()).expect("what this channel published"),
        cut
    );
}

/// The decision beside `ROWS`, carried to the version that can honour
/// it. Nothing here fires while the workspace still states the version
/// whose packages are already published; the first commit that moves
/// that number turns this red, and the message says what to rename.
#[test]
fn the_platform_packages_take_the_scope_from_the_next_version() {
    let root = crate::root::this_checkout();
    let version =
        crate::package::workspace_package(root, "version").expect("the workspace states a version");
    if version == UNSCOPED_THROUGH {
        return;
    }
    assert!(
        !ROOT_PACKAGE.contains('@') && !ROOT_PACKAGE.contains('/'),
        "the root package keeps its bare name, because `bunx {ROOT_PACKAGE}` is \
         what this channel exists for"
    );
    let scope = format!("@{ROOT_PACKAGE}/");
    for row in PLATFORMS {
        assert!(
            row.package.starts_with(&scope),
            "the workspace states {version}, which is past {UNSCOPED_THROUGH}, \
             so {} belongs under the {scope} scope. Rename it here and in \
             the shim's table.",
            row.package
        );
    }
}

/// The shim is what every root package carries, so its own contract
/// is worth holding: it resolves a platform package and never
/// installs anything.
#[test]
fn the_shim_execs_rather_than_installs() {
    let shim = super::SHIM;
    assert!(shim.contains("require.resolve"), "the shim must resolve");
    assert!(
        !shim.contains("sprawling install"),
        "PATH belongs to whoever installed; the shim must not install"
    );
}

#[test]
#[expect(clippy::disallowed_methods, reason = "developer tool (child D4)")]
fn mei_shim_smoke_routes_tty_to_calyx_and_headless_flags_to_core() {
    let shim = super::SHIM;
    let start = shim
        .find("function wantsHeadless")
        .expect("Calyx D4 lives in wantsHeadless");
    let rest = shim
        .get(start..)
        .expect("wantsHeadless starts on a char boundary");
    let end = rest
        .find("\n}\n")
        .expect("wantsHeadless is a closed function");
    let function = rest
        .get(..=end + 1)
        .expect("wantsHeadless ends on a char boundary");
    assert!(
        function.contains("\"-p\"")
            && function.contains("--resume")
            && function.contains("stdinIsTTY"),
        "{function}"
    );
    assert!(shim.contains("childEnv.MEI_BIN = core"));
    assert!(shim.contains("row.face"));

    let script = format!(
        "{function}\n\
         const cases = [\n\
           [['-p', 'x'], true, true],\n\
           [['--resume', 'a.jsonl'], true, true],\n\
           [['--resume=a.jsonl'], true, true],\n\
           [[], true, false],\n\
           [[], false, true],\n\
           [['--replay', 'a.jsonl'], true, false],\n\
         ];\n\
         for (const [argv, tty, want] of cases) {{\n\
           const got = wantsHeadless(argv, tty);\n\
           if (got !== want) {{\n\
             console.error(JSON.stringify({{ argv, tty, got, want }}));\n\
             process.exit(1);\n\
           }}\n\
         }}\n"
    );
    let status = std::process::Command::new("node")
        .arg("-e")
        .arg(script)
        .status()
        .expect("node runs the shim smoke matrix");
    assert!(status.success(), "Mei shim routing smoke failed: {status}");
}

#[test]
fn channel_packages_preserve_the_archive_binary_and_dispatch_both_runtimes() {
    use std::io::Write as _;
    let fixture = std::env::temp_dir().join(format!("sprawling-channel-{}", std::process::id()));
    let assets = fixture.join("assets");
    std::fs::create_dir_all(&assets).unwrap();
    std::fs::write(
        fixture.join("Cargo.toml"),
        "[workspace.package]
version = '0.0.10'
repository = 'https://example.invalid/source'
description = 'fixture description'
",
    )
    .unwrap();
    std::fs::create_dir_all(fixture.join("Mei")).unwrap();
    std::fs::write(
        fixture.join("Mei/Cargo.toml"),
        "[package]\ndescription = 'Archetype:Mei fixture'\n",
    )
    .unwrap();
    let row = &PLATFORMS[0];
    let archive = assets.join(format!("sprawling-0.0.10{}", row.suffix));
    let mut zip = zip::ZipWriter::new(std::fs::File::create(archive).unwrap());
    zip.start_file(
        format!("archive/{}", row.binary),
        zip::write::SimpleFileOptions::default()
            .system(zip::System::Unix)
            .unix_permissions(0o755),
    )
    .unwrap();
    zip.write_all(b"archive binary bytes").unwrap();
    zip.finish().unwrap();
    for row in PLATFORMS.iter().filter(|row| row.os != "win32") {
        let archive = assets.join(format!("mei-0.0.10{}", row.suffix));
        let mut zip = zip::ZipWriter::new(std::fs::File::create(archive).unwrap());
        zip.start_file(
            "archive/mei",
            zip::write::SimpleFileOptions::default()
                .system(zip::System::Unix)
                .unix_permissions(0o755),
        )
        .unwrap();
        zip.write_all(b"Mei archive binary bytes").unwrap();
        zip.start_file(
            "archive/calyx",
            zip::write::SimpleFileOptions::default()
                .system(zip::System::Unix)
                .unix_permissions(0o755),
        )
        .unwrap();
        zip.write_all(b"Calyx archive binary bytes").unwrap();
        zip.finish().unwrap();
    }
    let out = fixture.join("npm");
    super::run(&fixture, "v0.0.10-Alpha-261005", &assets, &out).unwrap();
    let manifest: serde_json::Value = serde_json::from_slice(
        &std::fs::read(out.join(ROOT_PACKAGE).join("package.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(manifest["bin"]["sprawling"], "bin/sprawling.js");
    assert_eq!(manifest["bin"][super::RUNTIME_ENTRY], "bin/sprawling.cmd");
    assert_eq!(manifest["description"], "fixture description");
    assert_eq!(
        std::fs::read(out.join(row.package).join("bin").join(row.binary)).unwrap(),
        b"archive binary bytes"
    );
    assert_eq!(
        std::fs::read_to_string(out.join(ROOT_PACKAGE).join("bin/sprawling.cmd")).unwrap(),
        super::LAUNCHER
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        assert_eq!(
            std::fs::metadata(out.join(row.package).join("bin").join(row.binary))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o755
        );
    }
    let mei_root = out.join("@sprawling/mei");
    let mei_manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(mei_root.join("package.json")).unwrap()).unwrap();
    assert_eq!(mei_manifest["bin"]["mei"], "bin/mei.js");
    assert_eq!(mei_manifest["bin"].as_object().unwrap().len(), 1);
    assert!(
        std::fs::read_to_string(mei_root.join("bin/mei.js"))
            .unwrap()
            .starts_with("#!/usr/bin/env bun\n")
    );
    assert!(!mei_root.join("bin/mei.cmd").exists());
    assert_eq!(
        mei_manifest["optionalDependencies"]
            .as_object()
            .unwrap()
            .len(),
        2
    );
    for row in PLATFORMS.iter().filter(|row| row.os != "win32") {
        let package = super::mei_package(row).expect("a POSIX row has a Mei package");
        assert_eq!(
            std::fs::read(out.join(package).join("bin/mei")).unwrap(),
            b"Mei archive binary bytes"
        );
        assert_eq!(
            std::fs::read(out.join(package).join("bin/calyx")).unwrap(),
            b"Calyx archive binary bytes"
        );
    }
    std::fs::remove_dir_all(fixture).unwrap();
}

#[test]
fn channel_requires_mei_archives_with_the_platform_binary_name() {
    use std::io::Write as _;
    let fixture =
        std::env::temp_dir().join(format!("sprawling-mei-channel-{}", std::process::id()));
    if fixture.exists() {
        std::fs::remove_dir_all(&fixture).unwrap();
    }
    let assets = fixture.join("assets");
    std::fs::create_dir_all(&assets).unwrap();
    std::fs::create_dir_all(fixture.join("Mei")).unwrap();
    std::fs::write(
        fixture.join("Cargo.toml"),
        "[workspace.package]\nversion = '0.0.10'\nrepository = 'https://example.invalid/source'\ndescription = 'fixture'\n",
    )
    .unwrap();
    std::fs::write(
        fixture.join("Mei/Cargo.toml"),
        "[package]\ndescription = 'Mei fixture'\n",
    )
    .unwrap();
    let row = &PLATFORMS[0];
    let archive = assets.join(format!("sprawling-0.0.10{}", row.suffix));
    let mut zip = zip::ZipWriter::new(std::fs::File::create(archive).unwrap());
    zip.start_file(
        format!("archive/{}", row.binary),
        zip::write::SimpleFileOptions::default()
            .system(zip::System::Unix)
            .unix_permissions(0o755),
    )
    .unwrap();
    zip.write_all(b"sprawling").unwrap();
    zip.finish().unwrap();

    let missing = super::run(
        &fixture,
        "v0.0.10-Alpha-261005",
        &assets,
        &fixture.join("missing"),
    )
    .expect_err("a release must not silently omit Mei's npm channel");
    assert!(missing.to_string().contains("no Mei release archive"));

    let archive = assets.join(format!("mei-0.0.10{}", row.suffix));
    let mut zip = zip::ZipWriter::new(std::fs::File::create(archive).unwrap());
    zip.start_file(
        "archive/mei",
        zip::write::SimpleFileOptions::default()
            .system(zip::System::Unix)
            .unix_permissions(0o755),
    )
    .unwrap();
    zip.write_all(b"mei").unwrap();
    zip.finish().unwrap();
    let wrong_name = super::run(
        &fixture,
        "v0.0.10-Alpha-261005",
        &assets,
        &fixture.join("wrong-name"),
    )
    .expect_err("a Windows Mei archive must hold mei.exe");
    assert!(
        wrong_name
            .to_string()
            .contains("holds no file named mei.exe"),
        "{wrong_name}"
    );
    std::fs::remove_dir_all(fixture).unwrap();
}

#[test]
fn system_channels_share_the_archive_digest_and_release_identity() {
    use std::io::Write as _;
    let fixture =
        std::env::temp_dir().join(format!("sprawling-system-channel-{}", std::process::id()));
    let assets = fixture.join("assets");
    std::fs::create_dir_all(&assets).unwrap();
    std::fs::write(
        fixture.join("Cargo.toml"),
        "[workspace.package]
version = '0.0.10'
repository = 'https://example.invalid/source'
description = 'fixture'
",
    )
    .unwrap();
    std::fs::create_dir_all(fixture.join("Mei")).unwrap();
    std::fs::write(
        fixture.join("Mei/Cargo.toml"),
        "[package]\ndescription = 'Archetype:Mei fixture'\n",
    )
    .unwrap();
    for row in PLATFORMS.iter().filter(|row| row.os != "win32") {
        let archive = assets.join(format!("sprawling-0.0.10{}", row.suffix));
        let mut zip = zip::ZipWriter::new(std::fs::File::create(archive).unwrap());
        zip.start_file(
            format!("application/{}", row.binary),
            zip::write::SimpleFileOptions::default()
                .system(zip::System::Unix)
                .unix_permissions(0o755),
        )
        .unwrap();
        zip.write_all(b"binary").unwrap();
        zip.finish().unwrap();
    }
    for row in PLATFORMS.iter().filter(|row| row.os != "win32") {
        let archive = assets.join(format!("mei-0.0.10{}", row.suffix));
        let mut zip = zip::ZipWriter::new(std::fs::File::create(archive).unwrap());
        zip.start_file(
            "application/mei",
            zip::write::SimpleFileOptions::default()
                .system(zip::System::Unix)
                .unix_permissions(0o755),
        )
        .unwrap();
        zip.write_all(b"Mei binary").unwrap();
        zip.start_file(
            "application/calyx",
            zip::write::SimpleFileOptions::default()
                .system(zip::System::Unix)
                .unix_permissions(0o755),
        )
        .unwrap();
        zip.write_all(b"Calyx binary").unwrap();
        zip.finish().unwrap();
    }
    let out = fixture.join("out");
    super::run(&fixture, "v0.0.10-Alpha-261005", &assets, &out).unwrap();
    let pkgbuild = std::fs::read_to_string(out.join("aur/PKGBUILD")).unwrap();
    let srcinfo = std::fs::read_to_string(out.join("aur/.SRCINFO")).unwrap();
    assert!(pkgbuild.contains("pkgver=0.0.10_pre.261005"));
    let executable = pkgbuild
        .lines()
        .find_map(|line| line.strip_prefix("  ln -s \""))
        .unwrap()
        .split('"')
        .next()
        .unwrap();
    assert!(kernel::release::is_aur_install(std::path::Path::new(
        executable
    )));
    let package = pkgbuild
        .lines()
        .find_map(|line| line.strip_prefix("pkgname="))
        .unwrap();
    assert!(
        srcinfo
            .lines()
            .any(|line| line == format!("pkgname = {package}"))
    );
    let digest = srcinfo
        .lines()
        .find_map(|line| line.trim().strip_prefix("sha256sums = "))
        .unwrap();
    assert!(pkgbuild.contains(digest));
    assert!(pkgbuild.contains("/releases/download/v0.0.10-Alpha-261005/"));
    std::fs::remove_dir_all(fixture).unwrap();
}
