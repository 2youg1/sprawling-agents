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

use super::{Audience, Selection, kind_named, write_records, write_runs};
use kernel::{Address, B3Hash, EventDraft, EventKind, EventRecord, Payload, RunId, Seq, TimeMs};
use serde_json::{Value, json};
use std::path::Path;

pub(super) fn run(n: u8) -> RunId {
    RunId::parse(&format!("0198f6a2-7c4a-7bbb-9d1e-0000000000{n:02}")).unwrap()
}

/// Two rooms, a fork, a successor and a second stretch of one room,
/// written as one segment; returns the raw lines as the file holds them.
/// The question `asker` raises, and the answer that closes it.
fn question(asker: RunId) -> (Value, Value) {
    let cluster = kernel::ClusterKey {
        class: kernel::ApprovalClass::Question,
        detail: "push".to_owned(),
    };
    let item = kernel::ApprovalItem {
        id: kernel::ApprovalId::of(&asker, Seq::new(1)),
        actor: "tester".to_owned(),
        action_desc: "push".to_owned(),
        artifact: kernel::Locator::Cas {
            hash: B3Hash::digest(b""),
            range: None,
        },
        cluster_key: cluster.clone(),
        created: TimeMs::new(0),
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

pub(super) fn write_city_ledger(dir: &Path) -> Vec<Vec<u8>> {
    let lab = Some(Address::parse("lab/a").unwrap());
    let yard = Some(Address::parse("yard/b").unwrap());
    let (asked_by_3, _) = question(run(3));
    let (asked_by_2, answered_2) = question(run(2));
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
            lab.clone(),
            EventKind::ToolCalled,
            json!({"tool": "E_TOOL"}),
        ),
        (run(3), lab, EventKind::ApprovalRequested, asked_by_3),
        (run(2), yard, EventKind::ApprovalRequested, asked_by_2),
        (RunId::CITY, None, EventKind::ApprovalResolved, answered_2),
    ];
    let mut prev = kernel::GENESIS_PREV;
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

/// The first window of a short ledger is all of it, and the whole fold
/// arrives in a later poll; a run that starts after that is in the next
/// poll, with only the line that started it; a poll with nothing new
/// takes nothing.
#[test]
fn follow_takes_a_run_that_started_after_the_viewer_opened() {
    let dir = tempfile::tempdir().unwrap();
    let lines = write_city_ledger(dir.path());
    let (mut follow, (runs, rows)) = super::follow::Follow::open(dir.path()).unwrap();
    assert_eq!((runs.len(), rows.len()), (4, lines.len()));
    let filled = (0..10_000).find_map(|_| {
        std::thread::sleep(std::time::Duration::from_millis(1));
        follow.poll().unwrap()
    });
    let Some(super::follow::Polled::Filled((_, whole))) = filled else {
        panic!("the whole fold never arrived");
    };
    assert_eq!(whole, rows);
    let draft = EventDraft {
        run: run(5),
        t: TimeMs::new(99),
        who: "tester".to_owned(),
        addr: Some(Address::parse("lab/c").unwrap()),
        kind: EventKind::RunStarted,
        data: Payload::new(serde_json::Map::new()).unwrap(),
        ig: false,
    };
    let seq = Seq::new(u64::try_from(lines.len()).unwrap());
    let prev = B3Hash::digest(lines.last().unwrap());
    let started = EventRecord::from_draft(draft, seq, prev)
        .canonical_line()
        .unwrap();
    let mut segment = std::fs::OpenOptions::new()
        .append(true)
        .open(dir.path().join("ledger-00000000000000000000.jsonl"))
        .unwrap();
    std::io::Write::write_all(&mut segment, &[started.as_slice(), b"\n"].concat()).unwrap();
    let Some(super::follow::Polled::Appended((runs, rows))) = follow.poll().unwrap() else {
        panic!("the appended run never arrived");
    };
    assert_eq!(runs.last().map(|line| line.run), Some(run(5)));
    assert_eq!(
        rows.iter()
            .map(|row| (row.seq, row.line.as_bytes()))
            .collect::<Vec<_>>(),
        vec![(seq, started.as_slice())]
    );
    assert!(follow.poll().unwrap().is_none());
}

pub(super) fn viewed(dir: &Path, chosen: &Selection, audience: Audience) -> Vec<u8> {
    let mut out = Vec::new();
    write_records(dir, chosen, audience, &mut out).unwrap();
    out
}

/// What a person gets by reading the same file with grep and jq: the
/// raw lines, in file order, that pass every condition.
pub(super) fn grepped(
    lines: &[Vec<u8>],
    keep: impl Fn(&Value, &[u8]) -> bool,
    tail: usize,
) -> Vec<u8> {
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
            String::from_utf8(viewed(dir.path(), &chosen, Audience::Agent)).unwrap(),
            String::from_utf8(expected).unwrap(),
            "{chosen:?}"
        );
    }
}

/// At a terminal each line is led by its chain hash, which is the
/// `prev` the next line holds, and then the line byte for byte.
#[test]
fn the_records_lens_names_each_line_by_its_chain_hash() {
    let dir = tempfile::tempdir().unwrap();
    let lines = write_city_ledger(dir.path());
    let shown = viewed(dir.path(), &Selection::default(), Audience::Person);
    let next_prevs = lines[1..]
        .iter()
        .map(|line| EventRecord::parse_line(line).unwrap().prev().to_string())
        .chain([B3Hash::digest(lines.last().unwrap()).to_string()]);
    let expected: Vec<u8> = lines
        .iter()
        .zip(next_prevs)
        .flat_map(|(line, hash)| [hash.as_bytes(), b"  ", line.as_slice(), b"\n"].concat())
        .collect();
    assert_eq!(
        String::from_utf8(shown).unwrap(),
        String::from_utf8(expected).unwrap()
    );
}

/// Every parent pointer `--runs` writes is the one the ledger wrote, and
/// every question a run raised counts until its answer lands.
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
                (state, unanswered): (&str, usize)| {
        json!({
            "run": r.to_string(), "addr": addr, "session": session,
            "parent": parent.map(|p| p.to_string()), "forked_at": forked_at,
            "predecessor": predecessor.map(|p| p.to_string()),
            "first_seq": seqs.0, "last_seq": seqs.1, "state": state,
            "unanswered": unanswered,
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
                ("frozen", 0)
            ),
            line(
                run(2),
                Some("yard/b"),
                None,
                None,
                None,
                None,
                (3, 12),
                ("active", 0)
            ),
            line(
                run(3),
                None,
                None,
                Some(run(1)),
                Some(2),
                None,
                (5, 11),
                ("active", 1)
            ),
            line(
                run(4),
                Some("lab/a"),
                Some(7),
                Some(run(2)),
                None,
                Some(run(1)),
                (8, 10),
                ("active", 0)
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
