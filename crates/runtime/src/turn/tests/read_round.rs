// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a read costs end to end: from the moment the model's reply with
//! one `read` call is in hand to the moment the next request is handed
//! to the model, through the real `ReadTool` and a real `JsonlLedger` on
//! disk. The gap holds the reply's accounting, the wave (admission, the
//! read, the secret scan, `settle`), the barrier that makes the turn's
//! lines durable (`append_all` and its sync, runtime D36) and the next
//! assembly. The phase readings beside it time the read alone and the
//! barrier alone, so the share each has of the gap is visible.

#![allow(
    clippy::disallowed_methods,
    clippy::arithmetic_side_effects,
    reason = "test code: an instrument reads the wall clock"
)]

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::super::*;
use super::helpers::*;
use crate::bench::{BenchOutcome, Clearance, Ticket, ToolBench};
use crate::catalog::Catalog;
use crate::handoff::Handoff;
use crate::run::{RunHooks, SafePoint};
use kernel::{IdemKey, ModelRequest, Seq, Tool, ToolOutcome};

/// A face over a bench holding the real `read`, timing each call from
/// its admission to its accounting, which holds the read itself.
struct Reads {
    bench: ToolBench,
    next: u64,
    admitted: Option<Instant>,
    spans: Vec<Duration>,
}

impl Reads {
    fn new(city: &Path) -> Reads {
        let domain =
            kernel::WriteDomain::new(vec![kernel::Address::parse("lab").unwrap()]).unwrap();
        let mut bench = ToolBench::new(domain);
        let everywhere: crate::tools::ReadBound =
            Arc::new(|_: &kernel::Address| kernel::ReadVerdict::Open);
        bench
            .register(Box::new(
                crate::tools::ReadTool::new(
                    city,
                    Arc::new(Mutex::new(Catalog::new())),
                    everywhere,
                    Path::new("no-store"),
                )
                .unwrap(),
            ))
            .unwrap();
        Reads {
            bench,
            next: 0,
            admitted: None,
            spans: Vec::new(),
        }
    }
}

fn answer(outcome: BenchOutcome) -> Result<ToolOutcome, AxError> {
    match outcome {
        BenchOutcome::Ran { outcome, .. } | BenchOutcome::Duplicate { outcome } => Ok(outcome),
        BenchOutcome::Refused { refusal } => Err(*refusal),
    }
}

impl ConcurrentInvoke for Reads {
    fn meta_of(&self, call: &ToolCall) -> Option<&kernel::ToolMeta> {
        self.bench.meta_of(&call.name)
    }

    fn admit(&mut self, call: &ToolCall, t: TimeMs) -> Admitted {
        self.admitted = Some(Instant::now());
        let at = self.next;
        self.next = at + 1;
        let key = IdemKey::derive(&run_id(), Seq::new(at), b"read_round");
        match self.bench.clear(call, &key, t) {
            Ok(Clearance::Cleared(ticket)) => Admitted::Cleared(ticket),
            Ok(Clearance::Answered(outcome)) => Admitted::Answered(answer(outcome)),
            Err(refused) => Admitted::Answered(Err(refused)),
        }
    }

    fn tool(&self, ticket: &Ticket) -> Result<&dyn Tool, AxError> {
        self.bench.tool_for(ticket)
    }

    fn account(
        &mut self,
        _call: &ToolCall,
        ticket: Ticket,
        answered: Result<ToolOutcome, AxError>,
    ) -> Result<ToolOutcome, AxError> {
        if let Some(admitted) = self.admitted.take() {
            self.spans.push(admitted.elapsed());
        }
        self.bench.account(ticket, answered).and_then(answer)
    }
}

/// The real ledger, each call timed: every call is one barrier.
struct Timed {
    inner: storage::JsonlLedger,
    spans: Vec<Duration>,
}

impl Ledger for Timed {
    fn append(&mut self, draft: kernel::EventDraft) -> Result<kernel::EventRef, AxError> {
        let started = Instant::now();
        let echo = Ledger::append(&mut self.inner, draft);
        self.spans.push(started.elapsed());
        echo
    }

    fn append_all(
        &mut self,
        drafts: Vec<kernel::EventDraft>,
    ) -> Result<Vec<kernel::EventRef>, AxError> {
        let started = Instant::now();
        let echoes = Ledger::append_all(&mut self.inner, drafts);
        self.spans.push(started.elapsed());
        echoes
    }
}

/// What one run of reads left: the gap of every turn, from a reply to
/// the next request, the span of every read, and every barrier.
struct Spans {
    gaps: Vec<Duration>,
    reads: Vec<Duration>,
    barriers: Vec<Duration>,
}

/// A model that asks for one read per turn, `turns` times, and writes
/// down when each request reached it and when each reply left it.
struct OneReadPerTurn {
    turns: usize,
    asked: Vec<Instant>,
    replied: Vec<Instant>,
}

impl Model for OneReadPerTurn {
    fn call(&mut self, _req: &ModelRequest) -> Result<ModelReturn, AxError> {
        self.asked.push(Instant::now());
        let turn = self.asked.len();
        let calls = if turn <= self.turns {
            let mut args = serde_json::Map::new();
            args.insert(
                "file_path".to_owned(),
                serde_json::Value::String("lab/file.rs".to_owned()),
            );
            vec![ToolCall {
                id: format!("c{turn}"),
                name: kernel::ToolName::parse("read").unwrap(),
                args: Payload::new(args).unwrap(),
            }]
        } else {
            Vec::new()
        };
        let reply = ModelReturn::bare(
            kernel::model::message_payload(&[ContentBlock::Text {
                text: "next".to_owned(),
            }])?,
            calls,
        );
        self.replied.push(Instant::now());
        Ok(reply)
    }
}

/// One run of `turns` reads of a `size`-byte file; the gap of every
/// turn, from a reply to the next request.
fn spans(size: usize, turns: usize) -> Spans {
    let city = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(city.path().join("lab")).unwrap();
    let line = "    let value = compute(left, right).unwrap_or_default();\n";
    let text = line.repeat(size / line.len() + 1);
    std::fs::write(
        city.path().join("lab").join("file.rs"),
        text.get(..size).unwrap(),
    )
    .unwrap();
    let ledger_dir = tempfile::tempdir().unwrap();
    let (inner, _) = storage::JsonlLedger::open(ledger_dir.path(), TimeMs::new(1)).unwrap();
    let mut ledger = Timed {
        inner,
        spans: Vec::new(),
    };
    let mut face = Reads::new(city.path());
    let mut model = OneReadPerTurn {
        turns,
        asked: Vec::new(),
        replied: Vec::new(),
    };
    let mut now = || Ok(TimeMs::new(1));
    let mut interrupt = |_: SafePoint| Interrupt::None;
    let mut hooks = RunHooks {
        now: &mut now,
        monotonic_us: &mut || 0,
        interrupt: &mut interrupt,
        checkpoint: None,
        writes: &|_: &ToolCall| kernel::Writes::Nothing,
        invoke: &mut face,
        wait: &mut |_: TimeMs| crate::NextCall::Allowed,
        deltas: None,
    };
    let handoff = Handoff::new(
        vec![job_locator()],
        "read round".to_owned(),
        "not recorded".to_owned(),
        "an instrument ran it".to_owned(),
        "not recorded".to_owned(),
    )
    .unwrap();
    crate::run::drive(run_plan(), &mut ledger, &mut model, &mut hooks, &handoff).unwrap();
    Spans {
        gaps: model
            .replied
            .iter()
            .zip(model.asked.iter().skip(1))
            .take(turns)
            .map(|(replied, asked)| asked.duration_since(*replied))
            .collect(),
        reads: face.spans,
        barriers: ledger.spans,
    }
}

fn micros_at(samples: &mut [Duration], per_mille: usize) -> f64 {
    samples.sort_unstable();
    let index = (samples.len() * per_mille).div_ceil(1000).saturating_sub(1);
    samples[index.min(samples.len() - 1)].as_secs_f64() * 1e6
}

#[test]
#[ignore = "an instrument: run it by name with --run-ignored only --no-capture"]
fn instrument_read_round_trip() {
    for size in [20 << 10, 100 << 10, 1 << 20] {
        // The first turn pays the run's first assembly and the ledger's
        // first segment; it is left out of every reading.
        let (mut gaps, mut reads, mut barriers) = (Vec::new(), Vec::new(), Vec::new());
        for _ in 0..5 {
            let mut run = spans(size, 41);
            gaps.extend(run.gaps.split_off(1));
            reads.extend(run.reads.split_off(1));
            barriers.extend(run.barriers.split_off(1));
        }
        println!(
            "read_round_trip size={size} turns={} gap_p50_us={:.1} gap_p95_us={:.1} \
             gap_p99_us={:.1} read_p50_us={:.1} read_p95_us={:.1} barrier_p50_us={:.1} \
             barrier_p95_us={:.1}",
            gaps.len(),
            micros_at(&mut gaps, 500),
            micros_at(&mut gaps, 950),
            micros_at(&mut gaps, 990),
            micros_at(&mut reads, 500),
            micros_at(&mut reads, 950),
            micros_at(&mut barriers, 500),
            micros_at(&mut barriers, 950),
        );
    }
}
