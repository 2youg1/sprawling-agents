// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A city whose process died while it was writing its history opens
//! again by the rules (sprawling-SPEC.md 8-127): the half-written line is
//! cut and said, the chain verifies, a call whose answer was lost is
//! closed as unknown rather than as failed, and the views answer from
//! what survived, where the run that died is frozen.
//!
//! The death is made on the disk, not in the process: a real run writes
//! its history, the worker is dropped, which releases the writer's lock
//! as a killed process does, and the history is cut where the process
//! would have died - half way through the line that answered a call.
//! Every line after that one was never written. What a power cut adds,
//! the loss of bytes the platform had not yet made durable, is the
//! ledger's own contract and is held inside `storage` against its
//! fault filesystem.

use std::path::Path;

use kernel::event::record::ToolAnswer;
use kernel::{AxCode, EventKind, EventRecord, RunId, Seq};
use serde_json::json;

use crate::city::{self, History};
use crate::script::{self, Step, call_id};

/// What a city that reopened after the death says about itself.
#[derive(Debug, PartialEq)]
struct Reopened {
    /// What the startup scan reports opening cut, in its own words.
    opening: String,
    /// How many calls the scan closed.
    closed_calls: usize,
    /// The codes of every answer the dead call has now, in order.
    answers: Vec<Option<String>>,
    /// Whether the history says, in a line of its own, that it was cut.
    truncation_recorded: bool,
    /// Where the city view says each run it lists has got to.
    listed: Vec<Listed>,
    /// Whether the history says, on the freeze it wrote, that the process
    /// died: the run did not end, it was lost.
    death_recorded: bool,
    /// Whether the next task sent to the same room started a run and
    /// froze it: the room is not held by the run that died in it.
    room_works_again: bool,
}

#[test]
fn a_city_killed_while_writing_an_answer_reopens_with_the_torn_line_cut_and_the_call_unknown() {
    let dir = tempfile::tempdir().unwrap();
    let (factory, _, _) = script::scripted(vec![Step {
        tool: "status",
        args: json!({}),
    }]);
    let (mut worker, ledger) = city::city_with_a_model(dir.path(), factory);
    city::raise(&mut worker, "lab", "minimal");
    city::rules(
        dir.path(),
        "lab",
        "confidential = false\nwrite = \"everything\"\n",
    );
    city::dispatch(&mut worker, "lab/lead").unwrap();
    drop(worker);

    let torn = die_while_answering(&ledger, &call_id(1));

    let mut reopened = city::open_worker(dir.path(), script::scripted(Vec::new()).0);
    let scan = reopened.startup_scan().unwrap();
    let view = city_view(dir.path());
    city::dispatch(&mut reopened, "lab/lead").unwrap();
    drop(reopened);
    let history = History::read(&ledger);
    let started: Vec<RunId> = history.started().iter().map(|one| one.run).collect();
    let run = started[0];
    let answers = history
        .calls_of(run)
        .into_iter()
        .find(|call| call.called.id == call_id(1))
        .unwrap()
        .results
        .into_iter()
        .map(|result| match result.answer {
            ToolAnswer::Answered { .. } => None,
            ToolAnswer::Failed { error } => error
                .as_map()
                .get("code")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned),
        })
        .collect();
    assert_eq!(
        Reopened {
            opening: format!("{:?}", scan.opening),
            closed_calls: scan.closed_calls,
            answers,
            truncation_recorded: history.says(EventKind::LogTruncated, ""),
            listed: view
                .runs
                .iter()
                .map(|summary| Listed {
                    frozen: summary.frozen,
                    completion: summary.completion.clone(),
                    last_kind: summary.last_kind,
                    last_seq: summary.last_seq,
                })
                .collect(),
            death_recorded: history.says(EventKind::RunFrozen, "\"cause\":\"process_died\""),
            room_works_again: started.len() == 2 && history.wrote(started[1], EventKind::RunFrozen),
        },
        Reopened {
            opening: format!("TailDropped {{ bytes: {torn} }}"),
            closed_calls: 1,
            answers: vec![Some(AxCode::ToolOutcomeUnknown.as_str().to_owned())],
            truncation_recorded: true,
            listed: vec![Listed {
                frozen: true,
                completion: Some("cancelled".to_owned()),
                last_kind: EventKind::RunFrozen,
                last_seq: last_before(&ledger, run, started[1]),
            }],
            death_recorded: true,
            room_works_again: true,
        },
    );
}

/// One run as the city view lists it: whether it is frozen, how it
/// ended, and its last line.
#[derive(Debug, PartialEq)]
struct Listed {
    frozen: bool,
    completion: Option<String>,
    last_kind: EventKind,
    last_seq: Seq,
}

/// What the city view says, asked the way a one-shot question asks it.
fn city_view(city_root: &Path) -> wire::CityAnswer {
    let wire::Answer::City(view) =
        accounting::views::ask(city_root, &wire::Query::CityView).unwrap()
    else {
        panic!("a city question is answered with the city");
    };
    view
}

/// The seq of the last line `run` wrote before `next` started, read off
/// the verified history.
fn last_before(ledger_dir: &Path, run: RunId, next: RunId) -> Seq {
    runtime::replay::verify_ledger_dir(ledger_dir)
        .unwrap()
        .raw_lines()
        .iter()
        .map(|line| EventRecord::parse_line(line).unwrap())
        .take_while(|record| record.run() != next)
        .filter(|record| record.run() == run)
        .map(|record| record.seq())
        .last()
        .unwrap()
}

/// Cuts the history half way through the line that answered `call`,
/// as a process that died while writing it leaves it, and answers how
/// many bytes of that line reached the disk.
fn die_while_answering(ledger_dir: &Path, call: &str) -> u64 {
    let needle = format!("\"tool_use_id\":\"{call}\"");
    let segment = storage::ledger_segments_at(ledger_dir)
        .unwrap()
        .into_iter()
        .find(|segment| String::from_utf8_lossy(&std::fs::read(segment).unwrap()).contains(&needle))
        .unwrap();
    let bytes = std::fs::read(&segment).unwrap();
    let text = String::from_utf8_lossy(&bytes);
    let mut start = 0usize;
    let line = text
        .split_inclusive('\n')
        .find(|line| {
            let found = line.contains("\"kind\":\"tool_result\"") && line.contains(&needle);
            if !found {
                start = start.checked_add(line.len()).unwrap();
            }
            found
        })
        .unwrap();
    let kept = line.len() / 2;
    std::fs::write(&segment, &bytes[..start.checked_add(kept).unwrap()]).unwrap();
    for later in storage::ledger_segments_at(ledger_dir)
        .unwrap()
        .into_iter()
        .filter(|later| *later > segment)
    {
        std::fs::remove_file(later).unwrap();
    }
    u64::try_from(kept).unwrap()
}
