// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{
    Address, AxError, BuildingPolicy, Carrier, Completion, Ledger, Locator, Model, Payload, RunId,
    TimeMs, ToolCall, ToolDef,
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

pub struct RunPlan {
    pub run: RunId,
    pub who: String,
    pub addr: Address,
    pub task: String,
    pub goal: String,

    pub opening: Opening,
    pub job: Locator,

    pub parent: Option<RunId>,

    pub predecessor: Option<RunId>,

    pub dispatched_by: kernel::event::Who,

    pub routing: Option<kernel::event::record::ModelRouting>,

    pub run_policy: crate::mode::PolicyCell,

    pub inherited: Vec<ChatMessage>,
    pub shape: CallShape,

    pub second_threshold: Option<kernel::SecondThreshold>,

    pub context_lock: Option<kernel::ContextLock>,

    pub succession: std::sync::Arc<std::sync::Mutex<crate::SuccessionDesk>>,

    pub context: crate::ContextReading,
    pub prefix: FrozenPrefix,
    pub policy: BuildingPolicy,
    pub tools: Vec<ToolDef>,

    pub skills: Vec<SkillPin>,

    pub retries: kernel::Retries,

    pub naming: Option<kernel::B3Hash>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SafePoint {
    BeforeAssemble { turn: u32 },
    BeforeCall { turn: u32 },
    BeforeWave { turn: u32 },

    BeforeToolCall { turn: u32, call: u32 },

    BeforeSpawn { turn: u32 },
}

#[derive(Debug)]
pub enum Advance {
    Turned,
    Concluded(Completion),
}

#[derive(Debug)]
pub struct CheckpointFailure {
    cause: AxError,
    refusal: Option<kernel::event::record::SecretEgressBlocked>,
}

impl CheckpointFailure {
    pub fn refusal(&self) -> Option<&kernel::event::record::SecretEgressBlocked> {
        self.refusal.as_ref()
    }

    pub fn into_cause(self) -> AxError {
        self.cause
    }
}

impl From<storage::StorageError> for CheckpointFailure {
    fn from(err: storage::StorageError) -> Self {
        let refusal = if let storage::StorageError::SecretEgress(report) = &err {
            Some(report.clone())
        } else {
            None
        };
        CheckpointFailure {
            cause: err.into_ax(),
            refusal,
        }
    }
}

impl From<AxError> for CheckpointFailure {
    fn from(cause: AxError) -> Self {
        CheckpointFailure {
            cause,
            refusal: None,
        }
    }
}

pub struct RunHooks<'a> {
    pub now: &'a mut dyn FnMut() -> Result<TimeMs, AxError>,

    pub monotonic_us: &'a mut dyn FnMut() -> u64,

    pub interrupt: &'a mut dyn FnMut(SafePoint) -> Interrupt,

    pub checkpoint: Option<&'a mut dyn FnMut(TimeMs) -> Result<Payload, CheckpointFailure>>,

    pub writes: &'a dyn Fn(&ToolCall) -> kernel::Writes,

    pub invoke: &'a mut dyn crate::ConcurrentInvoke,

    pub wait: &'a mut dyn FnMut(TimeMs) -> crate::NextCall,

    pub deltas: Option<&'a mut (dyn FnMut(&kernel::Increment) + 'a)>,
}

pub struct Active {
    conversation: Conversation,
    turns: u32,
    last_turn_t: Option<TimeMs>,

    failure: Option<kernel::AxError>,

    gauge: ContextGauge,

    prior_shape: Option<crate::prefix::shape::PromptShape>,

    prompt: crate::turn::PromptRecord,

    checkpoint: checkpoint::CheckpointPolicy,

    lines: crate::turn::HeldLines,

    watchdog: crate::Watchdog,
}

pub struct Frozen {
    completion: Completion,
    turns: u32,

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

    pub fn plan(&self) -> &RunPlan {
        &self.plan
    }

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

pub fn drive(
    plan: RunPlan,
    ledger: &mut dyn Ledger,
    model: &mut dyn Model,
    hooks: &mut RunHooks<'_>,
    handoff: &Handoff,
) -> Result<Run<Frozen>, AxError> {
    let mut run = Run::dispatch(plan, ledger, hooks)?;
    let ending = loop {
        if !run.state.watchdog.round_open() {
            match run.state.watchdog.open_round(model.account_roster()) {
                Ok(Some(account)) => model.select_account(&account),
                Ok(None) => {}
                Err(exhausted) => {
                    run.state.lines.barrier(ledger)?;
                    let t = (hooks.now)()?;
                    run.record_refusal(ledger, t, &exhausted)?;
                    run.state.failure = Some(exhausted);
                    break Completion::Cancelled;
                }
            }
        }
        match run.advance(ledger, model, hooks) {
            Ok(Advance::Turned) => run.state.watchdog.on_provider_answered(),
            Ok(Advance::Concluded(completion)) => break completion,

            Err(err) => {
                run.state.lines.barrier(ledger)?;
                let Carrier::Event(kind) = err.code().carrier() else {
                    run.state.failure = Some(err.clone());
                    run.freeze(ledger, handoff, Completion::Cancelled, hooks)?;
                    return Err(err);
                };
                let t = (hooks.now)()?;

                let disposal = run.state.watchdog.on_provider_failure(&err, t);
                match &disposal {
                    crate::Disposal::BackOff { until, .. } => {
                        let until = *until;
                        run.record_fired(ledger, t, &disposal)?;
                        match (hooks.wait)(until) {
                            crate::NextCall::Allowed => continue,
                            crate::NextCall::Halted => break Completion::Cancelled,
                        }
                    }
                    crate::Disposal::Switch { to, .. } => {
                        let to = to.clone();
                        run.record_fired(ledger, t, &disposal)?;
                        model.select_account(&to);
                        continue;
                    }
                    crate::Disposal::Freeze {
                        reason: crate::FreezeReason::AccountsExhausted,
                    } => {
                        let exhausted = run.state.watchdog.exhausted();
                        run.record_refusal(ledger, t, &exhausted)?;
                        run.state.failure = Some(exhausted);
                    }
                    crate::Disposal::Proceed
                    | crate::Disposal::CorrectiveSteer { .. }
                    | crate::Disposal::Freeze {
                        reason: crate::FreezeReason::Stall | crate::FreezeReason::ProviderRefused,
                    } => {
                        ledger.append(run.line(t, kind, Payload::of(&err)?))?;
                    }
                }

                run.state.failure.get_or_insert(err);
                break Completion::Cancelled;
            }
        }
    };
    run.freeze(ledger, handoff, ending, hooks)
}
