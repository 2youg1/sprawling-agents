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

mod support;

use std::process::Command;

use serde_json::Value;
use support::serve;

fn answer() -> Value {
    serde_json::json!({"stop_reason": "end_turn", "content": "done"})
}

fn run_at(directory: &std::path::Path, base: &str, prompt: &str, log: &std::path::Path) {
    let output = support::command(base, prompt)
        .current_dir(directory)
        .stderr(std::fs::File::create(log).unwrap())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn resume_at(
    directory: &std::path::Path,
    base: &str,
    prompt: &str,
    session: &std::path::Path,
    segment: &std::path::Path,
) {
    let output = support::command(base, prompt)
        .current_dir(directory)
        .arg("--resume")
        .arg(session)
        .stderr(std::fs::File::create(segment).unwrap())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn append(target: &std::path::Path, segment: &std::path::Path) {
    use std::io::Write as _;
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(target)
        .unwrap();
    file.write_all(&std::fs::read(segment).unwrap()).unwrap();
}

fn guide_occurrences(request: &Value, marker: &str) -> usize {
    request.to_string().matches(marker).count()
}

#[test]
fn later_turns_and_resume_keep_exactly_the_original_guide() {
    let directory = tempfile::tempdir().unwrap();
    let guide = directory.path().join("AGENTS.md");
    let session = directory.path().join("session.jsonl");
    let first_segment = directory.path().join("first.jsonl");
    let second_segment = directory.path().join("second.jsonl");
    std::fs::write(&guide, "original-rule").unwrap();

    let (base, first) = serve(vec![answer()]);
    run_at(directory.path(), &base, "first", &first_segment);
    std::fs::copy(&first_segment, &session).unwrap();
    let first_request = first.join().unwrap().remove(0);
    assert_eq!(
        first_request["messages"][0]["content"],
        serde_json::json!([
            {"type": "text", "text": "<workspace-instructions source=\"AGENTS.md\">\noriginal-rule\n</workspace-instructions>"},
            {"type": "text", "text": "first"}
        ])
    );

    std::fs::write(&guide, "changed-rule").unwrap();
    let (base, second) = serve(vec![answer()]);
    resume_at(directory.path(), &base, "second", &session, &second_segment);
    let second_request = second.join().unwrap().remove(0);
    assert_eq!(guide_occurrences(&second_request, "original-rule"), 1);
    assert_eq!(guide_occurrences(&second_request, "changed-rule"), 0);
    assert_eq!(second_request["messages"].as_array().unwrap().len(), 3);
    assert_eq!(second_request["messages"][2]["content"], "second");
    append(&session, &second_segment);

    std::fs::remove_file(&guide).unwrap();
    let (base, third) = serve(vec![answer()]);
    let third_segment = directory.path().join("third.jsonl");
    resume_at(directory.path(), &base, "third", &session, &third_segment);
    let third_request = third.join().unwrap().remove(0);
    assert_eq!(guide_occurrences(&third_request, "original-rule"), 1);
    assert_eq!(third_request["messages"].as_array().unwrap().len(), 5);
    assert_eq!(third_request["messages"][4]["content"], "third");
}

#[test]
fn resume_never_backfills_a_guide_absent_from_history() {
    let directory = tempfile::tempdir().unwrap();
    let session = directory.path().join("legacy.jsonl");
    std::fs::write(directory.path().join("AGENTS.md"), "new-rule").unwrap();
    std::fs::write(
        &session,
        "{\"role\":\"user\",\"content\":\"old\"}\n{\"role\":\"assistant\",\"content\":\"done\",\"stop_reason\":\"end_turn\"}\n",
    )
    .unwrap();
    let (base, gateway) = serve(vec![answer()]);
    resume_at(
        directory.path(),
        &base,
        "next",
        &session,
        &directory.path().join("next.jsonl"),
    );
    let request = gateway.join().unwrap().remove(0);
    assert_eq!(
        request["messages"],
        serde_json::json!([
            {"role": "user", "content": "old"},
            {"role": "assistant", "content": "done"},
            {"role": "user", "content": "next"}
        ])
    );
    assert_eq!(guide_occurrences(&request, "new-rule"), 0);
}

#[test]
fn a_failed_first_run_is_not_logged_or_duplicated_on_retry() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("AGENTS.md"), "retry-rule").unwrap();
    let failed_segment = directory.path().join("failed.jsonl");
    let (base, first) = serve(vec![
        serde_json::json!({"stop_reason": "max_tokens", "content": []}),
    ]);
    let output = support::command(&base, "first")
        .current_dir(directory.path())
        .stderr(std::fs::File::create(&failed_segment).unwrap())
        .output()
        .unwrap();
    assert!(!output.status.success());
    first.join().unwrap();

    let successful = directory.path().join("successful.jsonl");
    let (base, second) = serve(vec![answer()]);
    run_at(directory.path(), &base, "retry", &successful);
    let request = second.join().unwrap().remove(0);
    assert_eq!(guide_occurrences(&request, "retry-rule"), 1);
    assert_eq!(request["messages"].as_array().unwrap().len(), 1);
    assert_eq!(
        std::fs::read_to_string(successful)
            .unwrap()
            .matches("retry-rule")
            .count(),
        1
    );
}

#[test]
fn an_independent_headless_call_is_a_new_session() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("AGENTS.md"), "current-rule").unwrap();
    let (base, gateway) = serve(vec![answer(), answer()]);
    for prompt in ["one", "two"] {
        run_at(
            directory.path(),
            &base,
            prompt,
            &directory.path().join(format!("{prompt}.jsonl")),
        );
    }
    for request in gateway.join().unwrap() {
        assert_eq!(guide_occurrences(&request, "current-rule"), 1);
        assert_eq!(request["messages"].as_array().unwrap().len(), 1);
    }
}

#[test]
fn only_the_exact_workspace_top_level_is_read() {
    let parent = tempfile::tempdir().unwrap();
    let child = parent.path().join("child");
    std::fs::create_dir(&child).unwrap();
    std::fs::write(parent.path().join("AGENTS.md"), "parent-rule").unwrap();
    let (base, gateway) = serve(vec![answer()]);
    run_at(&child, &base, "task", &parent.path().join("run.jsonl"));
    let request = gateway.join().unwrap().remove(0);
    assert_eq!(request["messages"][0]["content"], "task");
}

#[test]
#[expect(clippy::disallowed_methods, reason = "test fixture process")]
fn malformed_workspace_input_fails_before_calling_the_model() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("AGENTS.md"), [0xff]).unwrap();
    let base = "http://127.0.0.1:1";
    let output = Command::new(env!("CARGO_BIN_EXE_mei"))
        .args(["-p", "task"])
        .current_dir(directory.path())
        .env("HARNESS_MODEL", "test/model")
        .env("ANTHROPIC_BASE_URL", base)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("E_MEI_WORKSPACE"));
}
