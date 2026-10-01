// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The run driver: dispatch, turns, freeze — the single authority for the
//! event sequence a run leaves behind. The real city and citysim call this
//! same code, so the simulator's byte fixtures are evidence about
//! production rather than about a second implementation of the same idea.
//!
//! Time arrives through the `now` hook and is never sampled here. The
//! caller decides what a clock is: a counter in the simulator, the one
//! sanctioned wall-clock sample in `bin::assembly`.

use kernel::{
    Address, AxError, BuildingPolicy, Carrier, Completion, EventDraft, Ledger, Locator, Model,
    Payload, RunId, TimeMs, ToolCall, ToolDef,
};

use kernel::ChatMessage;

use crate::catalog::SkillPin;
use crate::conversation::{Conversation, Opening};
use crate::handoff::Handoff;
use crate::prefix::FrozenPrefix;
use crate::reminder::ContextGauge;
use crate::turn::{CallShape, Interrupt};

mod charter;
mod checkpoint;
mod harness;
mod lifecycle;

pub use charter::Charter;
pub use harness::{Conclusion, Cut, HarnessRun};

/// Everything constant about one run. Assembled by the caller, because
/// what a prefix contains and which tools exist are decisions of the city
/// that dispatches, not of the loop that runs.
pub struct RunPlan {
    pub run: RunId,
    pub who: String,
    pub addr: Address,
    pub task: String,
    pub goal: String,
    /// Whether this session was handed a written task or a person.
    /// Decided by the city when it laid the brief down, carried here so
    /// the window and the prefix's run segment cannot disagree about
    /// which situation the agent is in.
    pub opening: Opening,
    pub job: Locator,
    /// The run that handed this work down, when one did. Written into
    /// `run_started` so the tree is a fact anybody folding the ledger
    /// can read, rather than an inference from two lines happening to
    /// be next to each other.
    pub parent: Option<RunId>,
    /// The run this one replaces, when a resident succeeded itself.
    /// Written into `run_started` beside `parent`, so a lineage is a
    /// fact folded from the ledger rather than inferred from two runs
    /// sharing an address.
    pub predecessor: Option<RunId>,
    /// Who dispatched this run, written into `run_started`: the line's
    /// author is always the city's desk, so only the dispatch site
    /// knows whether the person, the city or a resident sent it.
    pub dispatched_by: kernel::event::Who,
    /// The run policy this run was dispatched under, written into
    /// `run_started` as it was chosen (kernel-SPEC 8-77). Named apart
    /// from `policy`, which is the building's, because the two answer
    /// different questions: what the building allows, and what this
    /// dispatch asked for inside that.
    pub run_policy: kernel::RunPolicy,
    /// The conversation this run starts from, when it is the first run of
    /// a session that branched off another. Empty for every other run.
    ///
    /// It is the window's material and not the prefix's: a prefix segment
    /// is a document a person can read and edit, while this is what the
    /// model said and what the tools answered, and the ledger is where
    /// those live (`runtime::fork::inherited_indexed` rebuilds them from it).
    pub inherited: Vec<ChatMessage>,
    pub shape: CallShape,
    /// Where the context reminder's second rung sits for this run: the
    /// configuration ladder's answer, or `None` when no layer stated one
    /// and the policy default sounds. Frozen with the run like every
    /// setting here, so "each rung sounds once per run" cannot depend on
    /// when somebody edited a file.
    pub second_threshold: Option<kernel::SecondThreshold>,
    /// Where this run records the provider's count after each call. The
    /// caller keeps a clone for whatever reports the reading, which is
    /// `status`: the run writes it and nothing else does.
    pub context: crate::ContextReading,
    pub prefix: FrozenPrefix,
    pub policy: BuildingPolicy,
    pub tools: Vec<ToolDef>,
    /// The skills this run's reading room admitted, and what each one
    /// hashed to when the shelf was read. Written into `run_started`
    /// beside `parent`, and for the same reason: once the process that
    /// read the shelf is gone, the ledger is the only thing that can
    /// still say which bytes this run was given.
    pub skills: Vec<SkillPin>,
    /// How many times this run may make a failed call again, as the
    /// person set it on the endpoint it calls. Frozen with the run for
    /// the reason every other setting here is: a number changed halfway
    /// through would make the account of what this run did depend on
    /// when somebody looked at a form.
    pub retries: kernel::Retries,
}

/// Where the driver stops to ask whether anything arrived. The turn layer
/// owns three cancellation-safe points; this enum is how the driver names
/// them to a caller that knows nothing about turn internals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SafePoint {
    BeforeAssemble {
        turn: u32,
    },
    BeforeCall {
        turn: u32,
    },
    BeforeWave {
        turn: u32,
    },
    /// Before one call of the wave, asked once per call. A wave is as
    /// many effects as the model asked for, so a halt that arrives
    /// while the third of eight runs stops the fourth rather than the
    /// next turn.
    BeforeToolCall {
        turn: u32,
        call: u32,
    },
    /// After the wave, before the run acts on what the turn decided.
    /// The one boundary a run that concluded still passes through, so a
    /// cancel arriving that late stops the work the turn handed down
    /// instead of arriving at a run that has already ended.
    BeforeSpawn {
        turn: u32,
    },
}

/// What one turn did. Exhaustive on purpose: a new ending has to make
/// every caller say what it does about it.
#[derive(Debug)]
pub enum Advance {
    Turned,
    Concluded(Completion),
}

/// The things the driver cannot decide for itself. Closures rather
/// than traits: no second implementation exists yet, and this
/// library introduces a trait at a seam that already has one.
pub struct RunHooks<'a> {
    /// The clock, called by the driver on its own thread and nowhere else:
    /// once per dispatch line, once per turn, once when each model attempt
    /// is sent and once when its reply is whole, once when each tool call
    /// starts and once when it answers, and once per freeze that is not a
    /// cancellation.
    pub now: &'a mut dyn FnMut() -> Result<TimeMs, AxError>,
    /// Answers what arrived at a safe point.
    pub interrupt: &'a mut dyn FnMut(SafePoint) -> Interrupt,
    /// The pre-wave checkpoint. `None` runs without a net, which
    /// the tool layer refuses for anything that can delete.
    pub checkpoint: Option<&'a mut dyn FnMut(TimeMs) -> Result<Payload, AxError>>,
    /// What a call may write, by its declared effect, asked before the
    /// wave runs: a wave whose every call answers `Nothing` changes no
    /// file, so it needs no checkpoint of its own (§8-45).
    pub writes: &'a dyn Fn(&ToolCall) -> kernel::Writes,
    /// Runs a wave's tool calls in three stages (see
    /// [`crate::ConcurrentInvoke`]). `admit` receives the turn's stamp; a
    /// call's own moments are read by the turn from `now`, never by the
    /// tool face, which holds no clock of its own.
    pub invoke: &'a mut dyn crate::ConcurrentInvoke,
    /// Holds the run until the moment the watchdog set for the next call
    /// to a provider that failed, and answers whether that call may go.
    ///
    /// `Halted` comes back as soon as a halt reaches the run, without
    /// waiting the rest out: a backoff grows to a minute, and a brake
    /// that engages a minute late is not one. A counted clock answers
    /// `Allowed` at once, because nothing it replays waits in real time.
    pub wait: &'a mut dyn FnMut(TimeMs) -> crate::NextCall,
    /// Where text goes while the model is still saying it.
    ///
    /// `None` runs the model without asking for a stream, which is what
    /// citysim and every offline replay do: an increment changes nothing
    /// a run decides, so a driver with nowhere to put one asks for none
    /// and the call is byte-identical to what it always was.
    ///
    /// It takes no result, because nothing here may fail a call. What
    /// arrives is a thing to look at; the record of what was said is
    /// written from `model_returned`, once, afterwards.
    pub deltas: Option<&'a mut (dyn FnMut(&kernel::Increment) + 'a)>,
}

/// An active run: turns may still be taken.
pub struct Active {
    conversation: Conversation,
    turns: u32,
    last_turn_t: Option<TimeMs>,
    /// How full the window is, read off each call's reported usage.
    gauge: ContextGauge,
    /// The cache shape of the request this run sent last, which is what
    /// the next one is compared against. `None` before the first
    /// request, and that absence is the `FirstRequest` a record states.
    prior_shape: Option<crate::prefix::shape::PromptShape>,
    /// The `prompt_assembled` payload this run wrote last, which a turn
    /// does not write again (runtime-SPEC.md section 8-39, item 5).
    prompt: crate::turn::PromptRecord,
    /// Whether the next wave needs a checkpoint (§8-45).
    checkpoint: checkpoint::CheckpointPolicy,
}

/// A frozen run. There is no method back to [`Active`]: waking an old run
/// is not something this type can spell, and `resume` takes a [`Handoff`]
/// rather than a run.
pub struct Frozen {
    completion: Completion,
    turns: u32,
    /// Kept past the freeze for one reader: the transcript written
    /// beside the room, which is what this run's model actually saw.
    conversation: Conversation,
}

pub struct Run<S> {
    plan: RunPlan,
    state: S,
}

impl Run<Frozen> {
    pub fn completion(&self) -> &Completion {
        &self.state.completion
    }

    /// What this run was dispatched with, for a successor that takes
    /// the same work on.
    pub fn plan(&self) -> &RunPlan {
        &self.plan
    }

    /// What the model saw, scanned and ready to write beside the room.
    ///
    /// # Errors
    /// Propagates a message that will not serialise.
    pub fn transcript(&self) -> Result<crate::transcript::Transcript, AxError> {
        crate::transcript::Transcript::of(self.plan.run, &self.state.conversation)
    }

    pub fn turns(&self) -> u32 {
        self.state.turns
    }

    pub fn run_id(&self) -> RunId {
        self.plan.run
    }
}

/// Dispatch, turn until an ending, freeze. The loop is here rather than in
/// each caller because the ending rules — an empty wave concludes, a
/// cancellation is an ending too — are the part that must not drift
/// between the city and the simulator.
///
/// **The loop counts no turns.** There is no ceiling to reach, because
/// nobody can price a piece of work before it runs. What
/// ends a run is what it did: a turn that concluded, a failure with a
/// carrier event, or an interruption a safe point delivered — and what
/// stops one from outside is `Halt`, which shuts the scope and kills the
/// backlog members inside it.
///
/// # Errors
/// Propagates ledger and provider failures.
pub fn drive(
    plan: RunPlan,
    ledger: &mut dyn Ledger,
    model: &mut dyn Model,
    hooks: &mut RunHooks<'_>,
    handoff: &Handoff,
) -> Result<Run<Frozen>, AxError> {
    let mut watchdog = crate::Watchdog::new(plan.retries, plan.run);
    let mut run = Run::dispatch(plan, ledger, hooks)?;
    let ending = loop {
        match run.advance(ledger, model, hooks) {
            Ok(Advance::Turned) => watchdog.on_provider_answered(),
            Ok(Advance::Concluded(completion)) => break completion,
            // A run always ends. A mid-turn failure whose code has a
            // carrier event (provider down, budget, watchdog) is written
            // into history under that carrier and the run freezes as
            // cancelled, so a 401 from a provider never leaves a run
            // "started" with an event stream that simply went quiet.
            //
            // A loadtime code names no carrier, and it leaves by the
            // same door: the verdict is written first and the diagnosis
            // travels afterwards, so the two facts do not compete.
            // `E_WIRE_MISMATCH` means a provider spelled its dialect
            // wrong while the ledger is in perfect health. Where the
            // store really is the casualty, `freeze` fails on its own
            // append and that failure is what travels, which is more
            // honest than deciding in advance that nothing can be
            // written.
            Err(err) => {
                let Carrier::Event(kind) = err.code().carrier() else {
                    run.freeze(ledger, handoff, Completion::Cancelled, hooks)?;
                    return Err(err);
                };
                let t = (hooks.now)()?;
                // A provider that failed for a reason worth retrying is
                // asked again, and the attempt is recorded rather than
                // repeated silently: the second `model_called` in the
                // history is what a person reads the retry off.
                // The watchdog decides whether there is a next call and
                // when; the line records that moment before the wait,
                // and the wait is where a halt reaches a run that has
                // no turn in flight to stop.
                let disposal = watchdog.on_provider_failure(&err, t);
                if let crate::Disposal::BackOff { until, .. } = disposal {
                    ledger.append(EventDraft {
                        run: run.plan.run,
                        t,
                        who: run.plan.who.clone(),
                        addr: Some(run.plan.addr.clone()),
                        kind: kernel::EventKind::WatchdogFired,
                        data: watchdog.fired_payload(&disposal)?,
                        ig: false,
                    })?;
                    match (hooks.wait)(until) {
                        crate::NextCall::Allowed => continue,
                        crate::NextCall::Halted => break Completion::Cancelled,
                    }
                }
                // The failure itself is the payload, through the one
                // door: an encoding that fails travels as a refusal
                // rather than as an empty object.
                let data = Payload::of(&err)?;
                ledger.append(EventDraft {
                    run: run.plan.run,
                    t,
                    who: run.plan.who.clone(),
                    addr: Some(run.plan.addr.clone()),
                    kind,
                    data,
                    ig: false,
                })?;
                break Completion::Cancelled;
            }
        }
    };
    run.freeze(ledger, handoff, ending, hooks)
}
