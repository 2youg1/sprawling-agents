// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::wildcard_enum_match_arm,
    reason = "test code"
)]

use super::{Built, Registries, built, channel, judged};
use kernel::ReleaseVerdict;
use std::io::{BufRead, BufReader, Write};
use wire::{InstallChannel, ReleaseAnswer};

/// The source this build was cut from, and the newer one published
/// after it; a nightly test compares the two through GitHub.
const INSTALLED_SHA: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const NEWEST_SHA: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

/// A published nightly newer than the installed one, followed by the
/// curated release both curated queries read, so one fixture shows the
/// curated answer skipping nightlies.
const PUBLISHED: &str = r#"[{"tag_name":"nightly-261106-bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","draft":false,"published_at":"2026-11-06T00:00:00Z","target_commitish":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"},{"tag_name":"v0.0.8-Alpha-261006","draft":false}]"#;
/// A loopback stand-in for both registries: `/npm` answers npm's
/// manifest, `/crates` crates.io's, `/github` the release list the
/// caller supplies, every request closes its connection, and the
/// User-Agent of each request is sent back to the test.
fn registries(
    npm: &'static str,
    crates: &'static str,
    published: &'static str,
) -> (String, std::sync::mpsc::Receiver<String>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let (agents, seen) = std::sync::mpsc::channel();
    let compared = format!("GET /compare/{NEWEST_SHA}...{INSTALLED_SHA} ");
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = stream.unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut head = Vec::new();
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if line.trim().is_empty() {
                    break;
                }
                head.push(line);
            }
            let body = if head[0].contains("/github") {
                published
            } else if head[0].contains("/compare") {
                // The path is the answer: the newest snapshot is the
                // base and this build's source the head, so `behind`
                // says the installed nightly is older. Any other
                // operand or order is `diverged`, which the caller
                // reads as unconfirmed rather than as an order.
                if head[0].starts_with(&compared) {
                    r#"{"status":"behind"}"#
                } else {
                    r#"{"status":"diverged"}"#
                }
            } else if head[0].contains("/crates") {
                crates
            } else {
                npm
            };
            let agent = head
                .iter()
                .find_map(|line| {
                    line.to_ascii_lowercase()
                        .strip_prefix("user-agent:")
                        .map(|it| it.trim().to_owned())
                })
                .unwrap_or_default();
            agents.send(agent).unwrap();
            write!(
                stream,
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
        }
    });
    (base, seen)
}

/// npm carries a re-cut of 0.0.8 with a later date, which crates.io
/// cannot carry: a binary cargo installed is current against the
/// crates.io answer of its own release, and one npm installed is told
/// it is behind.
#[test]
fn a_crates_answer_of_the_same_release_reads_as_current() {
    let (base, seen) = registries(
        r#"{"version":"0.0.8-pre.261005"}"#,
        r#"{"crate":{"max_version":"0.0.8"}}"#,
        PUBLISHED,
    );
    let npm = format!("{base}/npm");
    let crates = format!("{base}/crates");
    let at = Registries {
        npm: &npm,
        crates: &crates,
        github: &format!("{base}/github"),
        compare: &format!("{base}/compare"),
    };
    let mine = || Built::Released(kernel::Release::from_npm_version("0.0.8-pre.261002").unwrap());
    let verdict = |answer: ReleaseAnswer| match answer {
        ReleaseAnswer::Stands {
            verdict,
            newest,
            update,
            ..
        } => (verdict, newest.version, update.command),
        other => panic!("not judged: {other:?}"),
    };
    assert_eq!(
        [
            verdict(judged(mine(), InstallChannel::Cargo, &at)),
            verdict(judged(mine(), InstallChannel::Npm, &at)),
        ],
        [
            (
                ReleaseVerdict::Current,
                "0.0.8".to_owned(),
                Some(super::CARGO_UPDATE.to_owned())
            ),
            (
                ReleaseVerdict::Behind,
                "0.0.8-pre.261005".to_owned(),
                Some(super::NPM_UPDATE.to_owned())
            ),
        ]
    );
    let agents: Vec<String> = seen.try_iter().collect();
    assert_eq!(agents.len(), 6);
    assert!(
        agents.iter().all(|agent| agent.starts_with("sprawling/")),
        "{agents:?}"
    );
}

#[test]
fn each_origin_selects_its_own_registry_and_never_switches_updaters() {
    let (base, _seen) = registries(
        r#"{"version":"0.0.9-pre.261005"}"#,
        r#"{"crate":{"max_version":"1.0.0"}}"#,
        PUBLISHED,
    );
    let at = Registries {
        npm: &format!("{base}/npm"),
        crates: &format!("{base}/crates"),
        github: &format!("{base}/github"),
        compare: &format!("{base}/compare"),
    };
    let mine = || Built::Released(kernel::Release::from_npm_version("0.0.8-pre.261002").unwrap());
    let read = |origin| match judged(mine(), origin, &at) {
        ReleaseAnswer::Stands { newest, update, .. } => {
            (newest.version, update.command, update.alternatives)
        }
        other => panic!("not judged: {other:?}"),
    };
    assert_eq!(
        [
            InstallChannel::Npm,
            InstallChannel::Bun,
            InstallChannel::Binstall,
            InstallChannel::Archive,
            InstallChannel::CargoOrBinstall,
            InstallChannel::Package,
            InstallChannel::Aur
        ]
        .map(read),
        [
            (
                "0.0.9-pre.261005".to_owned(),
                Some(super::NPM_UPDATE.to_owned()),
                vec![]
            ),
            (
                "0.0.9-pre.261005".to_owned(),
                Some(super::BUN_UPDATE.to_owned()),
                vec![]
            ),
            (
                "1.0.0".to_owned(),
                Some(super::BINSTALL_UPDATE.to_owned()),
                vec![]
            ),
            (
                "0.0.8-pre.261006".to_owned(),
                Some(super::archive_command(super::ArchiveVersion::Tag(
                    "v0.0.8-Alpha-261006"
                ))),
                vec![]
            ),
            (
                "1.0.0".to_owned(),
                None,
                vec![
                    super::CARGO_UPDATE.to_owned(),
                    super::BINSTALL_UPDATE.to_owned()
                ]
            ),
            (
                "0.0.9-pre.261005".to_owned(),
                None,
                vec![super::NPM_UPDATE.to_owned(), super::BUN_UPDATE.to_owned()]
            ),
            (
                "0.0.8-pre.261006".to_owned(),
                Some(super::AUR_UPDATE.to_owned()),
                vec![]
            ),
        ]
    );
    assert!(matches!(
        judged(mine(), InstallChannel::Unknown, &at),
        ReleaseAnswer::Unconfirmed {
            update: wire::UpdateHint { command: None, .. },
            ..
        }
    ));
    assert!(
        matches!(judged(Built::FromSource, InstallChannel::CargoOrBinstall, &at), ReleaseAnswer::Stands { newest, mine, .. } if newest.version == "1.0.0" && mine.released.is_empty())
    );
}

#[test]
fn a_nightly_archive_compares_complete_source_and_keeps_its_channel() {
    let (base, _seen) = registries(
        r#"{"version":"0.0.9-pre.261005"}"#,
        r#"{"crate":{"max_version":"1.0.0"}}"#,
        PUBLISHED,
    );
    let at = Registries {
        npm: &format!("{base}/npm"),
        crates: &format!("{base}/crates"),
        github: &format!("{base}/github"),
        compare: &format!("{base}/compare"),
    };
    let mine = Built::Nightly(
        super::NightlyBuild::new(&format!("nightly-261105-{INSTALLED_SHA}"), INSTALLED_SHA)
            .unwrap(),
    );
    let newest =
        super::NightlyBuild::new(&format!("nightly-261106-{NEWEST_SHA}"), NEWEST_SHA).unwrap();
    match judged(mine, InstallChannel::Archive, &at) {
        ReleaseAnswer::Stands {
            verdict,
            newest: found,
            update,
            ..
        } => assert_eq!(
            (verdict, found.version, update.command),
            (
                ReleaseVerdict::Behind,
                format!("nightly-261106-{NEWEST_SHA}"),
                Some(super::archive_command(super::ArchiveVersion::Nightly(
                    &newest
                )))
            )
        ),
        other => panic!("not judged: {other:?}"),
    }
}

/// An unreadable identity is not the newest snapshot: a malformed newer
/// entry must not hide the version a person can install.
#[test]
fn an_unreadable_nightly_does_not_hide_a_valid_older_one() {
    let (base, _seen) = registries(
        r#"{"version":"0.0.9-pre.261005"}"#,
        r#"{"crate":{"max_version":"1.0.0"}}"#,
        r#"[{"tag_name":"nightly-261107-bc930372","draft":false,"published_at":"2026-11-07T00:00:00Z","target_commitish":"bc930372"},{"tag_name":"nightly-261106-bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","draft":false,"published_at":"2026-11-06T00:00:00Z","target_commitish":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}]"#,
    );
    let at = Registries {
        npm: &format!("{base}/npm"),
        crates: &format!("{base}/crates"),
        github: &format!("{base}/github"),
        compare: &format!("{base}/compare"),
    };
    let mine = Built::Nightly(
        super::NightlyBuild::new(&format!("nightly-261105-{INSTALLED_SHA}"), INSTALLED_SHA)
            .unwrap(),
    );
    match judged(mine, InstallChannel::Archive, &at) {
        ReleaseAnswer::Stands { newest, .. } => {
            assert_eq!(newest.version, format!("nightly-261106-{NEWEST_SHA}"));
        }
        other => panic!("not judged: {other:?}"),
    }
}

/// The four channels, read from fixture paths rather than from where
/// this test binary happens to run. Each OS spells its paths its own
/// way; the rule reads components, so one fixture set covers all three.
#[test]
fn the_update_command_follows_the_channel_the_binary_came_through() {
    let root = tempfile::tempdir().unwrap();
    let cargo_bin = root.path().join(".cargo").join("bin");
    let archive = root.path().join("sprawling-0.0.9");
    std::fs::create_dir_all(archive.join("skills")).unwrap();
    let exe = |dir: &std::path::Path| dir.join("sprawling");
    let npm = root
        .path()
        .join("lib")
        .join("node_modules")
        .join("sprawling")
        .join("bin");
    let bunx = root.path().join(".bun").join("install").join("cache");
    assert_eq!(
        [
            channel(&exe(&npm), Some(&cargo_bin)),
            channel(&exe(&bunx), Some(&cargo_bin)),
            channel(&exe(&cargo_bin), Some(&cargo_bin)),
            channel(&exe(&archive), Some(&cargo_bin)),
            channel(&exe(&root.path().join("target")), Some(&cargo_bin)),
            channel(&exe(&cargo_bin), None),
        ],
        [
            InstallChannel::Package,
            InstallChannel::Package,
            InstallChannel::CargoOrBinstall,
            InstallChannel::Archive,
            InstallChannel::Source,
            InstallChannel::Source,
        ]
    );
}

#[test]
fn system_package_paths_select_their_own_update_guidance() {
    let root = tempfile::tempdir().unwrap();
    let aur = root
        .path()
        .join(kernel::release::aur_install_directory())
        .join("sprawling");
    assert_eq!(channel(&aur, None), InstallChannel::Aur);
}

/// The check this test exists for is not which state a test binary
/// is in, but that reading it never panics and never invents a
/// release: `cargo test` sets no tag, and a build that did set one
/// must decode it rather than carry it as text.
#[test]
fn a_binary_knows_whether_it_came_out_of_a_release() {
    match built().expect("this build's own tag decodes") {
        Built::FromSource => {}
        Built::Released(mine) => {
            assert_eq!(mine.version(), env!("CARGO_PKG_VERSION"));
            assert_eq!(
                kernel::Release::from_npm_version(&mine.npm_version())
                    .expect("what this release publishes"),
                mine
            );
        }
        Built::Nightly(mine) => {
            assert!(mine.tag().starts_with("nightly-"));
            assert_eq!(mine.source_sha().len(), 40);
        }
    }
}
