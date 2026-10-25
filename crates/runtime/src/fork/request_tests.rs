// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The first request a branch sends opens with the bytes of the last
//! request its mother sent (`crates/runtime/spec/Fork.lean` §8-2): a provider reuses a
//! cached prompt only for a prefix that matches byte for byte, so a
//! branch that re-worded any message its mother sent pays for the whole
//! conversation again.
//!
//! Both runs are driven for real, through `drive`, onto a ledger on
//! disk; the branch is rebuilt from that ledger by
//! `inherited_indexed`, the door the city takes. What is compared is
//! each message as the model seam received it, serialised.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use kernel::{
    Address, AxError, Ceiling, ChatMessage, ContentBlock, Locator, Model, ModelRequest,
    ModelReturn, Payload, Retries, RunId, Seq, TimeMs, ToolCall, ToolName, ToolOutcome,
};

use crate::conversation::Opening;
use crate::prefix::{FrozenPrefix, FrozenSegment, SegmentSlot};
use crate::turn::{CallShape, Interrupt};
use crate::{Handoff, RunHooks, RunPlan, SafePoint, drive};

/// A model that answers from a script and keeps, for every request,
/// each message it was sent, serialised.
struct Recorded {
    replies: Vec<ModelReturn>,
    sent: Vec<Vec<String>>,
}

impl Model for Recorded {
    fn call(&mut self, req: &ModelRequest) -> Result<ModelReturn, AxError> {
        self.sent.push(
            req.chat
                .messages
                .iter()
                .map(|message| serde_json::to_string(message).unwrap())
                .collect(),
        );
        Ok(self.replies.remove(0))
    }
}

fn says(text: &str, calls: Vec<ToolCall>) -> ModelReturn {
    let mut blocks = vec![ContentBlock::Text {
        text: text.to_owned(),
    }];
    blocks.extend(calls.iter().map(|call| ContentBlock::ToolUse {
        id: call.id.clone(),
        name: call.name.clone(),
        input: call.args.clone(),
    }));
    ModelReturn::bare(kernel::model::message_payload(&blocks).unwrap(), calls)
}

#[test]
fn a_branch_first_request_opens_with_the_bytes_of_the_mothers_last() {
    let dir = tempfile::tempdir().unwrap();
    let (mut ledger, _) = storage::JsonlLedger::open(dir.path(), TimeMs::new(0)).unwrap();
    let status = ToolCall {
        id: "tu_1".to_owned(),
        name: ToolName::parse("status").unwrap(),
        args: Payload::empty(),
    };
    let mut mother = Recorded {
        replies: vec![
            says("reading it now", vec![status]),
            says("it reads 42", Vec::new()),
        ],
        sent: Vec::new(),
    };
    run(
        &mut ledger,
        &mut mother,
        plan(RunId::from_bytes([1; 16]), "measure the meter", Vec::new()),
    );

    let mut index = storage::LedgerIndex::empty();
    index.refresh(dir.path()).unwrap();
    let tail = Seq::new(u64::try_from(index.len()).unwrap() - 1);
    let inherited = super::inherited_indexed(&index, dir.path(), tail).unwrap();
    let mut daughter = Recorded {
        replies: vec![says("in feet it is 138", Vec::new())],
        sent: Vec::new(),
    };
    run(
        &mut ledger,
        &mut daughter,
        plan(
            RunId::from_bytes([2; 16]),
            "and in feet?",
            inherited.messages,
        ),
    );

    let last = mother.sent.last().unwrap();
    let first = &daughter.sent[0];
    assert_eq!(first.get(..last.len()), Some(&last[..]));
}

/// A read the renderer draws as a header and a marked body, and a refusal
/// it draws as one line, reach a branch as the bytes the mother was
/// shown: the rebuild draws each line with the renderer it records.
#[test]
fn a_branch_is_shown_the_mothers_reads_and_refusals_as_she_was() {
    let dir = tempfile::tempdir().unwrap();
    let (mut ledger, _) = storage::JsonlLedger::open(dir.path(), TimeMs::new(0)).unwrap();
    let call = |id: &str, name: &str| ToolCall {
        id: id.to_owned(),
        name: ToolName::parse(name).unwrap(),
        args: Payload::of(&serde_json::json!({ "path": "lab/a.md" })).unwrap(),
    };
    let mut mother = Recorded {
        replies: vec![
            says(
                "reading it",
                vec![call("tu_1", "read"), call("tu_2", "search")],
            ),
            says("done", Vec::new()),
        ],
        sent: Vec::new(),
    };
    let text: String = (0..14).map(|n| format!("row {n}\n")).collect();
    let answer = move |call: &ToolCall| match call.name.as_str() {
        "read" => Ok(ToolOutcome {
            result: Payload::of(&serde_json::json!({
                "path": "lab/a.md", "version": "cas:b3-00", "bytes": text.len(), "text": text,
            }))
            .unwrap(),
            attachments: Vec::new(),
        }),
        _ => Err(kernel::AxError::failure(
            kernel::AxCode::InvalidArgs,
            "search",
            "missing string argument `text`",
        )
        .with_recovery("pass the substring to look for")),
    };
    run_answering(
        &mut ledger,
        &mut mother,
        plan(RunId::from_bytes([1; 16]), "read the notes", Vec::new()),
        &answer,
    );

    let mut index = storage::LedgerIndex::empty();
    index.refresh(dir.path()).unwrap();
    let tail = Seq::new(u64::try_from(index.len()).unwrap() - 1);
    let inherited = super::inherited_indexed(&index, dir.path(), tail).unwrap();
    let mut daughter = Recorded {
        replies: vec![says("and so", Vec::new())],
        sent: Vec::new(),
    };
    run(
        &mut ledger,
        &mut daughter,
        plan(RunId::from_bytes([2; 16]), "go on", inherited.messages),
    );

    let last = mother.sent.last().unwrap();
    let first = &daughter.sent[0];
    assert_eq!(first.get(..last.len()), Some(&last[..]));
    let shown = last.join("\n");
    // The version an earlier read recorded is not drawn: edit takes none.
    assert!(shown.contains("lab/a.md lines 1-14 of 14") && !shown.contains("cas:b3-00"));
    assert!(shown.contains("E_INVALID_ARGS search: missing string argument"));
}

/// Drives one run onto `ledger` with nothing interrupting it and every
/// call answered with the same reading.
fn run(ledger: &mut storage::JsonlLedger, model: &mut Recorded, plan: RunPlan) {
    run_answering(ledger, model, plan, &|_: &ToolCall| {
        Ok(ToolOutcome {
            result: Payload::of(&serde_json::json!({ "said": "42" })).unwrap(),
            attachments: Vec::new(),
        })
    });
}

/// Drives one run onto `ledger`, every call answered by `answer`.
fn run_answering(
    ledger: &mut storage::JsonlLedger,
    model: &mut Recorded,
    plan: RunPlan,
    answer: &dyn Fn(&ToolCall) -> Result<ToolOutcome, AxError>,
) {
    let mut clock = 0u64;
    let mut now = || {
        clock = clock.saturating_add(1);
        Ok(TimeMs::new(clock))
    };
    let mut interrupt = |_: SafePoint| Interrupt::None;
    let mut invoke = |call: &ToolCall, _: TimeMs| answer(call);
    let mut hooks = RunHooks {
        now: &mut now,
        monotonic_us: &mut || 0,
        interrupt: &mut interrupt,
        checkpoint: None,
        writes: &|_: &ToolCall| kernel::Writes::Nothing,
        invoke: &mut invoke,
        wait: &mut |_: TimeMs| crate::NextCall::Allowed,
        deltas: None,
    };
    let handoff = Handoff::new(
        vec![job(&room())],
        "overview".to_owned(),
        "progress".to_owned(),
        "context".to_owned(),
        "next".to_owned(),
    )
    .unwrap();
    drive(plan, ledger, model, &mut hooks, &handoff).unwrap();
}

fn room() -> Address {
    Address::parse("lab/room1").unwrap()
}

fn job(addr: &Address) -> Locator {
    Locator::parse(&format!("file:{}/JOB.md@{}", addr.as_str(), "a".repeat(40))).unwrap()
}

/// One run the person dispatched in `lab/room1`, opening with what they
/// typed, after `inherited` when it is a branch.
fn plan(run: RunId, task: &str, inherited: Vec<ChatMessage>) -> RunPlan {
    RunPlan {
        run,
        who: "lab/room1".to_owned(),
        addr: room(),
        task: task.to_owned(),
        goal: "a number is written down".to_owned(),
        opening: Opening::WithPerson,
        job: job(&room()),
        parent: None,
        predecessor: None,
        dispatched_by: kernel::event::Who::Person,
        inherited,
        shape: CallShape {
            model: "script".to_owned(),
            max_tokens: Ceiling::new(4096),
            effort: None,
            context_tokens: 0,
        },
        second_threshold: None,
        context: crate::ContextReading::default(),
        prefix: FrozenPrefix::assemble(
            FrozenSegment::new(SegmentSlot::City, b"city".to_vec()),
            FrozenSegment::new(SegmentSlot::Building, b"building".to_vec()),
            FrozenSegment::new(SegmentSlot::Resident, b"resident".to_vec()),
            FrozenSegment::new(SegmentSlot::Run, b"run".to_vec()),
        )
        .unwrap(),
        policy: kernel::BuildingPolicy::default(),
        tools: Vec::new(),
        skills: Vec::new(),
        retries: Retries::UntilHalted,
        run_policy: crate::PolicyCell::new(kernel::RunPolicy::of(kernel::Mode::Work)),
        naming: None,
    }
}
