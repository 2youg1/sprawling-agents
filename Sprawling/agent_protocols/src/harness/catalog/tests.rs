// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A recorded CDN index and the shipped snapshot, read as catalogs.

#![allow(clippy::unwrap_used, clippy::indexing_slicing, reason = "test code")]

use super::*;
use crate::harness::entry::Pin;
use crate::harness::roster::OFFICIAL;

/// Seven agents of the index as the CDN served it, with authors'
/// addresses, icons and binary download fields removed: three
/// distribution kinds, `args` and `env` on a package, two binaries.
const RECORDED: &str = include_str!("recorded-index.json");

#[test]
fn a_recorded_index_reads_every_distribution_kind_into_a_launch() {
    let catalog = Catalog::read(RECORDED, "d".to_owned(), "e".to_owned()).unwrap();
    let launch = |id: &str| catalog.find(id).map(|entry| entry.launch.clone());
    assert_eq!(
        (launch("grok-build"), launch("fast-agent")),
        (
            Some(Launch {
                program: Launcher::Npx.program().to_owned(),
                args: ["-y", "@xai-official/grok@1.0.50", "agent", "stdio"]
                    .map(str::to_owned)
                    .to_vec(),
                env: Vec::new(),
            }),
            Some(Launch {
                program: "uvx".to_owned(),
                args: ["fast-agent-acp==0.10.1", "-x"].map(str::to_owned).to_vec(),
                env: vec![("FAST_AGENT_MODEL".to_owned(), "codexplan".to_owned())],
            }),
        )
    );
    let kimi = catalog.find("kimi").unwrap();
    assert_eq!(
        (
            kimi.launch.program.trim_end_matches(".exe"),
            kimi.launch.args.as_slice(),
            kimi.launch.pin()
        ),
        ("kimi", ["acp".to_owned()].as_slice(), Pin::Unknown)
    );
    assert_eq!(
        (
            catalog.find("claude-acp").unwrap().licence.as_deref(),
            catalog.entries.len()
        ),
        (Some("proprietary"), 7)
    );
}

/// Every built-in entry is in the shipped snapshot, and each one fetched
/// as a package names one exact version.
#[test]
fn the_shipped_snapshot_holds_every_builtin_entry_pinned() {
    let catalog = Catalog::bundled().unwrap();
    assert!(!catalog.date.is_empty() && !catalog.etag.is_empty());
    for official in OFFICIAL {
        let entry = catalog.find(official.registry_id).unwrap();
        assert_ne!(entry.launch.pin(), Pin::Floating, "{}", official.word);
    }
}
