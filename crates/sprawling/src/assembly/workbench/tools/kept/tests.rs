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

use crate::assembly::fixture::*;
use crate::assembly::*;

/// An anthropic-shaped key, put together at run time so that the tree
/// itself holds no key shape for `xtask secret` to find.
fn written() -> String {
    ["sk-", "ant-", &"Wq3z".repeat(10)].concat()
}

/// A key the model writes into a new file through `edit` reaches the
/// vault, and the file holds the reference that resolves to it.
#[test]
fn a_written_key_reaches_the_vault_and_not_the_file() {
    let key = written();
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    let (base_url, _provider) = fake_openai(
        &["m-local"],
        vec![
            tool_completion(
                "writing",
                "tu_1",
                "edit",
                serde_json::json!({
                    "path": "lab/room1/keys.md",
                    "base_version": "new",
                    "old": "",
                    "new": format!("token = {key}\n"),
                }),
            ),
            completion("done", None),
        ],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .handle(channels::Command::Dispatch {
            addr: Address::parse("lab/room1").unwrap(),
            task: "write the token down".to_owned(),
            goal: "the token is in keys.md".to_owned(),
            mode: kernel::Mode::PlanGoal,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"write"),
            session: None,
            effort: None,
        })
        .unwrap();

    let file = std::fs::read_to_string(dir.path().join("lab/room1/keys.md")).unwrap();
    assert!(!file.contains(&key), "the file holds the key: {file}");
    let at = file
        .find("secret:written/")
        .expect("the file holds the reference in the key's place");
    let reference = kernel::SecretRef::parse(file[at..].trim_end()).unwrap();
    let vault = worker.vault_handle();
    let held = vault.lock().unwrap().resolve(&reference).unwrap();
    assert_eq!(
        *held.into_vault_value(),
        key,
        "the vault holds the key itself"
    );
}
