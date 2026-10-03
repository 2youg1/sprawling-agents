// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The checks derived from the held-run half of
//! `crates/runtime/spec/Turn/Durability.lean` (runtime D36), driven
//! through a whole run: the lines a turn holds cross into the next
//! turn, so each turn pays `1 + writes` barriers and the run one more at
//! its end, and a power cut at any line still leaves a prefix of the
//! uncut history with no write ahead of its intent.

#![allow(clippy::arithmetic_side_effects, reason = "test code")]

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::super::*;
use super::durability::{Durable, Face};
use super::helpers::*;
use crate::conversation::Opening;
use crate::handoff::Handoff;
use crate::run::{RunHooks, RunPlan, SafePoint};
use kernel::{EventDraft, EventRef, ModelRequest};
use proptest::prelude::*;

/// A ledger whose every call is one disk barrier, counted where the
/// model can read the count too. The power goes out when it is about to
/// write line `cut_at`: that line and every later one never reach the
/// disk.
struct Tape {
    inner: TestLedger,
    durable: Durable,
    barriers: Arc<AtomicUsize>,
    cut_at: usize,
}

impl Tape {
    fn cut_at(line: usize) -> Tape {
        Tape {
            inner: TestLedger::new(),
            durable: Arc::default(),
            barriers: Arc::default(),
            cut_at: line,
        }
    }

    fn keep(&mut self, draft: EventDraft) -> Result<EventRef, AxError> {
        if self.durable.lock().unwrap().len() >= self.cut_at {
            return Err(AxError::failure(
                AxCode::Busy,
                "append to the ledger",
                "the power went out",
            )
            .with_recovery("open the ledger again; it holds what reached the disk"));
        }
        let kind = serde_json::to_value(draft.kind)
            .unwrap()
            .as_str()
            .unwrap()
            .to_owned();
        let echo = self.inner.append(draft)?;
        self.durable.lock().unwrap().push(kind);
        Ok(echo)
    }
}

impl Ledger for Tape {
    fn append(&mut self, draft: EventDraft) -> Result<EventRef, AxError> {
        self.barriers.fetch_add(1, Ordering::SeqCst);
        self.keep(draft)
    }

    fn append_all(&mut self, drafts: Vec<EventDraft>) -> Result<Vec<EventRef>, AxError> {
        self.barriers.fetch_add(1, Ordering::SeqCst);
        drafts.into_iter().map(|draft| self.keep(draft)).collect()
    }
}

/// What the model saw when it was asked: the barriers paid so far and
/// the last durable line.
#[derive(Debug, Clone, PartialEq)]
struct Asked {
    barriers: usize,
    last: Option<String>,
}

/// A model that asks for one wave per turn, in order, and then answers
/// with text and no call, which ends the run as done.
struct Waves {
    waves: std::vec::IntoIter<Vec<ToolCall>>,
    durable: Durable,
    barriers: Arc<AtomicUsize>,
    asked: Vec<Asked>,
}

impl Model for Waves {
    fn call(&mut self, _req: &ModelRequest) -> Result<ModelReturn, AxError> {
        self.asked.push(Asked {
            barriers: self.barriers.load(Ordering::SeqCst),
            last: self.durable.lock().unwrap().last().cloned(),
        });
        Ok(ModelReturn::bare(
            kernel::model::message_payload(&[ContentBlock::Text {
                text: "next".to_owned(),
            }])?,
            self.waves.next().unwrap_or_default(),
        ))
    }
}

fn plan() -> RunPlan {
    let addr = kernel::Address::parse("lab").unwrap();
    RunPlan {
        run: run_id(),
        who: "lab".to_owned(),
        addr,
        task: "read and write".to_owned(),
        goal: "turns that hold their lines".to_owned(),
        opening: Opening::FromJob,
        job: job(),
        parent: None,
        predecessor: None,
        dispatched_by: kernel::event::Who::Person,
        run_policy: crate::PolicyCell::new(kernel::RunPolicy::of(kernel::Mode::Work)),
        naming: None,
        inherited: Vec::new(),
        shape: shape(),
        second_threshold: None,
        context: crate::ContextReading::default(),
        prefix: prefix(),
        policy: BuildingPolicy::default(),
        tools: Vec::new(),
        skills: Vec::new(),
        retries: kernel::Retries::UntilHalted,
    }
}

fn job() -> kernel::Locator {
    kernel::Locator::parse(&format!("file:lab/JOB.md@{}", "a".repeat(40))).unwrap()
}

/// The wave a model asks for: `true` is a write, `false` a read.
fn wave_of(turn: usize, writes: &[bool]) -> Vec<ToolCall> {
    writes
        .iter()
        .enumerate()
        .map(|(i, write)| ToolCall {
            id: format!("c{turn}-{i}"),
            name: kernel::ToolName::parse(if *write { "write" } else { "read" }).unwrap(),
            args: Payload::empty(),
        })
        .collect()
}

/// What one run leaves: whether it froze, what reached the disk, what
/// the model and each tool saw when they ran.
struct Ran {
    froze: bool,
    durable: Vec<String>,
    barriers: usize,
    asked: Vec<Asked>,
    tools_saw: Vec<Vec<String>>,
}

fn run_of(waves: &[Vec<bool>], cut_at: usize) -> Ran {
    let mut ledger = Tape::cut_at(cut_at);
    let saw = Arc::default();
    let mut face = Face::new(&ledger.durable, &saw);
    let mut model = Waves {
        waves: waves
            .iter()
            .enumerate()
            .map(|(turn, writes)| wave_of(turn, writes))
            .collect::<Vec<_>>()
            .into_iter(),
        durable: Arc::clone(&ledger.durable),
        barriers: Arc::clone(&ledger.barriers),
        asked: Vec::new(),
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
        vec![job()],
        "held lines".to_owned(),
        "not recorded".to_owned(),
        "a test ran it".to_owned(),
        "not recorded".to_owned(),
    )
    .unwrap();
    let froze = crate::run::drive(plan(), &mut ledger, &mut model, &mut hooks, &handoff).is_ok();
    let durable = ledger.durable.lock().unwrap().clone();
    let tools_saw = saw.lock().unwrap().clone();
    Ran {
        froze,
        durable,
        barriers: ledger.barriers.load(Ordering::SeqCst),
        asked: model.asked,
        tools_saw,
    }
}

/// The lines the reference order appends between the dispatch pair and
/// the freeze, kind by kind (`tf1_records_match_reference`).
fn reference(waves: &[Vec<bool>]) -> Vec<String> {
    let mut lines = vec!["prompt_assembled".to_owned()];
    for writes in waves.iter().map(Vec::as_slice).chain([&[][..]]) {
        lines.extend(["prompt_shape_compared", "model_called", "model_returned"].map(String::from));
        for _ in writes {
            lines.extend(["tool_called", "tool_result"].map(String::from));
        }
    }
    lines
}

fn waves() -> impl Strategy<Value = Vec<Vec<bool>>> {
    proptest::collection::vec(proptest::collection::vec(any::<bool>(), 1..8), 0..6)
}

proptest! {
    /// `tf1_turn_barriers` and `tf1_run_refs_complete` through a run:
    /// from one model call to the next the run pays one barrier for each
    /// write and one for the next model call, and from the last model
    /// call to the end it pays one for its held lines and the freeze's
    /// own; every line the reference appends is on disk at the end, in
    /// the reference order.
    #[test]
    fn tf1_held_lines_cost_one_barrier_plus_one_per_write(waves in waves()) {
        let ran = run_of(&waves, usize::MAX);
        prop_assert!(ran.froze);
        let gaps: Vec<usize> = ran
            .asked
            .windows(2)
            .map(|pair| pair[1].barriers - pair[0].barriers)
            .collect();
        let expected: Vec<usize> = waves
            .iter()
            .map(|writes| 1 + writes.iter().filter(|write| **write).count())
            .collect();
        prop_assert_eq!(gaps, expected);
        let last = ran.asked.last().unwrap().barriers;
        let frozen = run_of(&[], usize::MAX);
        prop_assert_eq!(
            ran.barriers - last,
            frozen.barriers - frozen.asked.last().unwrap().barriers
        );
        let dispatch = 2;
        let tail = ran.durable.len() - dispatch - reference(&waves).len();
        prop_assert_eq!(
            &ran.durable[dispatch..ran.durable.len() - tail],
            &reference(&waves)[..]
        );
    }

    /// `tf1_effect_after_durability` and `tf1_write_intent_durable`
    /// through a run, and `held_cut_is_closed_cut`'s prefix half under a
    /// power cut at any line: every model call and every write sees its
    /// own intent as the last durable line, and what reached the disk is
    /// a prefix of the uncut run's lines.
    #[test]
    fn tf1_a_power_cut_in_a_held_run_leaves_a_prefix(waves in waves(), cut in 0usize..48) {
        let whole = run_of(&waves, usize::MAX).durable;
        let ran = run_of(&waves, cut);
        prop_assert!(whole.starts_with(&ran.durable));
        prop_assert_eq!(ran.froze, ran.durable.len() == whole.len());
        for asked in &ran.asked {
            prop_assert_eq!(asked.last.as_deref(), Some("model_called"));
        }
        let calls: Vec<bool> = waves.iter().flatten().copied().collect();
        for (i, seen) in ran.tools_saw.iter().enumerate() {
            if calls[i] {
                prop_assert_eq!(seen.last().map(String::as_str), Some("tool_called"));
            }
        }
    }
}
