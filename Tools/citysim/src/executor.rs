// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::cell::Cell;

use kernel::{
    Address, AxCode, AxError, B3Hash, BuildingPolicy, FrozenConfig, IdemKey, Locator, Payload,
    RunId, Seq, Temporal, TimeMs,
};
use runtime::bench::{BenchOutcome, ToolBench};
use runtime::clock::StampGate;
use runtime::handoff::Handoff;
use runtime::pipeline;
use runtime::prefix::{FrozenPrefix, FrozenSegment, SegmentSlot};
use runtime::run::{RunHooks, RunPlan, SafePoint, drive};
use runtime::turn::{CallShape, Interrupt};
use serde_json::{Map, Value};
use storage::Checkpoint;

use crate::mem_ledger::MemLedger;
use crate::script_model::ScriptModel;
use crate::sieving::{SieveWorld, package_exec};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancelPoint {
    BeforeAssemble { turn: u32 },
    BeforeCall { turn: u32 },
    BeforeWave { turn: u32 },
}

pub struct Scenario {
    pub run: RunId,
    pub who: String,
    pub addr: Address,
    pub task: String,
    pub goal: String,
    pub job_md: String,
    pub model: ScriptModel,

    pub bench: ToolBench,

    pub config: FrozenConfig,

    pub checkpoint: Option<(Checkpoint, Vec<String>)>,
    pub cancel: Option<CancelPoint>,

    pub steer: Option<(u32, String)>,

    pub sieve: Option<SieveWorld>,
}

pub struct ScenarioReport {
    pub lines: Vec<Vec<u8>>,
    pub completion: &'static str,
}

fn payload(map: Map<String, Value>) -> Result<Payload, AxError> {
    Payload::new(map)
}

fn job_locator(addr: &Address, job_md: &str) -> Result<Locator, AxError> {
    let digest = B3Hash::digest(job_md.as_bytes()).to_string();
    let oid: String = digest.chars().take(40).collect();
    Locator::parse(&format!("file:{}@{}", addr.as_str(), oid))
}

fn answer_at(
    point: SafePoint,
    cancel: Option<CancelPoint>,
    steer: Option<&(u32, String)>,
) -> Interrupt {
    let (here, index) = match point {
        SafePoint::BeforeAssemble { turn } => (CancelPoint::BeforeAssemble { turn }, turn),
        SafePoint::BeforeCall { turn } => (CancelPoint::BeforeCall { turn }, turn),
        SafePoint::BeforeWave { turn } => (CancelPoint::BeforeWave { turn }, turn),

        SafePoint::BeforeToolCall { .. } | SafePoint::BeforeSpawn { .. } => {
            return Interrupt::None;
        }
    };
    if cancel == Some(here) {
        return Interrupt::Cancel;
    }
    let wave = matches!(point, SafePoint::BeforeWave { .. });
    match steer {
        Some((at, text)) if wave && *at == index => Interrupt::Steer {
            speaker: runtime::conversation::Speaker::Person,
            text: text.clone(),
        },
        _ => Interrupt::None,
    }
}

fn clock_overflow() -> AxError {
    AxError::failure(
        AxCode::InvalidArgs,
        "advance scenario clock",
        "u64 overflow",
    )
    .with_recovery(
        "shorten this scenario or lower its per-step tick: the clock ran past the \
         milliseconds a u64 counts",
    )
}

fn next_place(placed: &Cell<Option<u64>>) -> Result<Seq, AxError> {
    let at = placed.get().ok_or_else(|| {
        AxError::failure(AxCode::InvalidArgs, "place a tool call", "u64 overflow").with_recovery(
            "split this scenario into shorter runs: every place a u64 counts is taken",
        )
    })?;
    placed.set(at.checked_add(1));
    Ok(Seq::new(at))
}

pub fn run_scenario(scenario: Scenario) -> Result<ScenarioReport, AxError> {
    let mut ledger = MemLedger::new();
    run_scenario_on(&mut ledger, scenario)
}

pub fn run_scenario_on(
    ledger: &mut MemLedger,
    scenario: Scenario,
) -> Result<ScenarioReport, AxError> {
    let Scenario {
        run,
        who,
        addr,
        task,
        goal,
        job_md,
        mut model,
        mut bench,
        config,
        mut checkpoint,
        cancel,
        steer,
        mut sieve,
    } = scenario;
    let mut stamps = StampGate::new(config.clock_stamp, config.clock_zones.clone());
    let job = job_locator(&addr, &job_md)?;

    let tick = Cell::new(0u64);
    let mut now = || {
        let value = tick.get();
        tick.set(value.checked_add(1).ok_or_else(clock_overflow)?);
        Ok(TimeMs::new(value))
    };
    let mut interrupt = |point: SafePoint| answer_at(point, cancel, steer.as_ref());

    let handoff = Handoff::new(
        vec![job.clone()],
        "scenario run".to_owned(),
        "see roadmap".to_owned(),
        "scripted world".to_owned(),
        "resume from the job file".to_owned(),
    )?;

    let prefix = FrozenPrefix::assemble(
        FrozenSegment::new(SegmentSlot::City, b"citysim city segment".to_vec()),
        FrozenSegment::new(SegmentSlot::Building, addr.as_str().as_bytes().to_vec()),
        FrozenSegment::new(SegmentSlot::Resident, who.as_bytes().to_vec()),
        FrozenSegment::new(SegmentSlot::Run, job.to_string().into_bytes()),
    )?;

    let plan = RunPlan {
        run,
        who: who.clone(),
        addr: addr.clone(),
        task,
        goal,
        opening: runtime::Opening::FromJob,
        job,
        parent: None,
        predecessor: None,
        dispatched_by: kernel::event::Who::Person,
        routing: None,
        run_policy: runtime::PolicyCell::new(kernel::RunPolicy::of(kernel::Mode::Work)),
        naming: None,

        inherited: Vec::new(),
        shape: CallShape {
            model: "script".to_owned(),
            generation: kernel::FrozenGeneration {
                max_tokens: kernel::Ceiling::new(4096),
                ..kernel::FrozenGeneration::default()
            },
            effort: None,
            context_tokens: 0,
        },
        second_threshold: None,
        context_lock: None,
        succession: std::sync::Arc::new(std::sync::Mutex::new(runtime::SuccessionDesk::new())),
        context: runtime::ContextReading::default(),
        prefix,
        policy: BuildingPolicy::default(),
        tools: Vec::new(),
        skills: Vec::new(),
        retries: kernel::Retries::UntilHalted,
    };

    let placed = Cell::new(Some(0u64));
    let mut invoke = |call: &kernel::ToolCall, t: TimeMs| {

        let key = IdemKey::derive(&run, next_place(&placed)?, &call.action()?);
        let temporal = bench
            .meta_of(&call.name)
            .map_or(Temporal::Timeless, |meta| meta.temporal);
        match bench.invoke(call, &key, t)? {

            BenchOutcome::Ran { outcome, .. } | BenchOutcome::Duplicate { outcome } => {

                let stamp = stamps.observe(t, temporal)?;
                if let Some(world) = sieve.as_mut()
                    && call.name.as_str() == kernel::ToolName::EXEC
                {
                    return package_exec(call, &outcome, world, stamp);
                }
                let bytes = serde_json::to_vec(&outcome.result).map_err(|err| {
                    AxError::failure(AxCode::InvalidArgs, "encode tool result", err.to_string())
                        .with_recovery(
                            "report this against the scripted tool that answered: a \
                             result payload holds strings and whole numbers only",
                        )
                })?;
                let packaged = pipeline::package(
                    &bytes,
                    pipeline::PackContext {
                        cap_bytes: 16_384,
                        stamp,
                        net_notice: false,
                        steer: None,
                        reminder: None,
                        offload: None,
                        sieve: None,

                        adviser: None,
                    },
                )?;
                let mut wrapped = Map::new();
                wrapped.insert("content".to_owned(), Value::String(packaged.content));
                Ok(kernel::ToolOutcome {
                    result: payload(wrapped)?,
                    attachments: Vec::new(),
                })
            }

            BenchOutcome::Refused { refusal } => Err(*refusal),
        }
    };

    let frozen = match checkpoint.as_mut() {
        Some((net, scope)) => {

            let of = storage::Provenance::new(
                run,
                addr.clone(),
                kernel::B3Hash::digest(who.as_bytes()),
                storage::ModelChoice {
                    id: "script".to_owned(),
                    effort: None,
                },
            );
            let mut checkpoint = |t: TimeMs| {
                net.wave_pre(scope, t, &of)
                    .map_err(runtime::CheckpointFailure::from)
            };
            let mut hooks = RunHooks {
                now: &mut now,

                monotonic_us: &mut || 0,
                interrupt: &mut interrupt,
                checkpoint: Some(&mut checkpoint),
                writes: &|_: &kernel::ToolCall| kernel::Writes::Domain,
                invoke: &mut invoke,
                wait: &mut |_: TimeMs| runtime::NextCall::Allowed,
                deltas: None,
            };
            drive(plan, ledger, &mut model, &mut hooks, &handoff)?
        }
        None => {
            let mut hooks = RunHooks {
                now: &mut now,

                monotonic_us: &mut || 0,
                interrupt: &mut interrupt,
                checkpoint: None,
                writes: &|_: &kernel::ToolCall| kernel::Writes::Domain,
                invoke: &mut invoke,
                wait: &mut |_: TimeMs| runtime::NextCall::Allowed,
                deltas: None,
            };
            drive(plan, ledger, &mut model, &mut hooks, &handoff)?
        }
    };

    Ok(ScenarioReport {
        lines: ledger.raw_lines().to_vec(),
        completion: frozen.completion().name(),
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, reason = "test code")]
mod tests {
    use super::*;

    #[test]
    fn the_call_after_the_last_place_is_refused() {
        let placed = Cell::new(Some(u64::MAX));
        assert_eq!(next_place(&placed).unwrap(), Seq::new(u64::MAX));
        let err = next_place(&placed).unwrap_err();
        assert_eq!(
            (err.code(), err.action()),
            (&AxCode::InvalidArgs, "place a tool call")
        );
    }
}
