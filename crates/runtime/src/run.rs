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
    Payload, RunId, TimeMs, ToolCall, ToolDef, ToolOutcome,
};
use serde_json::{Map, Value};

use crate::catalog::SkillPin;
use crate::handoff::Handoff;
use crate::prefix::FrozenPrefix;
use crate::reminder::ContextGauge;
use crate::turn::{CallShape, Interrupt};
use crate::window::{Opening, Window};

mod lifecycle;

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
    pub shape: CallShape,
    pub prefix: FrozenPrefix,
    pub policy: BuildingPolicy,
    pub tools: Vec<ToolDef>,
    /// The skills this run's reading room admitted, and what each one
    /// hashed to when the shelf was read. Written into `run_started`
    /// beside `parent`, and for the same reason: once the process that
    /// read the shelf is gone, the ledger is the only thing that can
    /// still say which bytes this run was given.
    pub skills: Vec<SkillPin>,
}

/// Where the driver stops to ask whether anything arrived. The turn layer
/// owns three cancellation-safe points; this enum is how the driver names
/// them to a caller that knows nothing about turn internals.
#[non_exhaustive]
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

/// The four things the driver cannot decide for itself. Four closures
/// rather than four traits: no second implementation exists yet, and this
/// library introduces a trait at a seam that already has one.
pub struct RunHooks<'a> {
    /// The clock. Called once per dispatch line, once per turn, and once
    /// per freeze that is not a cancellation.
    pub now: &'a mut dyn FnMut() -> Result<TimeMs, AxError>,
    /// Answers what arrived at a safe point.
    pub interrupt: &'a mut dyn FnMut(SafePoint) -> Interrupt,
    /// The pre-wave checkpoint fence. `None` runs without a net, which
    /// the tool layer refuses for anything that can delete.
    pub fence: Option<&'a mut dyn FnMut(TimeMs) -> Result<Payload, AxError>>,
    /// Runs one tool call. The turn's stamp rides along because the tool
    /// layer stamps results and derives idempotency keys from it, and a
    /// caller that sampled its own clock there would be a second time
    /// source inside one turn.
    pub invoke: &'a mut dyn FnMut(&ToolCall, TimeMs) -> Result<ToolOutcome, AxError>,
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
    window: Window,
    turns: u32,
    last_turn_t: Option<TimeMs>,
    /// How full the window is, read off each call's reported usage.
    gauge: ContextGauge,
}

/// A frozen run. There is no method back to [`Active`]: waking an old run
/// is not something this type can spell, and `resume` takes a [`Handoff`]
/// rather than a run.
pub struct Frozen {
    completion: Completion,
    turns: u32,
    /// Kept past the freeze for one reader: the transcript written
    /// beside the room, which is what this run's model actually saw.
    window: Window,
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
        crate::transcript::Transcript::of(self.plan.run, &self.state.window)
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
    let mut run = Run::dispatch(plan, ledger, hooks)?;
    let ending = loop {
        match run.advance(ledger, model, hooks) {
            Ok(Advance::Turned) => {}
            Ok(Advance::Concluded(completion)) => break completion,
            // A run always ends. A mid-turn failure whose code has a
            // carrier event (provider down, budget, watchdog) is written
            // into history under that carrier and the run freezes as
            // cancelled - before this arm existed, a 401 from a provider
            // left a run permanently "started": no event, no freeze, an
            // event stream that simply went quiet.
            //
            // A loadtime code names no carrier, and for a while that
            // meant it left by a second door. It does not: the verdict
            // is written first and the diagnosis travels afterwards, so
            // the two facts do not compete. The old reason - "when the
            // ledger itself is the casualty there is nothing truthful
            // left to write" - holds for a corrupt store and not for
            // `E_WIRE_MISMATCH`, where a provider spelled its dialect
            // wrong and the ledger is in perfect health. Where the store
            // really is the casualty, `freeze` fails on its own append
            // and that failure is what travels, which is more honest
            // than deciding in advance that nothing can be written.
            Err(err) => {
                let Carrier::Event(kind) = err.code().carrier() else {
                    run.freeze(ledger, handoff, Completion::Cancelled, hooks)?;
                    return Err(err);
                };
                let t = (hooks.now)()?;
                let mut data = Map::new();
                if let Ok(Value::Object(fields)) = serde_json::to_value(&err) {
                    data = fields;
                }
                ledger.append(EventDraft {
                    run: run.plan.run,
                    t,
                    who: run.plan.who.clone(),
                    addr: Some(run.plan.addr.clone()),
                    kind,
                    data: payload(data)?,
                    ig: false,
                })?;
                break Completion::Cancelled;
            }
        }
    };
    run.freeze(ledger, handoff, ending, hooks)
}

fn payload(map: Map<String, Value>) -> Result<Payload, AxError> {
    Payload::new(map)
}
