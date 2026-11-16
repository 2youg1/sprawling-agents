// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxError, Ledger};
use kernel::{RunId, TimeMs};
use runtime::run::{RunHooks, SafePoint, drive};
use runtime::{Interrupt, NextCall};

use super::placing::{Checkpointing, Placing};
use super::{Driven, Driving};

mod interrupting;
use interrupting::Interrupting;

#[derive(Clone)]
pub(crate) struct DriveContext {

    pub watching: Option<std::sync::Arc<dyn Fn(wire::Delta) + Send + Sync>>,

    pub person: Option<std::sync::Arc<dyn Fn(RunId) -> Interrupt + Send + Sync>>,

    pub backlog: runtime::Backlog,

    pub clock: std::sync::Arc<dyn crate::Clock + Send + Sync>,

    pub monotonic: fn() -> std::time::Instant,
}

const HALT_SLICE_MS: u64 = 50;

pub(super) fn scope_stopping(
    backlog: &runtime::Backlog,
    member: Option<runtime::BacklogId>,
) -> bool {
    member.is_some_and(|id| backlog.stopping(id).unwrap_or(true))
}

pub(crate) fn drive_run<L: Ledger>(
    driving: Driving,
    ledger: &mut L,
    context: DriveContext,
) -> Result<Driven, AxError> {
    let Driving {
        mut adapter,
        bench,
        signals,
        policy,
        write_root,
        checkpoint_scope,
        checkpoint_entropy_allow,
        run_id,
        of,
        sieving,
        member,
        plan,
        handoff,
        delivered,
        workbench,
    } = driving;
    let DriveContext {
        watching,
        person,
        backlog,
        clock,
        monotonic,
    } = context;

    let reading = sieving.clock.clone();
    let mut now = || clock.now().inspect(|at| reading.keep(*at));
    let origin = monotonic();

    let mut monotonic_us = || {
        u64::try_from(monotonic().saturating_duration_since(origin).as_micros()).unwrap_or(u64::MAX)
    };
    let declared = bench.declared_writes();

    let mut checkpoint_handle = storage::Checkpoint::open_writer(&write_root, run_id)
        .map_err(storage::StorageError::into_ax)?
        .with_entropy_allowlist(checkpoint_entropy_allow);

    let checkpointing = Checkpointing::opened();
    let asking = std::cell::RefCell::new(Interrupting {
        run_id,
        member,
        backlog,
        person,
        steers: signals,
        policy,
        held: None,
    });
    let (driven, ran) = {
        let mut interrupt = |point: SafePoint| {
            let mut asking = asking.borrow_mut();
            match point {
                SafePoint::BeforeWave { .. } => asking.answered(),

                SafePoint::BeforeAssemble { .. } => {
                    if let Some(ended) = asking.wait_for_reply(clock.as_ref()) {
                        return ended;
                    }
                }
                SafePoint::BeforeCall { .. }
                | SafePoint::BeforeToolCall { .. }
                | SafePoint::BeforeSpawn { .. } => {}
            }
            asking.ask()
        };
        let mut wait = |until: TimeMs| loop {
            if asking.borrow_mut().halted() {
                return NextCall::Halted;
            }

            let Ok(at) = clock.now() else {
                return NextCall::Allowed;
            };
            let left = until.value().saturating_sub(at.value());
            if left == 0 {
                return NextCall::Allowed;
            }
            std::thread::sleep(std::time::Duration::from_millis(left.min(HALT_SLICE_MS)));
        };
        let halted = || asking.borrow_mut().halted();
        let mut placing = Placing::new(bench, sieving, run_id, &checkpointing)
            .stopping(&halted)
            .reading(workbench.shown(), delivered)
            .ruling(workbench.rule_candidates())
            .resolving(std::sync::Arc::clone(&workbench.catalog));
        let mut checkpoint = |t: TimeMs| {
            let scope = checkpointing.take_scope(&checkpoint_scope);
            let payload = checkpoint_handle
                .wave_pre(&scope, t, &of)
                .map_err(runtime::CheckpointFailure::from)?;
            if let Some(oid) = payload
                .as_map()
                .get("oid")
                .and_then(serde_json::Value::as_str)
            {
                checkpointing.checkpointed.borrow_mut().push(oid.to_owned());
            }
            Ok(payload)
        };

        let mut watched = |held: &kernel::Increment| {
            if let Some(onto) = watching.as_ref() {
                onto(wire::Delta {
                    run: run_id,
                    increment: held.clone(),
                });
            }
        };
        let mut hooks = RunHooks {
            now: &mut now,
            monotonic_us: &mut monotonic_us,
            interrupt: &mut interrupt,
            checkpoint: Some(&mut checkpoint),
            writes: &|call: &kernel::ToolCall| declared.of(call),
            invoke: &mut placing,
            wait: &mut wait,
            deltas: watching.is_some().then_some(&mut watched),
        };
        let driven = drive(plan, ledger, &mut adapter, &mut hooks, &handoff);
        (driven, placing.ran())
    };

    checkpoint_handle
        .close_writer()
        .map_err(storage::StorageError::into_ax)?;
    Ok(Driven {
        outcome: driven,
        adapter,
        checkpointed: checkpointing.checkpointed.into_inner(),
        ran,

        raised: Vec::new(),
        workbench,
    })
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod wait_tests;
