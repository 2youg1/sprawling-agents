// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a bundle carries beside the lines (accounting-SPEC.md 8-17): a
//! call's duration only where both its moments were measured, a run's
//! policy, and a commit's base, diff and trace, each state kept apart.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use std::path::Path;

use kernel::{AxCode, AxError, EventDraft, EventKind, EventRecord, Payload, RunId, Seq, TimeMs};
use serde_json::{Value, json};

use super::super::{Request, Selection, export};
use super::{Line, addr, confidential, parsed, person, run, seqs, write};

/// The ledger lines `script` spells, chained from genesis, each line
/// written at its own ledger version and `t`.
fn versioned(script: Vec<(u32, u64, Line)>) -> Vec<Vec<u8>> {
    let mut prev = kernel::GENESIS_PREV;
    let mut out = Vec::new();
    for (seq, (v, t, (run, at, kind, data))) in script.into_iter().enumerate() {
        let draft = EventDraft {
            run,
            t: TimeMs::new(t),
            who: "tester".to_owned(),
            addr: at.map(addr),
            kind,
            data: Payload::new(data.as_object().cloned().unwrap()).unwrap(),
            ig: false,
        };
        let current = EventRecord::from_draft(draft, Seq::new(u64::try_from(seq).unwrap()), prev)
            .canonical_line()
            .unwrap();
        let line = String::from_utf8(current)
            .unwrap()
            .replacen("{\"v\":2,", &format!("{{\"v\":{v},"), 1)
            .into_bytes();
        prev = kernel::ledger::chain_hash(&line);
        out.push(line);
    }
    out
}

fn called(id: &str, tool: &str) -> Value {
    json!({"id": id, "name": tool, "args": {}})
}

fn answered(id: &str, tool: &str) -> Value {
    json!({"tool_use_id": id, "name": tool, "result": {"content": "ok"}})
}

fn policy() -> Value {
    json!({"mode": "work", "write": "full", "admit": "tested", "landing": "ordinary"})
}

fn request() -> Value {
    json!({"node": "1", "implementer": "lab/a", "branch": "lab-a", "commit": "1".repeat(40)})
}

/// Run 1 calls four tools: one written before lines recorded their own
/// moment, one measured, one the city answered itself after a restart,
/// and one never answered. Its pull request is refused at the merge for
/// want of the evidence its policy asked for.
fn timed() -> (tempfile::TempDir, Vec<Vec<u8>>) {
    let dir = tempfile::tempdir().unwrap();
    let r1 = run(1);
    let at = Some("lab/a");
    let restarted = serde_json::to_value(
        AxError::failure(
            AxCode::ToolOutcomeUnknown,
            "exec",
            "the city restarted while the call ran",
        )
        .with_recovery("call it again"),
    )
    .unwrap();
    let mut refused = request();
    refused["by"] = json!("mode");
    refused["why"] = json!("the run asked for tested work and no test ran; run the tests first");
    let written = versioned(vec![
        (
            1,
            0,
            (RunId::CITY, None, EventKind::CityInitialized, json!({})),
        ),
        (
            1,
            1_000,
            (r1, at, EventKind::RunStarted, json!({"policy": policy()})),
        ),
        (1, 1_000, (r1, at, EventKind::ModelCalled, json!({}))),
        (
            1,
            1_000,
            (r1, at, EventKind::ToolCalled, called("c1", "read")),
        ),
        (
            1,
            1_000,
            (r1, at, EventKind::ToolResult, answered("c1", "read")),
        ),
        (
            2,
            2_000,
            (r1, at, EventKind::ToolCalled, called("c2", "exec")),
        ),
        (
            2,
            2_350,
            (r1, at, EventKind::ToolResult, answered("c2", "exec")),
        ),
        (
            2,
            3_000,
            (r1, at, EventKind::ToolCalled, called("c3", "exec")),
        ),
        (
            2,
            9_000,
            (
                r1,
                at,
                EventKind::ToolResult,
                json!({"tool_use_id": "c3", "name": "exec", "error": restarted}),
            ),
        ),
        (
            2,
            9_500,
            (r1, at, EventKind::ToolCalled, called("c4", "read")),
        ),
        (2, 9_600, (r1, at, EventKind::PrOpened, request())),
        (2, 9_700, (r1, at, EventKind::PrRejected, refused)),
    ]);
    write(dir.path(), &written, b"");
    (dir, written)
}

#[test]
fn a_call_is_timed_only_where_both_its_moments_were_measured() {
    let (dir, _) = timed();
    let r1 = run(1).to_string();
    let whole = parsed(export(dir.path(), &person()).unwrap().bytes());
    let late = parsed(
        export(
            dir.path(),
            &Request {
                selection: Selection::new(Some(Seq::new(6)), None, None, None).unwrap(),
                ..person()
            },
        )
        .unwrap()
        .bytes(),
    );
    let refusal: Vec<(Value, Value)> = whole["moments"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|moment| moment["family"] == "pr")
        .map(|moment| (moment["opened"].clone(), moment["closed"].clone()))
        .collect();
    assert_eq!(
        (
            whole["calls"].clone(),
            whole["runs"][0]["policy"].clone(),
            refusal,
            late["calls"][0].clone(),
            seqs(&late["context"]).contains(&"5".to_owned()),
        ),
        (
            json!([
                {"run": r1, "id": "c1", "tool": "read",
                 "called": {"at": "3"}, "answered": {"at": "4"}, "took": "unknown"},
                {"run": r1, "id": "c2", "tool": "exec",
                 "called": {"at": "5"}, "answered": {"at": "6"}, "took": {"measured": "350"}},
                {"run": r1, "id": "c3", "tool": "exec",
                 "called": {"at": "7"}, "answered": {"at": "8"}, "took": "unknown"},
                {"run": r1, "id": "c4", "tool": "read",
                 "called": {"at": "9"}, "answered": "pending", "took": "unknown"},
            ]),
            policy(),
            vec![(json!({"at": "10"}), json!({"at": "11"}))],
            json!({"run": r1, "id": "c2", "tool": "exec",
                   "called": {"outside": "5"}, "answered": {"at": "6"}, "took": {"measured": "350"}}),
            true,
        )
    );
}

/// A commit in the city's own repository with `files`, on `parent`.
fn commit(root: &Path, files: &[(&str, &[u8])], parent: Option<git2::Oid>) -> git2::Oid {
    let repo = git2::Repository::open(root).unwrap();
    let mut index = repo.index().unwrap();
    for (path, body) in files {
        let on_disk = root.join(path);
        std::fs::create_dir_all(on_disk.parent().unwrap()).unwrap();
        std::fs::write(&on_disk, body).unwrap();
        index.add_path(Path::new(path)).unwrap();
    }
    index.write().unwrap();
    let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
    let signature =
        git2::Signature::new("tester", "tester@example.invalid", &git2::Time::new(0, 0)).unwrap();
    let parents: Vec<git2::Commit<'_>> = parent
        .map(|oid| repo.find_commit(oid).unwrap())
        .into_iter()
        .collect();
    let parents: Vec<&git2::Commit<'_>> = parents.iter().collect();
    repo.commit(None, &signature, &signature, "checkpoint", &tree, &parents)
        .unwrap()
}

/// The key-shaped line a patch must never echo.
fn key() -> String {
    format!("sk-ant-api03-{}", "a1B2c3D4e5".repeat(9))
}

/// Run 1 commits twice in `lab/a` and makes one call in between, then
/// announces a commit the repository does not hold. The second commit
/// changes a text file (one line key-shaped), a binary file, a file in
/// the confidential building `vault`, and leaves a fourth unchanged.
fn committed() -> (tempfile::TempDir, [String; 3]) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    git2::Repository::init(root).unwrap();
    let first = commit(
        root,
        &[
            ("lab/a/notes.md", b"one\n"),
            ("lab/a/pic.bin", b"\x00\x01\x02"),
            ("lab/a/same.md", b"same\n"),
            ("vault/x.md", b"plan\n"),
        ],
        None,
    );
    let second = commit(
        root,
        &[
            (
                "lab/a/notes.md",
                format!("one\ntwo\n{}\n", key()).as_bytes(),
            ),
            ("lab/a/pic.bin", b"\x00\x03"),
            ("vault/x.md", b"plan b\n"),
        ],
        Some(first),
    );
    let [first, second, absent] = [first.to_string(), second.to_string(), "ab".repeat(20)];
    let r1 = run(1);
    let at = Some("lab/a");
    let checkpoint = |oid: &str, files: &[&str]| {
        (
            r1,
            at,
            EventKind::CheckpointCommitted,
            json!({"oid": oid, "scope": ["lab"], "files": files}),
        )
    };
    let written = super::lines(vec![
        (RunId::CITY, None, EventKind::CityInitialized, json!({})),
        (
            RunId::CITY,
            Some("lab"),
            EventKind::BuildingCreated,
            json!({"addr": "lab", "template": "builds"}),
        ),
        (r1, at, EventKind::RunStarted, json!({})),
        checkpoint(&first, &["lab/a/notes.md"]),
        (r1, at, EventKind::ModelCalled, json!({})),
        (r1, at, EventKind::ToolCalled, called("c1", "edit")),
        (r1, at, EventKind::ToolResult, answered("c1", "edit")),
        checkpoint(
            &second,
            &[
                "lab/a/notes.md",
                "lab/a/pic.bin",
                "lab/a/same.md",
                "vault/x.md",
            ],
        ),
        checkpoint(&absent, &["lab/a/notes.md"]),
    ]);
    write(root, &written, b"");
    confidential(root, "vault");
    (dir, [first, second, absent])
}

#[test]
fn a_commits_diff_keeps_each_state_apart_and_its_trace_names_the_calls() {
    let (dir, [first, second, absent]) = committed();
    let bundle = parsed(export(dir.path(), &person()).unwrap().bytes());
    let evidence: Vec<(Value, Value, Value, Value)> = bundle["checkpoints"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            let held = &row["holds"]["committed"];
            (
                held["oid"].clone(),
                held["base"].clone(),
                held["diff"].clone(),
                held["trace"].clone(),
            )
        })
        .collect();
    let reason = kernel::secret::scan(key().as_bytes())[0]
        .provider
        .unwrap_or("high entropy");
    let traced = |calls: Value| json!({"traced": {"calls": calls, "nearby": []}});
    assert_eq!(
        evidence,
        vec![
            (json!(first), json!("none"), json!([]), traced(json!([]))),
            (
                json!(second),
                json!({"previous": first}),
                json!([
                    {"path": "lab/a/notes.md", "change": {"patch": {
                        "lines": [{"number": "1", "text": " one"}, {"number": "2", "text": "+two"}],
                        "credential": [{"number": "3", "reason": reason}],
                    }}},
                    {"path": "lab/a/pic.bin", "change": "binary"},
                    {"path": "lab/a/same.md", "change": "empty"},
                    {"path": "vault/x.md", "change": "withheld"},
                ]),
                traced(json!([{"at": "5"}])),
            ),
            (
                json!(absent),
                json!({"previous": second}),
                json!([{"path": "lab/a/notes.md", "change": "missing"}]),
                traced(json!([])),
            ),
        ]
    );
}
