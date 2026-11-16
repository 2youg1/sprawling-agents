// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use kernel::event::record::ToolAnswer;
use kernel::{AxCode, EventKind, EventRecord, RunId, Seq};
use serde_json::json;

use crate::city::{self, History};
use crate::script::{self, Step, call_id};

#[derive(Debug, PartialEq)]
struct Reopened {
    opening: String,

    closed_calls: usize,

    answers: Vec<Option<String>>,

    truncation_recorded: bool,

    listed: Vec<Listed>,

    death_recorded: bool,

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

    let mut reopened = city::open_worker(dir.path(), Box::new(script::scripted(Vec::new()).0));
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

#[derive(Debug, PartialEq)]
struct Listed {
    frozen: bool,
    completion: Option<String>,
    last_kind: EventKind,
    last_seq: Seq,
}

fn city_view(city_root: &Path) -> wire::CityAnswer {
    let wire::Answer::City(view) =
        accounting::views::ask(city_root, &wire::Query::CityView).unwrap()
    else {
        panic!("a city question is answered with the city");
    };
    view
}

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

#[derive(Debug, PartialEq)]
struct ReopenedWhole {
    opening: String,
    closed_calls: usize,
    listed: Vec<(bool, Option<String>, EventKind)>,
    death_recorded: bool,
    verifies: bool,
    room_works_again: bool,
}

#[test]
fn a_city_killed_between_a_job_put_and_its_checkpoint_line_reopens_with_nothing_to_cut() {
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

    die_before(&ledger, "\"kind\":\"checkpoint_committed\"", "\"job\"");

    let mut reopened = city::open_worker(dir.path(), Box::new(script::scripted(Vec::new()).0));
    let scan = reopened.startup_scan().unwrap();
    let view = city_view(dir.path());
    city::dispatch(&mut reopened, "lab/lead").unwrap();
    drop(reopened);
    let history = History::read(&ledger);
    let started: Vec<RunId> = history.started().iter().map(|one| one.run).collect();
    assert_eq!(
        ReopenedWhole {
            opening: format!("{:?}", scan.opening),
            closed_calls: scan.closed_calls,
            listed: view
                .runs
                .iter()
                .map(|summary| (
                    summary.frozen,
                    summary.completion.clone(),
                    summary.last_kind
                ))
                .collect(),
            death_recorded: history.says(EventKind::RunFrozen, "\"cause\":\"process_died\""),
            verifies: runtime::replay::verify_ledger_dir(&ledger).is_ok(),
            room_works_again: started.len() == 1 && history.wrote(started[0], EventKind::RunFrozen),
        },
        ReopenedWhole {
            opening: "Intact".to_owned(),
            closed_calls: 0,
            listed: Vec::new(),
            death_recorded: false,
            verifies: true,
            room_works_again: true,
        },
    );
}

fn die_before(ledger_dir: &Path, kind: &str, marker: &str) {
    let hit = |line: &str| line.contains(kind) && line.contains(marker);
    let segment = storage::ledger_segments_at(ledger_dir)
        .unwrap()
        .into_iter()
        .find(|segment| {
            String::from_utf8_lossy(&std::fs::read(segment).unwrap())
                .lines()
                .any(hit)
        })
        .unwrap_or_else(|| {
            let kinds: Vec<String> = storage::ledger_segments_at(ledger_dir)
                .unwrap()
                .iter()
                .flat_map(|segment| {
                    String::from_utf8_lossy(&std::fs::read(segment).unwrap())
                        .lines()
                        .filter(|line| line.contains("checkpoint"))
                        .map(str::to_owned)
                        .collect::<Vec<_>>()
                })
                .collect();
            panic!("no line carries {kind} and {marker}: {kinds:#?}")
        });
    let bytes = std::fs::read(&segment).unwrap();
    let text = String::from_utf8_lossy(&bytes);
    let kept: usize = text
        .split_inclusive('\n')
        .take_while(|line| !hit(line))
        .map(str::len)
        .sum();
    std::fs::write(&segment, &bytes[..kept]).unwrap();
    for later in storage::ledger_segments_at(ledger_dir)
        .unwrap()
        .into_iter()
        .filter(|later| *later > segment)
    {
        std::fs::remove_file(later).unwrap();
    }
}

fn a_run_that_wrote(city_root: &Path) -> std::path::PathBuf {
    let (factory, _, _) = script::scripted(vec![Step {
        tool: "edit",
        args: json!({
            "file_path": "lab/lead/notes.md",
            "old_string": "",
            "new_string": "# Notes\n",
        }),
    }]);
    let (mut worker, ledger) = city::city_with_a_model(city_root, factory);
    city::raise(&mut worker, "lab", "minimal");
    city::rules(
        city_root,
        "lab",
        "confidential = false\nwrite = \"everything\"\n",
    );
    city::dispatch(&mut worker, "lab/lead").unwrap();
    drop(worker);
    ledger
}

fn written_by_the_call(city_root: &Path) -> Vec<std::path::PathBuf> {
    let mut found = Vec::new();
    let mut open = vec![city_root.to_path_buf()];
    while let Some(dir) = open.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() && path.file_name() != Some(std::ffi::OsStr::new(".git")) {
                open.push(path);
            } else if path.file_name() == Some(std::ffi::OsStr::new("notes.md")) {
                found.push(path);
            }
        }
    }
    found
}

fn lines_of(ledger_dir: &Path) -> Vec<Vec<u8>> {
    runtime::replay::verify_ledger_dir(ledger_dir)
        .unwrap()
        .raw_lines()
        .to_vec()
}

#[derive(Debug, PartialEq)]
struct ReopenedAfterTheSwap {
    pinned: bool,

    kept: bool,

    named: usize,

    calls: usize,

    effect_repeated: bool,
    death_recorded: bool,
    verifies: bool,
    room_works_again: bool,
}

#[test]
fn a_city_killed_after_a_checkpoint_swap_and_before_its_line_loses_and_doubles_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let ledger = a_run_that_wrote(dir.path());
    let committed = History::read(&ledger);
    let run = committed.started()[0].run;
    let oid = lines_of(&ledger)
        .iter()
        .map(|raw| serde_json::from_slice::<serde_json::Value>(raw).unwrap())
        .find(|line| {
            line["kind"] == "checkpoint_committed" && line["run"] == run.to_string().as_str()
        })
        .and_then(|line| line["data"]["oid"].as_str().map(str::to_owned))
        .unwrap();

    die_before(&ledger, "\"kind\":\"checkpoint_committed\"", &oid);

    for path in written_by_the_call(dir.path()) {
        std::fs::remove_file(path).unwrap();
    }
    let before = lines_of(&ledger);
    let pinned = dir
        .path()
        .join(".git/refs/sprawling/runs")
        .join(run.to_string())
        .join(&oid)
        .is_file();

    let mut reopened = city::open_worker(dir.path(), Box::new(script::scripted(Vec::new()).0));
    reopened.startup_scan().unwrap();
    city::dispatch(&mut reopened, "lab/lead").unwrap();
    drop(reopened);
    let after = lines_of(&ledger);
    let history = History::read(&ledger);
    let started: Vec<RunId> = history.started().iter().map(|one| one.run).collect();
    assert_eq!(
        ReopenedAfterTheSwap {
            pinned,
            kept: after.starts_with(&before),
            named: after
                .iter()
                .filter(|raw| String::from_utf8_lossy(raw).contains(&oid))
                .count(),
            calls: history.calls_of(run).len(),
            effect_repeated: !written_by_the_call(dir.path()).is_empty(),
            death_recorded: history.says(EventKind::RunFrozen, "\"cause\":\"process_died\""),
            verifies: runtime::replay::verify_ledger_dir(&ledger).is_ok(),
            room_works_again: started.len() == 2 && history.wrote(started[1], EventKind::RunFrozen),
        },
        ReopenedAfterTheSwap {
            pinned: true,
            kept: true,
            named: 0,
            calls: 0,
            effect_repeated: false,
            death_recorded: true,
            verifies: true,
            room_works_again: true,
        },
    );
}

#[derive(Debug, PartialEq)]
struct ReopenedMidBarrier {
    opening: String,

    kept: bool,

    first_of_the_wave: usize,
    death_recorded: bool,
    verifies: bool,
    room_works_again: bool,
}

#[test]
fn a_city_killed_inside_a_barrier_keeps_the_whole_lines_once_and_cuts_the_torn_one() {
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

    let written = lines_of(&ledger);
    let segments = storage::ledger_segments_at(&ledger).unwrap();
    let segment = segments.last().unwrap();
    let bytes = std::fs::read(segment).unwrap();
    let lines: Vec<&[u8]> = bytes.split_inclusive(|byte| *byte == b'\n').collect();
    let kind_at = |at: usize| {
        EventRecord::parse_line(&lines[at][..lines[at].len() - 1])
            .unwrap()
            .kind()
    };
    let called: Vec<usize> = (0..lines.len())
        .filter(|at| kind_at(*at) == EventKind::ModelCalled)
        .collect();
    let wave = called[called.len() - 2] + 1..called[called.len() - 1];
    assert!(
        wave.len() >= 2,
        "the wave between the two model calls holds {:?}",
        wave.clone().map(kind_at).collect::<Vec<_>>()
    );
    let start: usize = lines[..wave.start].iter().map(|line| line.len()).sum();
    let whole = lines[wave.start];
    let torn = lines[wave.start + 1].len() / 2;
    std::fs::write(segment, &bytes[..start + whole.len() + torn]).unwrap();
    let earlier = written.len() - lines.len();
    let before = &written[..earlier + wave.start + 1];

    let mut reopened = city::open_worker(dir.path(), Box::new(script::scripted(Vec::new()).0));
    let scan = reopened.startup_scan().unwrap();
    city::dispatch(&mut reopened, "lab/lead").unwrap();
    drop(reopened);
    let after = lines_of(&ledger);
    let history = History::read(&ledger);
    let started: Vec<RunId> = history.started().iter().map(|one| one.run).collect();
    let whole_line = &whole[..whole.len() - 1];
    assert_eq!(
        ReopenedMidBarrier {
            opening: format!("{:?}", scan.opening),
            kept: after.starts_with(before),
            first_of_the_wave: after
                .iter()
                .filter(|raw| raw.as_slice() == whole_line)
                .count(),
            death_recorded: history.says(EventKind::RunFrozen, "\"cause\":\"process_died\""),
            verifies: runtime::replay::verify_ledger_dir(&ledger).is_ok(),
            room_works_again: started.len() == 2 && history.wrote(started[1], EventKind::RunFrozen),
        },
        ReopenedMidBarrier {
            opening: format!("TailDropped {{ bytes: {torn} }}"),
            kept: true,
            first_of_the_wave: 1,
            death_recorded: true,
            verifies: true,
            room_works_again: true,
        },
    );
}
