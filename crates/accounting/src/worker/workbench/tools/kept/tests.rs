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

use crate::worker::fixture::*;
use crate::worker::*;

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
    crate::worker::fixture::init_city(dir.path()).unwrap();
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
        .handle(wire::Command::Dispatch {
            addr: Address::parse("lab/room1").unwrap(),
            task: "write the token down".to_owned(),
            goal: "the token is in keys.md".to_owned(),
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"write"),
            session: None,
            effort: None,
            model: None,
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
    crate::worker::fixture::init_city(dir.path()).unwrap();
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
        .handle(wire::Command::Dispatch {
            addr: Address::parse("lab/room1").unwrap(),
            task: "read the token".to_owned(),
            goal: "the token is known".to_owned(),
            model: None,
            policy: kernel::RunPolicy::of(kernel::Mode::Work),
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

    fn invoke(&self, call: &kernel::ToolCall) -> Result<kernel::ToolOutcome, kernel::AxError> {
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
    let kept = Kept::new(
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

/// The same key handed to tools twice in one run is kept once: both calls
/// hand the tool one reference, and a different key gets its own.
#[test]
fn one_key_seen_twice_is_kept_under_one_reference() {
    use super::{Keeper, Kept};
    use kernel::Tool;
    let key = written();
    let other = ["sk-", "ant-", &"Pk7x".repeat(10)].concat();
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
    let kept = Kept::new(
        Box::new(Recording {
            meta,
            seen: seen.clone(),
        }),
        std::sync::Arc::new(Keeper::new(vault, 7)),
    );
    for text in [&key, &key, &other] {
        let args = serde_json::json!({ "text": text })
            .as_object()
            .unwrap()
            .clone();
        kept.invoke(&kernel::ToolCall {
            id: "tu_1".to_owned(),
            name: kernel::ToolName::parse("exec").unwrap(),
            args: kernel::Payload::new(args).unwrap(),
        })
        .unwrap();
    }

    let handed: Vec<String> = seen
        .lock()
        .unwrap()
        .iter()
        .map(|args| args.as_map()["text"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(
        handed,
        [
            "secret:written/anthropic-7-1",
            "secret:written/anthropic-7-1",
            "secret:written/anthropic-7-2",
        ],
    );
}

/// A tool that has already acted and returns a key it read.
struct Reading {
    meta: kernel::ToolMeta,
    found: String,
}

impl kernel::Tool for Reading {
    fn meta(&self) -> &kernel::ToolMeta {
        &self.meta
    }

    fn invoke(&self, _call: &kernel::ToolCall) -> Result<kernel::ToolOutcome, kernel::AxError> {
        let result = serde_json::json!({ "out": format!("token = {}", self.found) });
        Ok(kernel::ToolOutcome {
            result: kernel::Payload::new(result.as_object().unwrap().clone()).unwrap(),
            attachments: Vec::new(),
        })
    }
}

/// When the vault refuses a key in a result, the call still succeeds,
/// because the tool's effect has happened: the model reads a marker that
/// names the vault's code in the key's place, and never the key.
#[test]
fn a_result_key_the_vault_refuses_is_withheld_and_the_call_stands() {
    use super::{Keeper, Kept};
    use kernel::Tool;
    let key = written();
    let vault = std::sync::Arc::new(std::sync::Mutex::new(gateway::Custodian::in_memory()));
    let poisoner = vault.clone();
    std::thread::spawn(move || {
        let _held = poisoner.lock().unwrap();
        panic!("poison the vault");
    })
    .join()
    .unwrap_err();
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
    let kept = Kept::new(
        Box::new(Reading { meta, found: key }),
        std::sync::Arc::new(Keeper::new(vault, 7)),
    );
    let outcome = kept.invoke(&kernel::ToolCall {
        id: "tu_1".to_owned(),
        name: kernel::ToolName::parse("exec").unwrap(),
        args: kernel::Payload::new(serde_json::Map::new()).unwrap(),
    });

    assert_eq!(
        outcome.map(|outcome| serde_json::to_value(outcome.result.as_map()).unwrap()),
        Ok(serde_json::json!({ "out": "token = [key withheld: E_STORAGE_FATAL]" })),
    );
}
