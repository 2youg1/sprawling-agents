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
    // Which packages the shim knows is `platform::restated`'s question,
    // asked of the file on disk; asserting it here too would be a
    // second reader of one fact.
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
    let row = &PLATFORMS[0];
    let archive = assets.join(format!("sprawling-0.0.10{}", row.suffix));
    let mut zip = zip::ZipWriter::new(std::fs::File::create(archive).unwrap());
    zip.start_file(
        format!("archive/{}", row.binary),
        zip::write::SimpleFileOptions::default().unix_permissions(0o755),
    )
    .unwrap();
    zip.write_all(b"archive binary bytes").unwrap();
    zip.finish().unwrap();
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
    for row in PLATFORMS.iter().filter(|row| row.os != "win32") {
        let archive = assets.join(format!("sprawling-0.0.10{}", row.suffix));
        let mut zip = zip::ZipWriter::new(std::fs::File::create(archive).unwrap());
        zip.start_file(
            format!("application/{}", row.binary),
            zip::write::SimpleFileOptions::default().unix_permissions(0o755),
        )
        .unwrap();
        zip.write_all(b"binary").unwrap();
        zip.finish().unwrap();
    }
    let out = fixture.join("out");
    super::run(&fixture, "v0.0.10-Alpha-261005", &assets, &out).unwrap();
    assert!(
        out.join("homebrew/sprawling.rb").is_file(),
        "the Homebrew formula must be generated"
    );
    let formula = std::fs::read_to_string(out.join("homebrew/sprawling.rb")).unwrap();
    let pkgbuild = std::fs::read_to_string(out.join("aur/PKGBUILD")).unwrap();
    let srcinfo = std::fs::read_to_string(out.join("aur/.SRCINFO")).unwrap();
    assert!(formula.contains("on_macos") && formula.contains("on_linux"));
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
    assert!(formula.contains(digest) && pkgbuild.contains(digest));
    assert!(formula.contains("/releases/download/v0.0.10-Alpha-261005/"));
    std::fs::remove_dir_all(fixture).unwrap();
}
