// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeSet;

use super::lockfile::{allowed, document, lock_of, manifest_of, permitted, read_jsonc};
use super::{check, judge_lockfile, judge_runtime};

/// What `bun.lock` actually looks like: trailing commas after the last
/// entry of every object. A reader that handed this to `serde_json`
/// unchanged would fail on the first table and report the whole client
/// as drift.
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

/// A comma inside a string is content, and a `//` inside one is a URL.
/// A normaliser that edited either would change what is compared.
#[test]
fn a_comma_or_a_slash_inside_a_string_survives_normalisation() {
    let text = r#"{"home": "https://example.test/a,b", "range": ">=1, <2"}"#;
    assert_eq!(read_jsonc(text), text);
}

/// The two files disagreeing is the drift this assertion exists for, and
/// each of the three shapes it takes is named separately: a version that
/// moved, an entry the lockfile never learned, and one it kept after the
/// manifest dropped it.
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

/// Both directions. A gate that only refused additions would wave
/// through the day `solid-js` is deleted by accident, and that is the
/// half a person would notice last.
#[test]
fn the_runtime_allowlist_is_judged_in_both_directions() {
    let extra = manifest_of(
        &document(
            "package.json",
            r#"{"dependencies": {"effect": "1", "solid-js": "1", "lodash": "1"}}"#,
        )
        .unwrap(),
    );
    let mut out = Vec::new();
    judge_runtime(&extra, &mut out);
    assert_eq!(out.len(), 1);
    assert!(out[0].violation.contains("lodash"));

    let missing =
        manifest_of(&document("package.json", r#"{"dependencies": {"effect": "1"}}"#).unwrap());
    let mut out = Vec::new();
    judge_runtime(&missing, &mut out);
    assert_eq!(out.len(), 1);
    assert!(out[0].violation.contains("solid-js"));

    let exact = manifest_of(
        &document(
            "package.json",
            r#"{"dependencies": {"effect": "1", "solid-js": "1"}}"#,
        )
        .unwrap(),
    );
    let mut out = Vec::new();
    judge_runtime(&exact, &mut out);
    assert!(out.is_empty());
}

/// An `OR` offers a choice and an `AND` imposes all of it. Reading them
/// the other way round would either refuse half the ecosystem or admit a
/// licence this repository does not permit.
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

/// The allowlist has one home. A second copy inside this gate would let
/// the two sides of the tree carry different licences and call both
/// green.
#[test]
fn the_permitted_licences_are_this_repositorys_own_and_not_a_second_copy() {
    let text = std::fs::read_to_string(root().join("deny.toml")).unwrap();
    let allowlist = permitted(&text).unwrap();
    assert!(allowlist.contains("MPL-2.0"));
    assert!(!allowlist.contains("GPL-3.0"));
    assert!(permitted("[licenses]\nallow = []\n").is_err());
    assert!(permitted("[bans]\ndeny = []\n").is_err());
}

/// The gate against the tree it guards. This is the assertion that would
/// have caught card 6.1's omission on the day it happened.
#[test]
fn this_repository_passes_its_own_npm_gate() {
    let violations = check(&root()).unwrap();
    let said: Vec<String> = violations
        .iter()
        .map(|held| format!("{}: {}", held.location, held.violation))
        .collect();
    assert!(said.is_empty(), "{said:#?}");
}

pub(super) fn root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}
