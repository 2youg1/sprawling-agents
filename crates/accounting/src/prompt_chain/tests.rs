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
        bound: super::read_bound(root, at("lab")),
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
        &runtime::RuleContext {
            admitted: at("lab/pkg/probe/a.rs"),
            landed: at("lab/pkg/probe/a.rs"),
        },
        &BTreeSet::new(),
        &mut || false,
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
            nested_rules(
                scope(dir.path()),
                &runtime::RuleContext {
                    admitted: at(admitted),
                    landed: at(landed)
                },
                &BTreeSet::new(),
                &mut || false,
            )
            .is_empty(),
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
    assert_eq!(selected[0].clone().into_json().get("dropped"), None);
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
        &runtime::RuleContext {
            admitted: at("lab/pkg/probe/a.rs"),
            landed: at("lab/pkg/probe/a.rs"),
        },
        &BTreeSet::new(),
        &mut || false,
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
        &runtime::RuleContext {
            admitted: at("lab/pkg/probe/a.rs"),
            landed: at("lab/pkg/probe/a.rs"),
        },
        &BTreeSet::new(),
        &mut || false,
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
fn a_rule_behind_an_oversized_one_arrives_on_the_next_read() {
    let dir = project();
    let cap = kernel::consts_policy::INTERVAL_CAP_BYTES;
    write(
        dir.path(),
        "lab/pkg/AGENTS.md",
        "x".repeat(cap.saturating_add(1)).as_bytes(),
    );
    let mut delivery = RuleDelivery::default();
    let first = nested_rules(
        scope(dir.path()),
        &runtime::RuleContext {
            admitted: at("lab/pkg/probe/a.rs"),
            landed: at("lab/pkg/probe/a.rs"),
        },
        &delivery.seen(),
        &mut || false,
    );
    assert_eq!(delivery.answer("first", first).len(), 1);
    delivery.complete_wave(&BTreeSet::new());

    let second = nested_rules(
        scope(dir.path()),
        &runtime::RuleContext {
            admitted: at("lab/pkg/probe/a.rs"),
            landed: at("lab/pkg/probe/a.rs"),
        },
        &delivery.seen(),
        &mut || false,
    );
    let selected = delivery.answer("second", second);
    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].room, at("lab/pkg/probe"));
    assert_eq!(selected[0].text, "probe rules\n");
    assert_eq!(selected[0].dropped, 0);
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
            &runtime::RuleContext {
                admitted: at("lab/pkg/probe/a.rs"),
                landed: at("lab/pkg/probe/a.rs")
            },
            &BTreeSet::new(),
            &mut || false,
        )
        .into_iter()
        .all(|candidate| candidate.room != at("lab/pkg/probe"))
    );
}

#[test]
fn cancellation_during_scanning_never_delivers_a_partial_rule() {
    let dir = project();
    let mut polls = 0;
    let candidates = nested_rules(
        scope(dir.path()),
        &runtime::RuleContext {
            admitted: at("lab/pkg/probe/a.rs"),
            landed: at("lab/pkg/probe/a.rs"),
        },
        &BTreeSet::new(),
        &mut || {
            polls += 1;
            polls > 2
        },
    );
    assert!(candidates.is_empty());
    assert!(polls > 2);
}

#[test]
fn rendered_rule_headings_and_priority_fit_in_the_budget() {
    let cap = kernel::consts_policy::INTERVAL_CAP_BYTES;
    let mut delivery = RuleDelivery::default();
    for room in ["lab/pkg", "[lab/pkg"] {
        let selected = delivery.answer("read", vec![candidate(room, "界".repeat(cap))]);
        assert_eq!(selected.len(), 1);
        let payload = kernel::Payload::of(&serde_json::json!({
            "path": "lab/a.rs", "text": "",
            "rules": selected.into_iter().map(RuleCandidate::into_json).collect::<Vec<_>>()
        }))
        .unwrap();
        let rendered = runtime::rendering::shown(
            runtime::rendering::CURRENT,
            &kernel::ToolName::parse("read").unwrap(),
            &kernel::event::record::ToolAnswer::Answered { result: payload },
        )
        .unwrap();
        assert!(rendered.len().saturating_sub("lab/a.rs is empty\n".len()) <= cap);
    }
}

#[cfg(unix)]
#[test]
fn worktree_rules_use_the_main_city_authorization() {
    let main = project();
    let tree = project();
    write(
        main.path(),
        "secret/.sprawling/RULES.toml",
        b"confidential = true\nwrite = \"everything\"\n",
    );
    write(tree.path(), "secret/rules.txt", b"private text\n");
    let rule = tree.path().join("lab/pkg/AGENTS.md");
    std::fs::remove_file(&rule).unwrap();
    std::os::unix::fs::symlink("../../secret/rules.txt", rule).unwrap();
    let mut scope = scope(tree.path());
    scope.bound = super::read_bound(main.path(), at("lab"));
    let candidates = nested_rules(
        scope,
        &runtime::RuleContext {
            admitted: at("lab/pkg/probe/a.rs"),
            landed: at("lab/pkg/probe/a.rs"),
        },
        &BTreeSet::new(),
        &mut || false,
    );
    assert!(
        candidates
            .iter()
            .all(|candidate| !candidate.text.contains("private text"))
    );
    assert_eq!(candidates.len(), 1);
}
