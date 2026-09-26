// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]

use super::{Selection, kind_named, write_records, write_runs};
use kernel::{Address, B3Hash, EventDraft, EventKind, EventRecord, Payload, RunId, Seq, TimeMs};
use serde_json::{Value, json};
use std::path::Path;

fn run(n: u8) -> RunId {
    RunId::parse(&format!("0198f6a2-7c4a-7bbb-9d1e-0000000000{n:02}")).unwrap()
}

/// Two rooms, a fork, a successor and a second stretch of one room,
/// written as one segment; returns the raw lines as the file holds them.
fn write_city_ledger(dir: &Path) -> Vec<Vec<u8>> {
    let lab = Some(Address::parse("lab/a").unwrap());
    let yard = Some(Address::parse("yard/b").unwrap());
    let script: Vec<(RunId, Option<Address>, EventKind, Value)> = vec![
        (RunId::CITY, None, EventKind::CityInitialized, json!({})),
        (
            run(1),
            lab.clone(),
            EventKind::RunStarted,
            json!({"task": "E_TOOL x"}),
        ),
        (
            run(1),
            lab.clone(),
            EventKind::ToolCalled,
            json!({"tool": "read"}),
        ),
        (run(2), yard.clone(), EventKind::RunStarted, json!({})),
        (
            run(1),
            lab.clone(),
            EventKind::ToolCalled,
            json!({"tool": "E_TOOL"}),
        ),
        (
            run(3),
            lab.clone(),
            EventKind::RunForked,
            json!({"from": run(1).to_string(), "at_seq": 2}),
        ),
        (
            run(2),
            yard.clone(),
            EventKind::ToolCalled,
            json!({"tool": "write"}),
        ),
        (
            RunId::CITY,
            lab.clone(),
            EventKind::SessionOpened,
            json!({"carried": false}),
        ),
        (
            run(4),
            lab.clone(),
            EventKind::RunStarted,
            json!({"predecessor": run(1).to_string(), "parent": run(2).to_string()}),
        ),
        (
            run(1),
            lab.clone(),
            EventKind::RunFrozen,
            json!({"completion": "done"}),
        ),
        (
            run(4),
            lab,
            EventKind::ToolCalled,
            json!({"tool": "E_TOOL"}),
        ),
    ];
    let mut prev = B3Hash::digest(b"");
    let mut blob = Vec::new();
    let mut lines = Vec::new();
    for (seq, (run, addr, kind, data)) in script.into_iter().enumerate() {
        let draft = EventDraft {
            run,
            t: TimeMs::new(u64::try_from(seq).unwrap()),
            who: "tester".to_owned(),
            addr,
            kind,
            data: Payload::new(data.as_object().cloned().unwrap()).unwrap(),
            ig: false,
        };
        let line = EventRecord::from_draft(draft, Seq::new(u64::try_from(seq).unwrap()), prev)
            .canonical_line()
            .unwrap();
        prev = B3Hash::digest(&line);
        blob.extend_from_slice(&line);
        blob.push(b'\n');
        lines.push(line);
    }
    std::fs::write(dir.join("ledger-00000000000000000000.jsonl"), &blob).unwrap();
    lines
}

fn viewed(dir: &Path, chosen: &Selection) -> Vec<u8> {
    let mut out = Vec::new();
    write_records(dir, chosen, &mut out).unwrap();
    out
}

/// What a person gets by reading the same file with grep and jq: the
/// raw lines, in file order, that pass every condition.
fn grepped(lines: &[Vec<u8>], keep: impl Fn(&Value, &[u8]) -> bool, tail: usize) -> Vec<u8> {
    let kept: Vec<&Vec<u8>> = lines
        .iter()
        .filter(|line| keep(&serde_json::from_slice(line).unwrap(), line))
        .collect();
    let skip = kept.len().saturating_sub(tail);
    kept.into_iter()
        .skip(skip)
        .flat_map(|line| line.iter().copied().chain(*b"\n"))
        .collect()
}

fn holds(line: &[u8], text: &str) -> bool {
    line.windows(text.len()).any(|w| w == text.as_bytes())
}

/// Every filter the records lens takes, alone and together, answers
/// byte for byte what grep answers over the same file.
#[test]
fn records_are_byte_for_byte_what_grep_finds_in_the_same_ledger() {
    let dir = tempfile::tempdir().unwrap();
    let lines = write_city_ledger(dir.path());
    let all = usize::MAX;
    let cases: Vec<(Selection, Vec<u8>)> = vec![
        (
            Selection {
                kind: Some(EventKind::ToolCalled),
                ..Selection::default()
            },
            grepped(&lines, |v, _| v["kind"] == "tool_called", all),
        ),
        (
            Selection {
                who: Some("lab".to_owned()),
                ..Selection::default()
            },
            grepped(
                &lines,
                |v, _| v["addr"].as_str().is_some_and(|a| a.starts_with("lab")),
                all,
            ),
        ),
        (
            Selection {
                grep: Some("E_TOOL".to_owned()),
                from: Some(Seq::new(2)),
                ..Selection::default()
            },
            grepped(
                &lines,
                |v, l| holds(l, "E_TOOL") && v["seq"].as_u64() >= Some(2),
                all,
            ),
        ),
        (
            Selection {
                run: Some(run(1)),
                kind: Some(EventKind::ToolCalled),
                tail: Some(1),
                ..Selection::default()
            },
            grepped(
                &lines,
                |v, _| v["run"] == run(1).to_string() && v["kind"] == "tool_called",
                1,
            ),
        ),
        (
            Selection {
                tail: Some(3),
                ..Selection::default()
            },
            grepped(&lines, |_, _| true, 3),
        ),
    ];
    for (chosen, expected) in cases {
        assert_eq!(
            String::from_utf8(viewed(dir.path(), &chosen)).unwrap(),
            String::from_utf8(expected).unwrap(),
            "{chosen:?}"
        );
    }
}

/// Every parent pointer `--runs` writes is the one the ledger wrote.
#[test]
fn runs_carry_the_parent_pointers_the_ledger_wrote() {
    let dir = tempfile::tempdir().unwrap();
    write_city_ledger(dir.path());
    let mut out = Vec::new();
    write_runs(dir.path(), &mut out).unwrap();
    let got: Vec<Value> = out
        .split(|b| *b == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_slice(line).unwrap())
        .collect();
    let line = |r: RunId,
                addr: Option<&str>,
                session: Option<u64>,
                parent: Option<RunId>,
                forked_at: Option<u64>,
                predecessor: Option<RunId>,
                seqs: (u64, u64),
                state: &str| {
        json!({
            "run": r.to_string(), "addr": addr, "session": session,
            "parent": parent.map(|p| p.to_string()), "forked_at": forked_at,
            "predecessor": predecessor.map(|p| p.to_string()),
            "first_seq": seqs.0, "last_seq": seqs.1, "state": state,
        })
    };
    assert_eq!(
        got,
        vec![
            line(
                run(1),
                Some("lab/a"),
                None,
                None,
                None,
                None,
                (1, 9),
                "frozen"
            ),
            line(
                run(2),
                Some("yard/b"),
                None,
                None,
                None,
                None,
                (3, 6),
                "active"
            ),
            line(
                run(3),
                None,
                None,
                Some(run(1)),
                Some(2),
                None,
                (5, 5),
                "active"
            ),
            line(
                run(4),
                Some("lab/a"),
                Some(7),
                Some(run(2)),
                None,
                Some(run(1)),
                (8, 10),
                "active"
            ),
        ]
    );
}

/// A misspelt kind is a command-line error that names the close ones.
#[test]
fn a_misspelt_kind_names_the_kinds_it_was_close_to() {
    assert_eq!(kind_named("tool_called"), Ok(EventKind::ToolCalled));
    let said = kind_named("tool_caled").unwrap_err();
    assert!(
        said.contains("Did you mean") && said.contains("'tool_called'"),
        "{said}"
    );
}
