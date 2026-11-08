// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use std::collections::BTreeSet;

use kernel::Address;
use runtime::RuleCandidate;

use super::{RuleDelivery, RuleScope, nested_rules};

fn at(spelled: &str) -> Address {
    Address::parse(spelled).unwrap()
}

fn write(root: &std::path::Path, relative: &str, bytes: &[u8]) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
}

fn project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "lab/.sprawling/RULES.toml",
        b"confidential = false\nwrite = \"everything\"\n",
    );
    write(dir.path(), "lab/pkg/AGENTS.md", b"package rules\n");
    write(dir.path(), "lab/pkg/probe/AGENTS.md", b"probe rules\n");
    write(
        dir.path(),
        "lab/run/AGENTS.md",
        b"the run room is already frozen\n",
    );
    write(dir.path(), "lab/pkg/probe/a.rs", b"pub fn a() {}\n");
    write(dir.path(), "lab/pkg/probe/b.rs", b"pub fn b() {}\n");
    write(dir.path(), "lab/a.rs", b"pub fn shallow() {}\n");
    dir
}

fn scope(root: &std::path::Path) -> RuleScope {
    RuleScope {
        city_root: root.to_path_buf(),
        building: at("lab"),
        room: at("lab"),
    }
}

fn candidate(room: &str, text: String) -> RuleCandidate {
    RuleCandidate {
        room: at(room),
        text,
        dropped: 0,
    }
}

#[test]
fn a_deeper_directory_offers_its_rules_from_the_run_room_downwards() {
    let dir = project();
    let candidates = nested_rules(
        scope(dir.path()),
        &at("lab/pkg/probe/a.rs"),
        &at("lab/pkg/probe/a.rs"),
    );
    assert_eq!(
        candidates
            .into_iter()
            .map(|candidate| (candidate.room, candidate.text))
            .collect::<Vec<_>>(),
        vec![
            (at("lab/pkg"), "package rules\n".to_owned()),
            (at("lab/pkg/probe"), "probe rules\n".to_owned()),
        ]
    );
}

#[test]
fn only_a_strict_descendant_of_the_run_room_and_home_building_is_eligible() {
    let dir = project();
    for (admitted, landed) in [
        ("lab/a.rs", "lab/a.rs"),
        ("other/pkg/a.rs", "other/pkg/a.rs"),
        ("lab/pkg/probe/a.rs", "other/pkg/a.rs"),
    ] {
        assert!(
            nested_rules(scope(dir.path()), &at(admitted), &at(landed)).is_empty(),
            "{admitted} -> {landed} was eligible"
        );
    }
}

#[test]
fn a_second_file_in_the_same_room_does_not_deliver_the_same_rule_twice() {
    let candidates = vec![candidate("lab/pkg/probe", "probe rules\n".to_owned())];
    let mut delivery = RuleDelivery::default();
    assert_eq!(delivery.answer("first", candidates.clone()), candidates);
    delivery.complete_wave(&BTreeSet::new());
    assert!(delivery.answer("second", candidates).is_empty());
}

#[test]
fn the_first_oversized_rule_is_cut_with_its_marker_and_later_rules_wait() {
    let cap = kernel::consts_policy::INTERVAL_CAP_BYTES;
    let candidates = vec![
        candidate("lab/pkg", "x".repeat(cap.saturating_add(1))),
        candidate("lab/pkg/probe", "later\n".to_owned()),
    ];
    let mut delivery = RuleDelivery::default();
    let selected = delivery.answer("first", candidates);
    assert_eq!(selected.len(), 1);
    assert!(selected[0].text.len() <= cap);
    assert!(selected[0].text.contains("[truncated: "));
    assert!(selected[0].dropped > 0);
}

#[test]
fn an_oversized_rule_file_is_bounded_and_carries_its_omission() {
    let dir = project();
    let cap = kernel::consts_policy::INTERVAL_CAP_BYTES;
    write(
        dir.path(),
        "lab/pkg/probe/AGENTS.md",
        "x".repeat(cap.saturating_add(8_192)).as_bytes(),
    );
    let candidates = nested_rules(
        scope(dir.path()),
        &at("lab/pkg/probe/a.rs"),
        &at("lab/pkg/probe/a.rs"),
    );
    assert_eq!(candidates.len(), 2);
    assert!(candidates[1].text.len() <= cap);
    assert!(candidates[1].dropped > 0);
}

#[test]
fn a_rule_cut_inside_a_multibyte_character_keeps_its_valid_prefix() {
    let dir = project();
    let remaining =
        kernel::consts_policy::INTERVAL_CAP_BYTES.saturating_sub("package rules\n".len());
    let mut bytes = vec![b'x'; remaining.saturating_sub(1)];
    bytes.extend_from_slice("界".as_bytes());
    bytes.extend_from_slice(b"later\n");
    write(dir.path(), "lab/pkg/probe/AGENTS.md", &bytes);

    let candidates = nested_rules(
        scope(dir.path()),
        &at("lab/pkg/probe/a.rs"),
        &at("lab/pkg/probe/a.rs"),
    );
    let probe = candidates
        .iter()
        .find(|candidate| candidate.room == at("lab/pkg/probe"))
        .unwrap();
    assert!(probe.text.len() < remaining);
    assert!(probe.text.ends_with('x'));
    assert!(probe.dropped > 0);
}

#[test]
fn non_utf8_after_the_kept_prefix_skips_the_rule_instead_of_leaking_it() {
    let dir = project();
    let cap = kernel::consts_policy::INTERVAL_CAP_BYTES;
    let mut bytes = vec![b'x'; cap.saturating_add(16)];
    bytes[cap.saturating_add(4)] = 0xff;
    write(dir.path(), "lab/pkg/probe/AGENTS.md", &bytes);
    assert!(
        nested_rules(
            scope(dir.path()),
            &at("lab/pkg/probe/a.rs"),
            &at("lab/pkg/probe/a.rs"),
        )
        .into_iter()
        .all(|candidate| candidate.room != at("lab/pkg/probe"))
    );
}
