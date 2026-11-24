// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeSet;

use super::lockfile::{allowed, document, lock_of, manifest_of, permitted, read_jsonc};
use super::{RUNTIME, judge_lockfile, judge_runtime};

#[test]
fn the_lockfile_bun_writes_is_read_despite_its_trailing_commas() {
    let text = r#"{
  // bun explains itself here
  "workspaces": {
    "": {
      "dependencies": {
        "effect": "3.22.1",
        "solid-js": "1.9.15",
      },
      "devDependencies": {
        "vite": "8.2.2",
      },
    },
  },
}"#;
    let locked = lock_of(&document("bun.lock", text).unwrap()).unwrap();
    assert_eq!(
        locked.runtime.get("effect").map(String::as_str),
        Some("3.22.1")
    );
    assert_eq!(locked.development.len(), 1);
}

#[test]
fn a_comma_or_a_slash_inside_a_string_survives_normalisation() {
    let text = r#"{"home": "https://example.test/a,b", "range": ">=1, <2"}"#;
    assert_eq!(read_jsonc(text), text);
}

#[test]
fn every_way_the_lockfile_and_the_manifest_disagree_is_named() {
    let manifest = manifest_of(
        &document(
            "package.json",
            r#"{"dependencies": {"effect": "3.22.1", "solid-js": "1.9.15"}}"#,
        )
        .unwrap(),
    );
    let locked = manifest_of(
        &document(
            "bun.lock",
            r#"{"dependencies": {"effect": "3.21.0", "left-pad": "1.0.0"}}"#,
        )
        .unwrap(),
    );
    let mut out = Vec::new();
    judge_lockfile(&manifest, &locked, &mut out);
    let said: Vec<&str> = out.iter().map(|held| held.violation.as_str()).collect();
    assert_eq!(said.len(), 3, "{said:?}");
    assert!(said.iter().any(|held| held.contains("effect")));
    assert!(said.iter().any(|held| held.contains("solid-js")));
    assert!(said.iter().any(|held| held.contains("left-pad")));
}

#[test]
fn the_runtime_allowlist_is_judged_in_both_directions() {
    let judged = |names: &[&str]| {
        let listed: Vec<String> = names
            .iter()
            .map(|name| format!(r#""{name}": "1""#))
            .collect();
        let body = format!(r#"{{"dependencies": {{{}}}}}"#, listed.join(", "));
        let mut out = Vec::new();
        judge_runtime(
            &manifest_of(&document("package.json", &body).unwrap()),
            &mut out,
        );
        out
    };

    let extra: Vec<&str> = RUNTIME.iter().copied().chain(["lodash"]).collect();
    let out = judged(&extra);
    assert_eq!(out.len(), 1);
    assert!(out[0].violation.contains("lodash"));

    let missing: Vec<&str> = RUNTIME
        .iter()
        .copied()
        .filter(|name| *name != "solid-js")
        .collect();
    let out = judged(&missing);
    assert_eq!(out.len(), 1);
    assert!(out[0].violation.contains("solid-js"));

    assert!(judged(&RUNTIME).is_empty());
}

#[test]
fn an_spdx_expression_is_read_the_way_cargo_deny_reads_it() {
    let permitted: BTreeSet<String> = ["MIT".to_owned(), "Apache-2.0".to_owned()].into();
    assert!(allowed("MIT", &permitted));
    assert!(allowed("(MIT OR CC0-1.0)", &permitted));
    assert!(allowed("MIT AND Apache-2.0", &permitted));
    assert!(!allowed("MIT AND GPL-3.0", &permitted));
    assert!(!allowed("GPL-3.0", &permitted));
    assert!(
        !allowed("", &permitted),
        "an unstated licence is not a permitted one"
    );
}

#[test]
fn parentheses_group_before_an_or_splits_them() {
    let permitted: BTreeSet<String> = [
        "MIT".to_owned(),
        "Apache-2.0".to_owned(),
        "Zlib".to_owned(),
        "Apache-2.0 WITH LLVM-exception".to_owned(),
    ]
    .into();
    assert!(!allowed("(MIT OR Apache-2.0) AND GPL-3.0", &permitted));
    assert!(allowed("(MIT AND Zlib)", &permitted));
    assert!(allowed("MIT OR (GPL-3.0 AND Apache-2.0)", &permitted));
    assert!(allowed("Apache-2.0 WITH LLVM-exception", &permitted));
    assert!(!allowed("GPL-3.0 WITH LLVM-exception", &permitted));
    assert!(!allowed("(MIT OR Apache-2.0", &permitted));
    assert!(!allowed("MIT AND", &permitted));
}

#[test]
fn the_permitted_licences_are_this_repositorys_own_and_not_a_second_copy() {
    let text = std::fs::read_to_string(root().join("deny.toml")).unwrap();
    let allowlist = permitted(&text).unwrap();
    assert!(allowlist.contains("MPL-2.0"));
    assert!(!allowlist.contains("GPL-3.0"));
    assert!(permitted("[licenses]\nallow = []\n").is_err());
    assert!(permitted("[bans]\ndeny = []\n").is_err());
}

#[test]
fn every_workflow_running_the_gates_prepares_what_they_judge() {
    const PREPARES: &str = "./.github/actions/client-artifacts";
    const JUDGES: [&str; 2] = ["cargo xtask gates", "cargo nextest run --workspace"];
    assert!(
        root()
            .join(".github/actions/client-artifacts/action.yml")
            .is_file(),
        "the action every workflow names has to be the one that exists"
    );
    let mut unprepared = Vec::new();
    for entry in std::fs::read_dir(root().join(".github/workflows")).unwrap() {
        let path = entry.unwrap().path();
        let text = std::fs::read_to_string(&path).unwrap();

        let judges = text
            .lines()
            .filter(|line| !line.trim_start().starts_with('#'))
            .any(|line| JUDGES.iter().any(|command| line.contains(command)));
        if judges && !text.contains(PREPARES) {
            unprepared.push(path.display().to_string());
        }
    }
    assert!(unprepared.is_empty(), "{unprepared:#?}");
}

pub(super) fn root() -> std::path::PathBuf {
    crate::root::this_checkout().to_path_buf()
}
