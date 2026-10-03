// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One drive as a lane runs it: what it borrows from the city, the
//! three hooks that touch the ledger while it goes, and who may
//! interrupt it.
//!
//! It is a free function rather than a method because a lane is a
//! thread that holds no worker (`crates/sprawling/Spec.lean` §8-46-1): everything a
//! drive needs from the city arrives in [`DriveContext`], which is five
//! handles that clone, and the ledger arrives as a parameter — the
//! accounting thread hands its own, and a lane hands a
//! [`Relay`](crate::worker::relay::Relay).

use kernel::{AxError, Ledger};
use kernel::{RunId, TimeMs};
use runtime::run::{RunHooks, SafePoint, drive};
use runtime::{Interrupt, NextCall};

use super::placing::{Checkpointing, Placing};
use super::{Driven, Driving};

/// What one drive takes from the city, in handles rather than in loans.
///
/// Every field clones, and that is the point: N drives going at once
/// each hold their own, so none of them holds the worker. The ledger is
/// not among them because it is not shared — it arrives as a parameter,
/// and which one arrives is what separates the accounting thread from a
/// lane.
#[derive(Clone)]
pub(crate) struct DriveContext {
    /// Where a model's text goes while it is still arriving.
    pub watching: Option<std::sync::Arc<dyn Fn(wire::Delta) + Send + Sync>>,
    /// What the person asked of this run, read at its safe points. One
    /// handle per drive, all of them reading the same desk by run id:
    /// a steer and a cancel reach the run they name and no other
    /// (`crates/sprawling/Spec.lean` §8-42-1).
    pub person: Option<std::sync::Arc<dyn Fn(RunId) -> Interrupt + Send + Sync>>,
    /// What is still running while the runs go on, so a halt on a scope
    /// reaches a run inside it.
    pub backlog: runtime::Backlog,
    /// The worker's own clock, so a run's lines and the worker's are
    /// read from one time (`crates/accounting/spec/Clock.lean` §8-3).
    pub clock: std::sync::Arc<dyn crate::Clock + Send + Sync>,
    /// The monotonic clock a run's durations are read off
    /// (`Hands.monotonic`, kernel D20).
    pub monotonic: fn() -> std::time::Instant,
}

/// Who may interrupt one drive, in rank order: the halt that reached
/// its backlog member, the person, then a neighbour's steer.
///
/// Two speakers, one landing, and the person outranks the resident.
/// What keeps them apart where the model reads them is `collab::Steer`:
/// only the person's entrance can write the `user` prefix, and a
/// resident's writes `@` and its own address - the address a reply is
/// sent to. A run that could not tell the two apart would answer the
/// person by signalling them, and answer a neighbour by talking to
/// nobody.
struct Interrupting {
    run_id: RunId,
    member: Option<runtime::BacklogId>,
    backlog: runtime::Backlog,
    person: Option<std::sync::Arc<dyn Fn(RunId) -> Interrupt + Send + Sync>>,
    steers: std::sync::Arc<std::sync::Mutex<collab::SignalDesk>>,
    /// What arrived while the run waited out a provider and was not a
    /// halt. The desk hands each steer out once, so one taken during a
    /// wait is kept here for the safe point that follows it.
    held: Option<Interrupt>,
}

impl Interrupting {
    /// Whether a halt has reached the run, asked while it waits out a
    /// provider. Anything else that arrives is held for the next safe
    /// point rather than answered here.
    fn halted(&mut self) -> bool {
        if self.held.is_some() {
            return self.scope_stopping();
        }
        match self.ask() {
            Interrupt::Cancel => true,
            Interrupt::None => false,
            steer @ Interrupt::Steer { .. } => {
                self.held = Some(steer);
                false
            }
        }
    }

    /// A stopped scope outranks anything a person or a neighbour still
    /// has to say to the run.
    fn scope_stopping(&self) -> bool {
        scope_stopping(&self.backlog, self.member)
    }

    fn ask(&mut self) -> Interrupt {
        if self.scope_stopping() {
            return Interrupt::Cancel;
        }
        if let Some(held) = self.held.take() {
            return held;
        }
        let from_person = match self.person.as_ref() {
            Some(ask) => ask(self.run_id),
            None => Interrupt::None,
        };
        if !matches!(from_person, Interrupt::None) {
            return from_person;
        }
        // A desk nobody can take answers nothing rather than refusing:
        // a safe point is the wrong place to fail over a lock, and the
        // drive's own end will report it.
        let Ok(mut desk) = self.steers.lock() else {
            return Interrupt::None;
        };
        match desk.take_steer() {
            Ok(Some(steer)) => Interrupt::Steer {
                source: steer.source().to_owned(),
                text: steer.text().to_owned(),
            },
            Ok(None) => Interrupt::None,
            // A signal the desk took out of the queue and could not read as
            // a steer does not interrupt, and the ledger still has it: the
            // desk records the consumption before it reports the refusal.
            // A safe point is not the place to stop a run over a message it
            // cannot act on, which is the answer the person's own entrance
            // gives an empty steer (crate::worker::desk). What must not happen is
            // the two arriving here as one case; they do not.
            Err(_) => Interrupt::None,
        }
    }
}

/// Whether the scope a run's backlog member sits in is being stopped,
/// for a model run and a harness run alike (`crates/sprawling/Spec.lean` §8-124).
///
/// A backlog that cannot answer counts as stopped: it cannot promise the
/// scope is still open, and a run that carried on would be running
/// inside a scope a person may already have shut (`crates/sprawling/Spec.lean`
/// §8-73).
pub(super) fn scope_stopping(
    backlog: &runtime::Backlog,
    member: Option<runtime::BacklogId>,
) -> bool {
    member.is_some_and(|id| backlog.stopping(id).unwrap_or(true))
}

/// Runs the plan, and hands back what the drive left behind.
///
/// The three hooks live here because they are the only code that
/// touches the ledger while the driver owns it: one interrupt source
/// merging the person and the residents, one checkpoint going up before each
/// wave, and one tool face placing each call's key by its position.
/// Everything they collect - the commits a wave checkpointed against, what
/// the run's own commands did, and the items a gate raised - is theirs
/// only for the length of the drive, so it comes back as one value
/// rather than as four cells the caller has to keep in step.
///
/// The drive's own outcome stays a `Result` inside [`Driven`] rather
/// than being propagated: a run that failed still has desks to settle,
/// and settling them is what puts its last lines on the history.
///
/// **Generic in the ledger, and that is the whole crossing.** On the
/// accounting thread `L` is the city's `JsonlLedger`; on a lane it is
/// the relay, which carries each append to that same thread and waits
/// for it. One drive, one code path, one writer.
///
/// # Errors
/// Propagates a checkpoint that will not open, which is the one failure
/// that happens before the run starts.
pub(crate) fn drive_run<L: Ledger>(
    driving: Driving,
    ledger: &mut L,
    context: DriveContext,
) -> Result<Driven, AxError> {
    let Driving {
        mut adapter,
        bench,
        signals,
        write_root,
        checkpoint_scope,
        run_id,
        of,
        sieving,
        member,
        plan,
        handoff,
        workbench,
    } = driving;
    let DriveContext {
        watching,
        person,
        backlog,
        clock,
        monotonic,
    } = context;
    // Every reading the driver takes is kept where a command's clock
    // line and `status` read it, so neither needs a clock of its own.
    let reading = sieving.clock.clone();
    let mut now = || clock.now().inspect(|at| reading.keep(*at));
    let origin = monotonic();
    // Saturates rather than fails: u64 microseconds reach past half a
    // million years, so the cap is never a reading a run can take.
    let mut monotonic_us = || {
        u64::try_from(monotonic().saturating_duration_since(origin).as_micros()).unwrap_or(u64::MAX)
    };
    let declared = bench.declared_writes();
    // This run's own index: two nodes of one ready set drive at once by
    // design, and on one shared index libgit2's `.git/index.lock` refused
    // the second checkpoint (`crates/sprawling/spec/Accounting/Views.lean` §8-46-13).
    let mut checkpoint_handle = storage::Checkpoint::open_writer(&write_root, run_id)
        .map_err(storage::StorageError::into_ax)?;
    // What the wave checkpoint and the bench checkpointed, in the order they went
    // up, so the sweep afterwards knows which commit a deleted file can
    // be restored from; and what the calls since the last checkpoint said they
    // wrote, which is what the next checkpoint stages.
    let checkpointing = Checkpointing::opened();
    let asking = std::cell::RefCell::new(Interrupting {
        run_id,
        member,
        backlog,
        person,
        steers: signals,
        held: None,
    });
    let (driven, ran) = {
        let mut interrupt = |_: SafePoint| asking.borrow_mut().ask();
        // How late a halt may land while a run waits out a provider. It
        // is the scale a person notices, not a reading of this machine.
        const HALT_SLICE_MS: u64 = 50;
        let mut wait = |until: TimeMs| loop {
            if asking.borrow_mut().halted() {
                return NextCall::Halted;
            }
            // A clock that cannot be read sends at once: the turn that
            // follows samples the same clock and reports its failure.
            let Ok(at) = clock.now() else {
                return NextCall::Allowed;
            };
            let left = until.value().saturating_sub(at.value());
            if left == 0 {
                return NextCall::Allowed;
            }
            std::thread::sleep(std::time::Duration::from_millis(left.min(HALT_SLICE_MS)));
        };
        let mut placing = Placing::new(bench, sieving, run_id, &checkpointing)
            .resolving(std::sync::Arc::clone(&workbench.catalog));
        let mut checkpoint = |t: TimeMs| {
            let scope = checkpointing.take_scope(&checkpoint_scope);
            let payload = checkpoint_handle
                .wave_pre(&scope, t, &of)
                .map_err(storage::StorageError::into_ax)?;
            if let Some(oid) = payload
                .as_map()
                .get("oid")
                .and_then(serde_json::Value::as_str)
            {
                checkpointing.checkpointed.borrow_mut().push(oid.to_owned());
            }
            Ok(payload)
        };
        // Where text goes while the model is still saying it. The sink
        // is whatever the assembly layer was given - a socket task in a
        // running city, nothing at all in citysim and in replay - so a
        // run with nobody watching asks the provider for no stream and
        // behaves exactly as it always did.
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
    // The run writes nothing more here. A crash that skips this leaves the
    // index for `Checkpoint::sweep_writers` when the city next opens.
    checkpoint_handle
        .close_writer()
        .map_err(storage::StorageError::into_ax)?;
    Ok(Driven {
        outcome: driven,
        adapter,
        checkpointed: checkpointing.checkpointed.into_inner(),
        ran,
        // Nothing a door answers reaches a person any more: a door
        // answers Allow or Deny. The sweep is the one thing that still
        // raises a question, and it raises it after this returns.
        raised: Vec::new(),
        workbench,
    })
}
