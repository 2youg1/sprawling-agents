// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "test code"
)]

use crate::platform;

use super::contents::Entry;
use super::{ReleaseTarget, cargo_target_dir, run_mei, write_archive};

fn entry(name: &str, source: std::path::PathBuf) -> Entry {
    Entry {
        name: name.to_owned(),
        source,
        mode: 0o644,
    }
}

/// The two names that already exist must not move; a build for a
/// target other than this machine's default earns its triple in the
/// name, so a second Linux artifact cannot collide with the first.
#[test]
fn a_host_archive_keeps_its_name_and_a_target_archive_carries_its_triple() {
    let host = ReleaseTarget::Host.label();
    assert!(host.contains('-'), "host label: {host}");
    assert!(!host.starts_with('-'), "host label: {host}");
    let musl = ReleaseTarget::Triple("x86_64-unknown-linux-musl".to_owned());
    assert_eq!(musl.label(), "x86_64-unknown-linux-musl");
}

/// A `--target` build lands under `<target>/<triple>/release`, and
/// the packager must look there rather than at the host's directory.
#[test]
fn a_target_build_is_looked_for_under_its_own_triple() {
    let target = std::env::temp_dir().join("sprawling-triple");
    let triple = "x86_64-unknown-linux-musl";
    let dirs = [
        ReleaseTarget::Triple(triple.to_owned()),
        ReleaseTarget::Host,
    ]
    .map(|build| build.release_dir_in(&target));
    assert_eq!(
        dirs,
        [target.join(triple).join("release"), target.join("release")]
    );
}

/// A redirected target directory is where the release build landed;
/// `just mem` and `just package` go blind when it is ignored.
#[test]
fn the_target_dir_is_the_one_cargo_target_dir_names() {
    let root = std::env::temp_dir().join("sprawling-root");
    let lane = std::env::temp_dir().join("sprawling-lane");
    let resolved = [
        Some(lane.clone().into_os_string()),
        Some("lane".into()),
        Some("".into()),
        None,
    ]
    .map(|named| cargo_target_dir(&root, named));
    assert_eq!(
        resolved,
        [
            lane,
            root.join("lane"),
            root.join("target"),
            root.join("target")
        ]
    );
}

/// What the archive calls the executable follows the target it was
/// built for, never the machine that assembled it - and a target
/// this release publishes no archive for is refused rather than
/// guessed at.
#[test]
fn the_executable_name_follows_the_target_rather_than_the_host() {
    let musl = ReleaseTarget::Triple("x86_64-unknown-linux-musl".to_owned());
    assert_eq!(musl.binary_name().unwrap(), "sprawling");
    let host = ReleaseTarget::Host;
    assert!(
        host.binary_name().unwrap().starts_with("sprawling"),
        "this machine builds an archive the platform table does not claim"
    );
    let unpublished = ReleaseTarget::Triple("x86_64-pc-windows-gnu".to_owned());
    assert!(unpublished.binary_name().is_err());
}

/// The archive nests under one directory: unpacking it into a folder
/// full of other things must not scatter six files across it.
#[test]
fn every_entry_sits_under_one_directory() {
    let dir = std::env::temp_dir().join("sprawling-package-test");
    std::fs::create_dir_all(&dir).unwrap();
    let source = dir.join("payload.txt");
    std::fs::write(&source, b"payload").unwrap();
    let archive = dir.join("out.zip");
    write_archive(
        &archive,
        "sprawling-9.9.9-test",
        &[entry("QUICKSTART.md", source)],
    )
    .unwrap();

    let bytes = std::fs::read(&archive).unwrap();
    let listing = String::from_utf8_lossy(&bytes);
    assert!(
        listing.contains("sprawling-9.9.9-test/QUICKSTART.md"),
        "entry is not under the archive's own directory"
    );
}

/// Two runs over one tree produce one archive: a release that differs
/// from its own rebuild cannot be checked against its source.
#[test]
fn two_runs_over_the_same_files_produce_the_same_bytes() {
    let dir = std::env::temp_dir().join("sprawling-package-repro");
    std::fs::create_dir_all(&dir).unwrap();
    let source = dir.join("payload.txt");
    std::fs::write(&source, b"payload").unwrap();
    let entries = [entry("QUICKSTART.md", source)];

    let first = dir.join("first.zip");
    let second = dir.join("second.zip");
    write_archive(&first, "sprawling-9.9.9-test", &entries).unwrap();
    write_archive(&second, "sprawling-9.9.9-test", &entries).unwrap();
    assert_eq!(
        std::fs::read(&first).unwrap(),
        std::fs::read(&second).unwrap()
    );
}

#[test]
fn mei_archive_carries_its_posix_binary_guide_and_licence() {
    let root = std::env::temp_dir().join(format!("sprawling-mei-package-{}", std::process::id()));
    if root.exists() {
        std::fs::remove_dir_all(&root).unwrap();
    }
    let triple = "x86_64-unknown-linux-musl";
    let release = root.join("target").join(triple).join("release");
    std::fs::create_dir_all(&release).unwrap();
    std::fs::create_dir_all(root.join("Mei")).unwrap();
    std::fs::write(
        root.join("Cargo.toml"),
        "[workspace.package]\nversion = '0.0.17'\n",
    )
    .unwrap();
    std::fs::write(root.join("Mei/README.md"), b"guide").unwrap();
    std::fs::write(root.join("LICENSE"), b"licence").unwrap();
    std::fs::write(release.join("mei"), b"Mei binary").unwrap();
    std::fs::write(release.join("calyx"), b"Calyx binary").unwrap();

    let target = ReleaseTarget::Triple(triple.to_owned());
    run_mei(&root, &target).unwrap();
    let stem = format!("mei-0.0.17-{triple}");
    let archive = root.join("target/package").join(format!("{stem}.zip"));
    let mut zip = zip::ZipArchive::new(std::fs::File::open(archive).unwrap()).unwrap();
    let names: Vec<String> = (0..zip.len())
        .map(|index| zip.by_index(index).unwrap().name().unwrap().into_owned())
        .collect();
    assert_eq!(
        names,
        [
            format!("{stem}/mei"),
            format!("{stem}/calyx"),
            format!("{stem}/README.md"),
            format!("{stem}/LICENSE")
        ]
    );
    assert_eq!(zip.by_index(0).unwrap().unix_mode().unwrap() & 0o111, 0o111);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn mei_names_the_windows_binary_mei_exe() {
    let row = platform::with_suffix("-windows-x86_64.zip").unwrap();
    assert_eq!(
        platform::mei_package(row),
        Some("@sprawling/mei-windows-x64")
    );
    assert_eq!(platform::mei_binary(row), "mei.exe");
    assert_eq!(platform::calyx_binary(row), "calyx.exe");
}
