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

/// A key a tool reads out of the city reaches the vault, and the model is
/// handed the reference that resolves to it.
#[test]
fn a_key_a_tool_reads_reaches_the_vault_and_not_the_model() {
    let key = written();
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    std::fs::create_dir_all(dir.path().join("lab/room1")).unwrap();
    // Ignored, as a person keeps a key file: a key the checkpoint would
    // stage is refused there before any tool runs, which is a different
    // door from the one this test is about.
    std::fs::write(dir.path().join("lab/room1/.gitignore"), "keys.md\n").unwrap();
    std::fs::write(
        dir.path().join("lab/room1/keys.md"),
        format!("token = {key}\n"),
    )
    .unwrap();
    let (base_url, provider) = fake_openai(
        &["m-local"],
        vec![
            tool_completion(
                "reading",
                "tu_1",
                "read",
                serde_json::json!({ "path": "lab/room1/keys.md" }),
            ),
            completion("done", None),
        ],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .handle(channels::Command::Dispatch {
            addr: Address::parse("lab/room1").unwrap(),
            task: "read the token".to_owned(),
            goal: "the token is known".to_owned(),
            mode: kernel::Mode::PlanGoal,
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"read"),
            session: None,
            effort: None,
        })
        .unwrap();

    let asked = provider.bodies().join("\n");
    assert!(!asked.contains(&key), "a request carried the key: {asked}");
    let at = asked
        .find("secret:output/")
        .expect("the request carries the reference in the key's place");
    let reference: String = asked[at..]
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, ':' | '/' | '-' | '_' | '.'))
        .collect();
    let reference = kernel::SecretRef::parse(&reference).unwrap();
    let vault = worker.vault_handle();
    let held = vault.lock().unwrap().resolve(&reference).unwrap();
    assert_eq!(
        *held.into_vault_value(),
        key,
        "the vault holds the key itself"
    );
}

/// A tool that records the arguments it was handed, standing for exec,
/// delegate, an MCP tool or any other tool on the bench that is not edit.
struct Recording {
    meta: kernel::ToolMeta,
    seen: std::sync::Arc<std::sync::Mutex<Vec<kernel::Payload>>>,
}

impl kernel::Tool for Recording {
    fn meta(&self) -> &kernel::ToolMeta {
        &self.meta
    }

    fn invoke(&mut self, call: &kernel::ToolCall) -> Result<kernel::ToolOutcome, kernel::AxError> {
        self.seen.lock().unwrap().push(call.args.clone());
        Ok(kernel::ToolOutcome {
            result: kernel::Payload::new(serde_json::Map::new()).unwrap(),
            attachments: Vec::new(),
        })
    }
}

/// A key nested anywhere in the arguments of a tool other than edit,
/// such as the command line exec would run to write a file, reaches the
/// tool as the reference that resolves to it.
#[test]
fn a_key_in_any_tool_argument_reaches_the_vault_and_not_the_tool() {
    use super::{Keeper, Kept};
    use kernel::Tool;
    let key = written();
    let vault = std::sync::Arc::new(std::sync::Mutex::new(gateway::Custodian::in_memory()));
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let meta = kernel::ToolMeta {
        name: kernel::ToolName::parse("exec").unwrap(),
        disclosure: String::new(),
        params: kernel::Payload::new(serde_json::Map::new()).unwrap(),
        effect: kernel::Effect::Read,
        cost_tier: kernel::CostTier::Light,
        timeout: None,
        render: kernel::RenderIntent::Generic,
        temporal: kernel::Temporal::Timeless,
    };
    let mut kept = Kept::new(
        Box::new(Recording {
            meta,
            seen: seen.clone(),
        }),
        std::sync::Arc::new(Keeper::new(vault.clone(), 7)),
    );
    let args = serde_json::json!({
        "argv": ["sh", "-c", format!("echo {key} > f")],
    })
    .as_object()
    .unwrap()
    .clone();
    kept.invoke(&kernel::ToolCall {
        id: "tu_1".to_owned(),
        name: kernel::ToolName::parse("exec").unwrap(),
        args: kernel::Payload::new(args).unwrap(),
    })
    .unwrap();

    let handed = serde_json::to_string(&seen.lock().unwrap()[0]).unwrap();
    assert!(
        !handed.contains(&key),
        "the tool was handed the key: {handed}"
    );
    let at = handed
        .find("secret:written/")
        .expect("the tool was handed the reference in the key's place");
    let reference: String = handed[at..]
        .chars()
        .take_while(|c| !c.is_whitespace() && *c != '"')
        .collect();
    let reference = kernel::SecretRef::parse(&reference).unwrap();
    let held = vault.lock().unwrap().resolve(&reference).unwrap();
    assert_eq!(
        *held.into_vault_value(),
        key,
        "the vault holds the key itself"
    );
}
