// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A fixture city with a history (citysim-SPEC.md section 8-5-1).
//!
//! Shape: adapter. The city is raised by `init_city` and its history is
//! written through `storage::JsonlLedger::append_all`, so the segments,
//! the chain and the canonical bytes are the product's and no line is
//! spelled here. What this file owns is the shape of one run's history,
//! which is sized like the records of a city that has been working.
//!
//! A city is generated under a `.partial` name and renamed when it is
//! whole, so an interrupted generation is regenerated rather than reused.

use std::path::{Path, PathBuf};

use kernel::{Address, AxCode, AxError, EventDraft, EventKind, Payload, RunId, TimeMs};
use serde_json::{Value, json};

/// How much history a fixture city carries.
pub enum History {
    /// The three records `init` writes.
    Empty,
    /// That many runs of fifty records each, after the genesis.
    Runs(u32),
}

/// Turns per run: with `run_started` and `run_frozen`, fifty records.
const TURNS: u32 = 8;
/// Rooms the runs are spread over.
const ROOMS: u32 = 40;
/// Runs handed to the ledger per `append_all`: 10,000 records, so the
/// drafts held at once stay a few megabytes whatever the city's length.
const RUNS_PER_WAVE: u32 = 200;

/// The city `name` under `cities`, generated if it is not there yet.
///
/// # Errors
/// Propagates the product's refusals while raising the city or writing
/// its history, and the filesystem's while clearing or renaming.
pub fn fixture_city(cities: &Path, name: &str, history: History) -> Result<PathBuf, AxError> {
    let city = cities.join(name);
    if city.is_dir() {
        return Ok(city);
    }
    let partial = cities.join(format!("{name}.partial"));
    if partial.exists() {
        std::fs::remove_dir_all(&partial)
            .map_err(|err| disk("clear a partial city", &partial, &err))?;
    }
    let raised = sprawling::assembly::init_city(&partial)?;
    if let History::Runs(runs) = history {
        write_runs(&raised.ledger_dir, runs)?;
    }
    std::fs::rename(&partial, &city).map_err(|err| disk("keep a fixture city", &city, &err))?;
    Ok(city)
}

fn write_runs(ledger_dir: &Path, runs: u32) -> Result<(), AxError> {
    let lines = storage::read_raw_lines_at(ledger_dir).map_err(storage::StorageError::into_ax)?;
    let last = lines.last().ok_or_else(|| {
        AxError::failure(
            AxCode::EvidenceMissing,
            "continue a raised city",
            ledger_dir.display().to_string(),
        )
        .with_recovery("init writes a genesis line; delete the fixture city and run again")
    })?;
    let mut t = kernel::EventRecord::parse_line(last)?.t().value();
    let (mut ledger, _report) = storage::JsonlLedger::open(ledger_dir, TimeMs::new(t))
        .map_err(storage::StorageError::into_ax)?;
    let mut run = 0;
    while run < runs {
        let wave_end = run.saturating_add(RUNS_PER_WAVE).min(runs);
        let mut drafts = Vec::new();
        for index in run..wave_end {
            one_run(index, &mut t, &mut drafts)?;
        }
        ledger
            .append_all(drafts)
            .map_err(storage::StorageError::into_ax)?;
        run = wave_end;
    }
    Ok(())
}

/// One run's fifty drafts, sized like the records of a working city.
fn one_run(index: u32, t: &mut u64, into: &mut Vec<EventDraft>) -> Result<(), AxError> {
    let [a, b, c, d] = index.to_le_bytes();
    let run = RunId::from_bytes([a, b, c, d, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x5e]);
    let room = Address::parse(&format!("hall/r{}", index.checked_rem(ROOMS).unwrap_or(0)))?;
    let who = format!("worker@{}.1", room.as_str().replace('/', "."));
    let mut push = |kind: EventKind, data: Value, addr: Option<&Address>| -> Result<(), AxError> {
        *t = t.saturating_add(1);
        let Value::Object(map) = data else {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "draft a fixture record",
                "a payload that is not an object",
            )
            .with_recovery("this is a defect in bench_startup's history shape"));
        };
        into.push(EventDraft {
            run,
            t: TimeMs::new(*t),
            who: who.clone(),
            addr: addr.cloned(),
            kind,
            data: Payload::new(map)?,
            ig: false,
        });
        Ok(())
    };
    push(
        EventKind::RunStarted,
        json!({"goal": prose(60), "task": prose(120), "skills": []}),
        Some(&room),
    )?;
    for turn in 0..TURNS {
        let hashes: Vec<String> = (0..4)
            .map(|slot| {
                kernel::B3Hash::digest(format!("{index}.{turn}.{slot}").as_bytes()).to_string()
            })
            .collect();
        let segments: Vec<Value> = ["city", "building", "resident", "run"]
            .iter()
            .zip(&hashes)
            .map(|(slot, hash)| json!({"hash": hash, "len": 400, "skipped": [], "slot": slot, "sources": []}))
            .collect();
        push(
            EventKind::PromptAssembled,
            json!({"breakpoints": ["city", "building", "resident", "run"], "segments": segments}),
            None,
        )?;
        push(
            EventKind::ModelCalled,
            json!({"model": "m", "segments": hashes}),
            None,
        )?;
        push(
            EventKind::ModelReturned,
            json!({"calls": 1, "billed_usd_micros": 1200, "message": {"content": [{"kind": "text", "text": prose(700)}]}}),
            None,
        )?;
        let call = format!("call-{index}-{turn}");
        push(
            EventKind::ToolCalled,
            json!({"args": {"cmd": prose(80)}, "id": call, "name": "exec"}),
            None,
        )?;
        push(
            EventKind::ToolResult,
            json!({"name": "exec", "result": {"content": prose(2500)}, "tool_use_id": call}),
            None,
        )?;
        let oid = hashes
            .first()
            .map_or(String::new(), |hash| hash.chars().take(40).collect());
        push(
            EventKind::CheckpointCommitted,
            json!({"oid": oid, "scope": ["hall"], "files": ["hall/a.rs", "hall/b.rs"], "model": "m", "effort": "none"}),
            Some(&room),
        )?;
    }
    push(
        EventKind::RunFrozen,
        json!({"completion": "done"}),
        Some(&room),
    )
}

/// Ordinary prose of about `bytes` bytes.
fn prose(bytes: usize) -> String {
    "the parser reads a line and writes a record then checks the chain before it moves on "
        .chars()
        .cycle()
        .take(bytes)
        .collect()
}

fn disk(action: &'static str, path: &Path, err: &std::io::Error) -> AxError {
    AxError::failure(
        AxCode::StorageFatal,
        action,
        format!("{}: {err}", path.display()),
    )
    .with_recovery("free space beside the build directory and run again")
}
