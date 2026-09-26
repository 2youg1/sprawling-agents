// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::string_slice,
    reason = "test code"
)]

use std::path::Path;

use crate::assembly::fixture::*;
use crate::assembly::*;

/// An anthropic-shaped key, put together at run time so that the tree
/// itself holds no key shape for `xtask secret` to find.
fn pasted() -> String {
    ["sk-", "ant-", &"Kx7q".repeat(10)].concat()
}

/// Every file under `dir` whose bytes contain `needle`.
fn files_holding(dir: &Path, needle: &[u8]) -> Vec<std::path::PathBuf> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            found.extend(files_holding(&path, needle));
        } else if std::fs::read(&path)
            .unwrap()
            .windows(needle.len())
            .any(|window| window == needle)
        {
            found.push(path);
        }
    }
    found
}

/// A key pasted into the task text reaches the vault and nothing else:
/// no request the model receives, no ledger line and no file in the city
/// holds it, and the reference that stands in its place resolves to it.
#[test]
fn a_pasted_key_reaches_the_vault_and_nothing_else() {
    let key = pasted();
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    let (base_url, provider) = fake_openai(&["m-local"], vec![completion("done", None)]);
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .handle(channels::Command::Dispatch {
            addr: Address::parse("lab/room1").unwrap(),
            task: format!("call the messages API with {key} and report the model list"),
            goal: "the list is written down".to_owned(),
            mode: kernel::Mode::PlanGoal,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"paste"),
            session: None,
            effort: None,
        })
        .unwrap();

    // The worker holds a byte-range lock on a city file while it lives,
    // and Windows refuses a read of a locked range; drop it before the scan.
    let vault = worker.vault_handle();
    drop(worker);
    let asked = provider.bodies().join("\n");
    assert!(!asked.contains(&key), "a request carried the key: {asked}");
    let at = asked
        .find("secret:pasted/")
        .expect("the request carries the reference in the key's place");
    let reference: String = asked[at..]
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, ':' | '/' | '-' | '_' | '.'))
        .collect();
    let reference = kernel::SecretRef::parse(&reference).unwrap();
    let held = vault.lock().unwrap().resolve(&reference).unwrap();
    assert_eq!(
        *held.into_vault_value(),
        key,
        "the vault holds the key itself"
    );
    // The open city holds its ledger lock file, and Windows refuses to
    // read a locked range, so the city is closed before its files are read.
    drop(worker);
    assert_eq!(
        files_holding(dir.path(), key.as_bytes()),
        Vec::<std::path::PathBuf>::new(),
        "the key was written into the city"
    );
}
