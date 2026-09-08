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

use super::super::*;
use crate::assembly::fixture::*;
use crate::assembly::*;

/// The command that prints one file, on this machine.
fn print_command(name: &str) -> (String, Vec<String>) {
    if cfg!(windows) {
        (
            "cmd".to_owned(),
            vec!["/C".to_owned(), "type".to_owned(), name.to_owned()],
        )
    } else {
        ("cat".to_owned(), vec![name.to_owned()])
    }
}

/// A real dispatch whose command prints more than the sieve's floor
/// reaches the model sieved, and the way back to the original resolves
/// (sprawling-SPEC 8-43).
///
/// Before this, `driving` handed the bench's outcome to the turn as it
/// came, so the sieve ran in the simulator and never in the product.
#[test]
fn a_command_output_over_the_floor_reaches_the_model_sieved_with_the_way_back() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    let room = dir.path().join("lab").join("room1");
    std::fs::create_dir_all(&room).unwrap();
    let mut noise = String::new();
    for n in 0..400 {
        noise.push_str(&format!("    Checking crate{n} v0.{n}.0\n"));
    }
    std::fs::write(room.join("build.log"), &noise).unwrap();
    let (path, args) = print_command("build.log");
    let (base_url, provider) = fake_openai(
        &["m-local"],
        vec![
            tool_completion(
                "reading the build",
                "tu_1",
                "exec",
                serde_json::json!({ "arm": { "program": { "path": path, "args": args } } }),
            ),
            completion("read it", None),
        ],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    worker
        .handle(channels::Command::Dispatch {
            addr: Address::parse("lab/room1").unwrap(),
            task: "read the build log".to_owned(),
            goal: "say what it says".to_owned(),
            mode: channels::ModeTag::parse("build").unwrap(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::new(0), b"dispatch"),
            session: None,
            effort: None,
        })
        .unwrap();
    let bodies = provider.bodies();
    drop(provider);

    let with_result = bodies
        .iter()
        .find(|body| body.contains("tu_1") && body.contains("\"tool\""))
        .expect("the second call carries the tool result");
    assert!(
        with_result.contains("[sieve:"),
        "the model read the command's output unsieved: {with_result}"
    );
    let verified = runtime::replay::verify_ledger_dir(&report.ledger_dir).unwrap();
    let result = verified
        .raw_lines()
        .iter()
        .map(|line| serde_json::from_slice::<serde_json::Value>(line).unwrap())
        .find(|value| value["kind"] == "tool_result")
        .expect("the result is on the ledger");
    let content = result["data"]["result"]["content"].as_str().unwrap();
    assert!(
        content.len() < noise.len() / 4,
        "four hundred lines of one template were not folded: {content}"
    );
    let account = &result["data"]["result"]["sieve"][0];
    let original = account["original"].as_str().unwrap();
    assert!(
        original.starts_with("cas:b3-"),
        "not a cas locator: {original}"
    );
    let cas = memory::Cas::open(&dir.path().join(".sprawling").join("cas")).unwrap();
    let pinned = cas.get(&kernel::B3Hash::digest(noise.as_bytes())).unwrap();
    assert_eq!(
        String::from_utf8(pinned).unwrap(),
        noise,
        "the way back does not resolve to what the command printed"
    );
    let rest = std::path::Path::new(account["rest_path"].as_str().unwrap());
    assert!(rest.exists(), "the rest file is not where the account says");
    assert!(
        rest.starts_with(&room),
        "the rest file is outside the room a model may read: {}",
        rest.display()
    );
}
