// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::redundant_closure,
    clippy::disallowed_methods,
    reason = "test code"
)]

use kernel::{Address, AxError, Payload, RunId, TimeMs, ToolCall, ToolName, ToolOutcome};
use runtime::run::{RunHooks, SafePoint, drive};
use runtime::turn::Interrupt;

use super::support::*;

pub(super) const READS: u32 = 3;

/// Long enough that a thread the scheduler delays under a loaded build
/// still arrives; a serial wave waits it out on every read and fails.
pub(super) const ARRIVAL_WAIT: std::time::Duration = std::time::Duration::from_secs(5);

/// Where the reads of one wave meet: each read waits until every read of
/// the wave has started, then notes how many had started while it was
/// still running. A read that saw them all overlapped every other read.
pub(super) struct Meeting {
    pub(super) started: std::sync::Mutex<u32>,
    pub(super) arrived: std::sync::Condvar,
    pub(super) fewest_seen: std::sync::Mutex<u32>,
}

/// A read-only tool; with a meeting, each call waits at it.
pub(super) struct MeetingRead {
    pub(super) meta: kernel::ToolMeta,
    pub(super) meeting: Option<std::sync::Arc<Meeting>>,
}

impl kernel::Tool for MeetingRead {
    fn meta(&self) -> &kernel::ToolMeta {
        &self.meta
    }

    fn invoke(&self, _call: &ToolCall) -> Result<ToolOutcome, AxError> {
        if let Some(meeting) = &self.meeting {
            let mut started = meeting.started.lock().unwrap();
            *started = started.saturating_add(1);
            meeting.arrived.notify_all();
            let (started, _) = meeting
                .arrived
                .wait_timeout_while(started, ARRIVAL_WAIT, |seen| *seen < READS)
                .unwrap();
            let mut fewest = meeting.fewest_seen.lock().unwrap();
            *fewest = (*fewest).min(*started);
        }
        Ok(ToolOutcome {
            result: Payload::empty(),
            attachments: Vec::new(),
        })
    }
}

/// The bench's three stages, with each call's key placed by its
/// position in the run, as the lane places it. It stays because runtime
/// cannot depend on sprawling, where the served city's `Placing` lives;
/// that one is judged in sprawling's `driving::tests::placing`.
pub(super) struct Placed {
    pub(super) bench: runtime::bench::ToolBench,
    pub(super) next: u64,
}

impl Placed {
    pub(super) fn with(meeting: Option<std::sync::Arc<Meeting>>) -> Placed {
        Placed::of(Box::new(MeetingRead {
            meta: read_meta(),
            meeting,
        }))
    }

    pub(super) fn of(read: Box<dyn kernel::Tool>) -> Placed {
        let domain = kernel::WriteDomain::new(vec![Address::parse("lab").unwrap()]).unwrap();
        let mut bench = runtime::bench::ToolBench::new(domain);
        bench.register(read).unwrap();
        Placed { bench, next: 0 }
    }

    pub(super) fn key(&mut self, call: &ToolCall) -> kernel::IdemKey {
        let at = self.next;
        self.next = at.saturating_add(1);
        kernel::IdemKey::derive(
            &RunId::from_bytes([7; 16]),
            kernel::Seq::new(at),
            call.id.as_bytes(),
        )
    }
}

/// The serial reference: each call answered where it is admitted, so no
/// read starts early, with each tool's registration named as the lane's
/// own face names it - the ledgers then differ only if the order or the
/// payloads do.
pub(super) struct OneByOne(pub(super) Placed);

impl runtime::ConcurrentInvoke for OneByOne {
    fn meta_of(&self, call: &ToolCall) -> Option<&kernel::ToolMeta> {
        self.0.bench.meta_of(&call.name)
    }

    fn admit(&mut self, call: &ToolCall, t: TimeMs) -> runtime::Admitted {
        let key = self.0.key(call);
        runtime::Admitted::Answered(self.0.bench.invoke(call, &key, t).and_then(answer))
    }

    fn tool(&self, ticket: &runtime::bench::Ticket) -> Result<&dyn kernel::Tool, AxError> {
        self.0.bench.tool_for(ticket)
    }

    fn account(
        &mut self,
        _call: &ToolCall,
        ticket: runtime::bench::Ticket,
        answered: Result<ToolOutcome, AxError>,
    ) -> Result<ToolOutcome, AxError> {
        self.0.bench.account(ticket, answered).and_then(answer)
    }
}

pub(super) fn read_meta() -> kernel::ToolMeta {
    kernel::ToolMeta {
        name: ToolName::parse("read").unwrap(),
        disclosure: "reads a file".to_owned(),
        params: Payload::empty(),
        effect: kernel::Effect::Read,
        cost_tier: kernel::CostTier::Free,
        timeout: None,
        render: kernel::RenderIntent::Generic,
        temporal: kernel::Temporal::Timeless,
    }
}

pub(super) fn answer(outcome: runtime::bench::BenchOutcome) -> Result<ToolOutcome, AxError> {
    match outcome {
        runtime::bench::BenchOutcome::Ran { outcome, .. }
        | runtime::bench::BenchOutcome::Duplicate { outcome } => Ok(outcome),
        runtime::bench::BenchOutcome::Refused { refusal } => Err(*refusal),
    }
}

impl runtime::ConcurrentInvoke for Placed {
    fn meta_of(&self, call: &ToolCall) -> Option<&kernel::ToolMeta> {
        self.bench.meta_of(&call.name)
    }

    fn admit(&mut self, call: &ToolCall, t: TimeMs) -> runtime::Admitted {
        let key = self.key(call);
        match self.bench.clear(call, &key, t) {
            Ok(runtime::bench::Clearance::Cleared(ticket)) => runtime::Admitted::Cleared(ticket),
            Ok(runtime::bench::Clearance::Answered(outcome)) => {
                runtime::Admitted::Answered(answer(outcome))
            }
            Err(refused) => runtime::Admitted::Answered(Err(refused)),
        }
    }

    fn ahead(&self, call: &ToolCall) -> Option<&dyn kernel::Tool> {
        self.bench.tool_named(&call.name)
    }

    fn tool(&self, ticket: &runtime::bench::Ticket) -> Result<&dyn kernel::Tool, AxError> {
        self.bench.tool_for(ticket)
    }

    fn account(
        &mut self,
        _call: &ToolCall,
        ticket: runtime::bench::Ticket,
        answered: Result<ToolOutcome, AxError>,
    ) -> Result<ToolOutcome, AxError> {
        self.bench.account(ticket, answered).and_then(answer)
    }
}

pub(super) fn three_reads_driven(invoke: &mut dyn runtime::ConcurrentInvoke) -> RecordingLedger {
    let read = |id: &str| ToolCall {
        id: id.to_owned(),
        name: ToolName::parse("read").unwrap(),
        args: Payload::empty(),
    };
    let mut ledger = RecordingLedger::new();
    let mut model = ScriptedModel {
        seen: std::rc::Rc::new(std::cell::RefCell::new(Vec::new())),
        waves: vec![vec![read("r1"), read("r2"), read("r3")]],
    };
    // A stopped clock: reads that overlap take their readings in another
    // order than reads made one after another, and what the two ledgers
    // must share is the order and the payloads of their lines.
    let mut now = || -> Result<TimeMs, AxError> { Ok(TimeMs::new(0)) };
    let mut interrupt = |_: SafePoint| Interrupt::None;
    let mut hooks = RunHooks {
        now: &mut now,
        monotonic_us: &mut || 0,
        interrupt: &mut interrupt,
        checkpoint: None,
        writes: &|_: &kernel::ToolCall| kernel::Writes::Domain,
        invoke,
        wait: &mut |_: TimeMs| runtime::NextCall::Allowed,
        deltas: None,
    };
    drive(plan(), &mut ledger, &mut model, &mut hooks, &handoff()).unwrap();
    ledger
}
