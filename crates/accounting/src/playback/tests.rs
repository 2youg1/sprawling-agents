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

use std::path::Path;

use kernel::{Address, AxCode, B3Hash, EventDraft, EventKind, EventRecord, Payload, RunId, Seq};
use serde_json::{Value, json};

use super::{Confidential, Cutoff, Reader, Request, Selection, export};

pub(crate) fn run(n: u8) -> RunId {
    RunId::parse(&format!("0198f6a2-7c4a-7bbb-9d1e-0000000000{n:02}")).unwrap()
}

pub(crate) fn addr(raw: &str) -> Address {
    Address::parse(raw).unwrap()
}

pub(crate) type Line = (RunId, Option<&'static str>, EventKind, Value);

/// The ledger lines `script` spells, chained from genesis, each line's
/// `t` its seq.
pub(crate) fn lines(script: Vec<Line>) -> Vec<Vec<u8>> {
    lines_at(
        script
            .into_iter()
            .enumerate()
            .map(|(seq, line)| (u64::try_from(seq).unwrap(), line))
            .collect(),
    )
}

/// The ledger lines `script` spells, chained from genesis, each line at
/// the `t` it is paired with.
pub(crate) fn lines_at(script: Vec<(u64, Line)>) -> Vec<Vec<u8>> {
    let mut prev = kernel::GENESIS_PREV;
    let mut out = Vec::new();
    for (seq, (t, (run, at, kind, data))) in script.into_iter().enumerate() {
        let seq = u64::try_from(seq).unwrap();
        let draft = EventDraft {
            run,
            t: kernel::TimeMs::new(t),
            who: "tester".to_owned(),
            addr: at.map(addr),
            kind,
            data: Payload::new(data.as_object().cloned().unwrap()).unwrap(),
            ig: false,
        };
        let line = EventRecord::from_draft(draft, Seq::new(seq), prev)
            .canonical_line()
            .unwrap();
        prev = kernel::ledger::chain_hash(&line);
        out.push(line);
    }
    out
}

/// Writes `lines` as the city's one segment, followed by `tail` bytes.
pub(crate) fn write(root: &Path, lines: &[Vec<u8>], tail: &[u8]) {
    let dir = kernel::layout::CityLayout::new(root).ledger();
    std::fs::create_dir_all(&dir).unwrap();
    let mut blob: Vec<u8> = lines
        .iter()
        .flat_map(|line| [line.as_slice(), b"\n"].concat())
        .collect();
    blob.extend_from_slice(tail);
    std::fs::write(dir.join("ledger-00000000000000000000.jsonl"), blob).unwrap();
}

pub(crate) fn confidential(root: &Path, building: &str) {
    let path = city::rules_path(root, &addr(building));
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, "confidential = true\nwrite = \"everything\"\n").unwrap();
}

/// A city with an open building `lab` and a confidential one `vault`:
/// run 1 works in `lab` and once names a file in `vault`; run 2 works in
/// `vault`.
pub(crate) fn city() -> (tempfile::TempDir, Vec<Vec<u8>>) {
    let dir = tempfile::tempdir().unwrap();
    let written = lines(vec![
        (RunId::CITY, None, EventKind::CityInitialized, json!({})),
        (
            RunId::CITY,
            Some("lab"),
            EventKind::BuildingCreated,
            json!({"addr": "lab", "template": "builds"}),
        ),
        (
            RunId::CITY,
            Some("vault"),
            EventKind::BuildingCreated,
            json!({"addr": "vault", "template": "builds"}),
        ),
        (run(1), Some("lab/a"), EventKind::RunStarted, json!({})),
        (
            run(1),
            Some("lab/a"),
            EventKind::ToolCalled,
            json!({"tool": "read"}),
        ),
        (run(2), Some("vault/b"), EventKind::RunStarted, json!({})),
        (
            run(2),
            Some("vault/b"),
            EventKind::ToolCalled,
            json!({"tool": "secret-plan"}),
        ),
        (
            run(1),
            Some("lab/a"),
            EventKind::ToolCalled,
            json!({"path": "vault/notes.md"}),
        ),
        (run(1), Some("lab/a"), EventKind::RunFrozen, json!({})),
    ]);
    write(dir.path(), &written, b"");
    confidential(dir.path(), "vault");
    (dir, written)
}

pub(crate) fn person() -> Request {
    Request {
        selection: Selection::everything(),
        reader: Reader::Person(Confidential::Withheld),
        cutoff: Cutoff::Latest,
    }
}

pub(crate) fn parsed(bytes: &[u8]) -> Value {
    serde_json::from_slice(bytes).unwrap()
}

pub(crate) fn seqs(list: &Value) -> Vec<String> {
    list.as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["seq"].as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn a_frozen_history_exports_the_same_bytes_twice() {
    let (dir, _) = city();
    let first = export(dir.path(), &person()).unwrap();
    let second = export(dir.path(), &person()).unwrap();
    assert_eq!(first.bytes(), second.bytes());
    assert_eq!(
        seqs(&parsed(first.bytes())["events"]),
        ["0", "1", "3", "4", "8"]
    );
}

#[test]
fn a_broken_chain_refuses_the_whole_export() {
    let dir = tempfile::tempdir().unwrap();
    let (_, mut written) = city();
    let edited = String::from_utf8(written[4].clone())
        .unwrap()
        .replace("\"read\"", "\"reap\"");
    written[4] = edited.into_bytes();
    write(dir.path(), &written, b"");
    let refused = export(dir.path(), &person()).map(|bundle| bundle.events());
    assert_eq!(refused.map_err(|err| *err.code()), Err(AxCode::CasCorrupt));
}

#[test]
fn a_torn_tail_is_not_a_line_and_the_cutoff_is_the_last_whole_one() {
    let dir = tempfile::tempdir().unwrap();
    let (_, written) = city();
    write(dir.path(), &written, b"{\"v\":2,\"run\":\"half");
    confidential(dir.path(), "vault");
    let exported = export(dir.path(), &person()).unwrap();
    let cutoff = &parsed(exported.bytes())["source"]["cutoff"];
    assert_eq!(
        (
            cutoff["seq"].as_str().unwrap(),
            cutoff["chain_hash"].as_str().unwrap()
        ),
        (
            "8",
            kernel::ledger::chain_hash(&written[8]).to_string().as_str()
        )
    );
}

#[test]
fn a_confidential_line_reaches_no_table() {
    let (dir, _) = city();
    let withheld = export(dir.path(), &person()).unwrap();
    let resident = export(
        dir.path(),
        &Request {
            reader: Reader::Resident(addr("lab")),
            ..person()
        },
    )
    .unwrap();
    let included = export(
        dir.path(),
        &Request {
            reader: Reader::Person(Confidential::Included),
            ..person()
        },
    )
    .unwrap();
    let text = String::from_utf8(withheld.bytes().to_vec()).unwrap();
    let bundle = parsed(withheld.bytes());
    let runs: Vec<&str> = bundle["runs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["run"].as_str().unwrap())
        .collect();
    assert_eq!(
        (
            text.contains("secret-plan") || text.contains("vault/notes.md"),
            bundle["withheld"].clone(),
            runs,
            parsed(resident.bytes())["events"].clone() == bundle["events"],
            String::from_utf8(included.bytes().to_vec())
                .unwrap()
                .contains("secret-plan"),
        ),
        (
            false,
            json!({
                "events": "4",
                "kinds": [
                    {"kind": "building_created", "count": "1"},
                    {"kind": "run_started", "count": "1"},
                    {"kind": "tool_called", "count": "2"},
                ],
                "buildings": [{"building": "vault", "reason": "confidential"}],
                "credential": "0",
            }),
            vec![run(1).to_string().as_str()],
            true,
            true,
        )
    );
}

/// The question run 1 raises in `lab` and the answer the city writes on
/// its own run, with no address.
fn asked_and_answered() -> (Value, Value) {
    let cluster = kernel::ClusterKey {
        class: kernel::ApprovalClass::Question,
        detail: "push".to_owned(),
    };
    let item = kernel::ApprovalItem {
        id: kernel::ApprovalId::of(&run(1), Seq::new(1)),
        actor: "tester".to_owned(),
        action_desc: "push".to_owned(),
        artifact: kernel::Locator::Cas {
            hash: B3Hash::digest(b""),
            range: None,
        },
        cluster_key: cluster.clone(),
        created: kernel::TimeMs::new(0),
        tainted: false,
    };
    let answer = kernel::event::record::ApprovalResolved {
        id: item.id.clone(),
        verdict: kernel::Ruling::Allow,
        cluster,
    };
    (
        serde_json::to_value(item).unwrap(),
        serde_json::to_value(answer).unwrap(),
    )
}

#[test]
fn an_answer_on_the_city_run_is_context_outside_a_run_selection() {
    let dir = tempfile::tempdir().unwrap();
    let (asked, answered) = asked_and_answered();
    let written = lines(vec![
        (RunId::CITY, None, EventKind::CityInitialized, json!({})),
        (run(1), Some("lab/a"), EventKind::RunStarted, json!({})),
        (run(1), Some("lab/a"), EventKind::ApprovalRequested, asked),
        (RunId::CITY, None, EventKind::ApprovalResolved, answered),
    ]);
    write(dir.path(), &written, b"");
    let chosen = Request {
        selection: Selection::new(None, None, Some(run(1)), None).unwrap(),
        ..person()
    };
    let bundle = parsed(export(dir.path(), &chosen).unwrap().bytes());
    let approvals: Vec<(Value, Value)> = bundle["moments"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|moment| moment["family"] == "approval")
        .map(|moment| (moment["opened"].clone(), moment["closed"].clone()))
        .collect();
    assert_eq!(
        (approvals, seqs(&bundle["context"])),
        (
            vec![(json!({"at": "2"}), json!({"outside": "3"}))],
            vec!["3".to_owned()]
        )
    );
}

#[test]
fn a_merge_names_its_landed_commit_beside_the_checkpoints() {
    let dir = tempfile::tempdir().unwrap();
    let (checkpoint, landed) = ("1".repeat(40), "2".repeat(40));
    let written = lines(vec![
        (RunId::CITY, None, EventKind::CityInitialized, json!({})),
        (run(1), Some("lab/a"), EventKind::RunStarted, json!({})),
        (
            run(1),
            Some("lab/a"),
            EventKind::CheckpointCommitted,
            json!({"oid": checkpoint, "scope": ["lab"], "files": ["lab/a/notes.md"]}),
        ),
        (
            run(1),
            Some("lab/a"),
            EventKind::PrOpened,
            json!({"node": "1", "implementer": "lab/a", "branch": "lab-a", "commit": checkpoint}),
        ),
        (
            run(1),
            Some("lab/a"),
            EventKind::PrMerged,
            json!({
                "node": "1", "implementer": "lab/a", "branch": "lab-a",
                "reviewed_commit": checkpoint, "commit": landed, "verified_by": "lab/b",
            }),
        ),
    ]);
    write(dir.path(), &written, b"");
    let bundle = parsed(export(dir.path(), &person()).unwrap().bytes());
    let run = run(1).to_string();
    assert_eq!(
        bundle["checkpoints"],
        json!([
            {"seq": "2", "run": run, "holds": {"committed": {
                "oid": checkpoint, "scope": ["lab"], "files": ["lab/a/notes.md"],
                "base": "none", "diff": [],
                "trace": {"traced": {"calls": [], "nearby": []}}}}},
            {"seq": "4", "run": run, "holds": {"merged": {"oid": landed}}},
        ])
    );
}

#[test]
fn a_contradictory_range_is_refused_and_an_empty_one_is_a_bundle() {
    let (dir, _) = city();
    let contradiction = Selection::new(Some(Seq::new(5)), Some(Seq::new(2)), None, None);
    let empty = Request {
        selection: Selection::new(None, None, Some(run(9)), None).unwrap(),
        ..person()
    };
    let bundle = parsed(export(dir.path(), &empty).unwrap().bytes());
    assert_eq!(
        (
            contradiction.map(|_| ()).map_err(|err| *err.code()),
            bundle["events"].clone(),
            bundle["source"]["selection"]["run"].clone()
        ),
        (
            Err(AxCode::InvalidArgs),
            json!([]),
            json!(run(9).to_string())
        )
    );
}

mod checking;
mod evidence;
mod landing;
mod model;
mod page;
mod span;
mod tracing;
