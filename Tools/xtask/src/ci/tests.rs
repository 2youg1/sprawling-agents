// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

#[test]
fn full_and_base_conflict_in_both_orders_and_cannot_repeat() {
    for args in [
        vec!["run", "--full", "--base", "main"],
        vec!["run", "--base", "main", "--full"],
        vec!["run", "--full", "--full"],
    ] {
        assert!(parse(&args.into_iter().map(str::to_owned).collect::<Vec<_>>()).is_err());
    }
}

#[test]
fn file_scope_keeps_the_full_roster_and_invalid_baselines_fail() {
    let root = tempfile::tempdir().unwrap();
    let git = |args: &[&str]| {
        let output = baseline::git(root.path(), args).unwrap();
        String::from_utf8(output.stdout).unwrap().trim().to_owned()
    };
    git(&["init", "--quiet"]);
    std::fs::write(root.path().join("note.md"), "before\n").unwrap();
    git(&["add", "--all"]);
    let base = git(&["write-tree"]);
    std::fs::write(root.path().join("note.md"), "captured\n").unwrap();
    let captured = view::Snapshot::capture(root.path()).unwrap();
    std::fs::write(root.path().join("note.md"), "before\n").unwrap();
    let full = parse(&["plan", "--full", "--scope-base", &base].map(str::to_owned)).unwrap();
    let changed = comparison(root.path(), &full, &captured).unwrap().unwrap();
    assert_eq!(changed, BTreeSet::from(["note.md".to_owned()]));
    let prepared = suite::ACTIONS
        .iter()
        .map(|action| suite::PreparedAction {
            action,
            key: content::Key::named("selection-fixture".to_owned()),
            variables: std::collections::BTreeMap::new(),
            files: None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        selected(&full, Some(&changed), &prepared).unwrap().len(),
        prepared.len()
    );
    let affected = parse(&["plan", "--base", &base].map(str::to_owned)).unwrap();
    assert!(
        selected(&affected, Some(&changed), &prepared)
            .unwrap()
            .len()
            < prepared.len()
    );
    for flag in ["--base", "--scope-base"] {
        let invalid = parse(&["plan", flag, "missing-base-object"].map(str::to_owned)).unwrap();
        assert!(matches!(
            comparison(root.path(), &invalid, &captured),
            Err(XtaskError::CiBaselineRevision { revision, .. }) if revision == "missing-base-object"
        ));
    }
    captured.close().unwrap();
    root.close().unwrap();
}

#[test]
fn affected_and_scope_bases_conflict_in_both_orders() {
    for args in [
        vec!["run", "--base", "main", "--scope-base", "main"],
        vec!["run", "--scope-base", "main", "--base", "main"],
        vec!["run", "--scope-base", "main", "--scope-base", "main"],
    ] {
        assert!(parse(&args.into_iter().map(str::to_owned).collect::<Vec<_>>()).is_err());
    }
}
