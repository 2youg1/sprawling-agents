// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One drive, and what it leaves behind.

use std::path::Path;

use kernel::{AxCode, AxError};
use kernel::{RunId, TimeMs};
use runtime::Interrupt;
use runtime::bench::{BenchOutcome, ToolBench};
use runtime::run::{RunHooks, RunPlan, SafePoint, drive};

use super::{RunWorker, now_ms};

/// What one drive is handed: the machinery it runs on, and the run it
/// runs as.
///
/// Seven values that arrive together and are meaningless apart - a bench
/// without the resident that invokes it derives the wrong key, a fence
/// scope without the tree it fences covers the wrong files. They were
/// seven parameters, and `#[expect(clippy::too_many_arguments)]` sat
/// above them saying so.
pub(super) struct Driving<'a> {
    /// The model this run calls, already chosen and already credentialed.
    /// Owned, and handed back inside [`Driven`]: who holds the adapter is
    /// a fact the types state, not a loan the reader has to track.
    pub(super) adapter: Box<dyn kernel::Model + Send>,
    /// What routes a call the model makes.
    pub(super) bench: &'a mut ToolBench,
    /// Where a steer from a resident lands while the drive is going.
    pub(super) signals: &'a std::rc::Rc<std::cell::RefCell<collab::SignalDesk>>,
    /// The tree the run writes in: its own worktree under review, the
    /// city itself otherwise.
    pub(super) write_root: &'a Path,
    /// What a checkpoint fence covers, from [`Site::fence_scope`].
    pub(super) fence_scope: String,
    /// The resident this run works as, as three hooks will name it.
    pub(super) who: &'a str,
    pub(super) run_id: RunId,
    /// What every fence this drive raises is signed with (card-2.1).
    pub(super) of: memory::Provenance,
}

/// What one drive left behind, beside the run it froze.
///
/// Every field is written by a hook while the driver owns the ledger and
/// read after it gives it back, so none of them may be acted on until the
/// drive has returned.
pub(super) struct Driven {
    pub(super) outcome: Result<runtime::Run<runtime::run::Frozen>, AxError>,
    /// The adapter, home from the drive. The caller takes it apart back
    /// into the site it came from: a `Site` gains and loses no field.
    pub(super) adapter: Box<dyn kernel::Model + Send>,
    /// The commits each wave fenced against; the first is what the sweep
    /// restores a discarded file from.
    pub(super) fenced: Vec<String>,
    /// The run's own commands, as (passed, failed).
    pub(super) ran: (u32, u32),
    /// What a gate escalated while the ledger was not the worker's.
    pub(super) raised: Vec<kernel::ApprovalItem>,
}

impl RunWorker {
    /// Runs the plan, and hands back what the drive left behind.
    ///
    /// The three hooks live here because they are the only code that
    /// touches the ledger while the driver owns it: one interrupt source
    /// merging the person and the residents, one fence going up before
    /// each wave, and one invocation point deriving the key for a call.
    /// Everything they collect - the commits a wave fenced against, what
    /// the run's own commands did, and the items a gate raised - is
    /// theirs only for the length of the drive, so it comes back as one
    /// value rather than as four cells the caller has to keep in step.
    ///
    /// The drive's own outcome stays a `Result` inside [`Driven`] rather
    /// than being propagated: a run that failed still has desks to
    /// settle, and settling them is what puts its last lines on the
    /// history.
    ///
    /// # Errors
    /// Propagates a checkpoint that will not open, which is the one
    /// failure that happens before the run starts.
    pub(super) fn drive_dispatch(
        &mut self,
        plan: RunPlan,
        handoff: &runtime::handoff::Handoff,
        driving: Driving<'_>,
    ) -> Result<Driven, AxError> {
        let Driving {
            mut adapter,
            bench,
            signals,
            write_root,
            fence_scope,
            who,
            run_id,
            of,
        } = driving;
        let mut now = || now_ms();
        // Taken before the hooks borrow `self`: the sink outlives one
        // drive and belongs to the worker, not to this call.
        let watching = self.watching.clone();
        let bench_who = who.to_owned();
        let mut fence_point =
            memory::Checkpoint::open(write_root).map_err(memory::MemoryError::into_ax)?;
        // What the bench fenced, so the sweep afterwards knows which
        // commit a deleted file can be restored from.
        let fenced: std::rc::Rc<std::cell::RefCell<Vec<String>>> =
            std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let fenced_by_bench = std::rc::Rc::clone(&fenced);
        // Taken for the length of the drive and put back after: the
        // hooks cannot borrow the worker, and a source that stayed
        // behind would be a second one.
        let mut source = self.interrupts.take();
        // What the run's own commands did. It is the only evidence of
        // "the tests passed" the city can observe without being told,
        // and being told is what a mode is supposed to check.
        let ran: std::rc::Rc<std::cell::RefCell<(u32, u32)>> =
            std::rc::Rc::new(std::cell::RefCell::new((0, 0)));
        let ran_by_bench = std::rc::Rc::clone(&ran);
        // What the bench raised while the driver held the ledger.
        let raised: std::rc::Rc<std::cell::RefCell<Vec<kernel::ApprovalItem>>> =
            std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let raised_by_bench = std::rc::Rc::clone(&raised);
        let driven = {
            let raised = raised_by_bench;
            let fenced = fenced_by_bench;
            let ran = ran_by_bench;
            // Two speakers, one landing, and the person outranks the
            // resident. What keeps them apart where the model reads them
            // is `collab::Steer`: only the person's entrance can write
            // the `user` prefix, and a resident's writes `@` and its own
            // address - the address a reply is sent to. A run that could
            // not tell the two apart would answer the person by
            // signalling them, and answer a neighbour by talking to
            // nobody.
            let steers = std::rc::Rc::clone(signals);
            let mut interrupt = |_: SafePoint| {
                let from_person = match source.as_mut() {
                    Some(ask) => ask(run_id),
                    None => Interrupt::None,
                };
                if !matches!(from_person, Interrupt::None) {
                    return from_person;
                }
                // A desk in use answers nothing rather than refusing:
                // the interrupt runs between tool calls, so this borrow
                // is free in practice, and a safe point is the wrong
                // place to fail over a lock.
                let Ok(mut desk) = steers.try_borrow_mut() else {
                    return Interrupt::None;
                };
                match desk.take_steer() {
                    Some(steer) => Interrupt::Steer {
                        source: steer.source().to_owned(),
                        text: steer.text().to_owned(),
                    },
                    None => Interrupt::None,
                }
            };
            // Where this call sits in this run. The key used to derive
            // from the turn's millisecond stamp and the tool's name,
            // which broke twice over: it took a clock, which determinism
            // rule 7 forbids outright, and it ignored the arguments - so
            // two `read`s of two different files in one turn were one
            // key, and the second came back "this call was already
            // made". A model reads that as a fault in itself.
            let placed = std::cell::Cell::new(0u64);
            let mut invoke = |call: &kernel::ToolCall, t: TimeMs| {
                let at = placed.get();
                placed.set(at.saturating_add(1));
                // What the action is, is the tool face's to say
                // (kernel-SPEC 8-23). Two identical calls at two
                // positions are two keys and both run; the same position
                // replayed is one key, which is what deduplication is
                // for.
                let key = kernel::IdemKey::derive(&run_id, kernel::Seq::new(at), &call.action()?);
                let ctx = kernel::GateContext {
                    actor: bench_who.clone(),
                    now: t,
                    item_id: kernel::ApprovalId::new(format!("item-{}", t.value())).ok_or_else(
                        || AxError::failure(AxCode::InvalidArgs, "mint approval id", "empty id"),
                    )?,
                };
                match bench.invoke(call, &key, &ctx)? {
                    BenchOutcome::Ran {
                        outcome,
                        fenced: at,
                    } => {
                        if let Some(oid) = at {
                            fenced.borrow_mut().push(oid);
                        }
                        if call.name.as_str() == "exec" {
                            let failed = outcome
                                .result
                                .as_map()
                                .get("exit_code")
                                .and_then(serde_json::Value::as_i64)
                                .is_some_and(|code| code != 0);
                            let mut counts = ran.borrow_mut();
                            if failed {
                                counts.1 = counts.1.saturating_add(1);
                            } else {
                                counts.0 = counts.0.saturating_add(1);
                            }
                        }
                        Ok(outcome)
                    }
                    BenchOutcome::Refused { refusal } => Err(*refusal),
                    BenchOutcome::Pending { item } => {
                        // Stashed rather than recorded here: the ledger is
                        // the driver's for the length of the run, and one
                        // writer is the whole point. The record is written
                        // the moment the drive returns.
                        let id = item.id.as_str().to_owned();
                        raised.borrow_mut().push(*item);
                        Err(
                            AxError::failure(AxCode::ApprovalPending, "await approval", id)
                                .with_recovery(
                                    "answer the approval in the inbox, then dispatch again",
                                ),
                        )
                    }
                    BenchOutcome::Duplicate => Err(AxError::failure(
                        AxCode::InvalidArgs,
                        "invoke tool",
                        "this call was already made",
                    )),
                }
            };
            let mut fence = |t: TimeMs| {
                let payload = fence_point
                    .wave_pre(&fence_scope, t, &of)
                    .map_err(memory::MemoryError::into_ax)?;
                if let Some(oid) = payload
                    .as_map()
                    .get("oid")
                    .and_then(serde_json::Value::as_str)
                {
                    fenced.borrow_mut().push(oid.to_owned());
                }
                Ok(payload)
            };
            // Where text goes while the model is still saying it. The
            // sink is whatever the assembly layer was given - a socket
            // task in a running city, nothing at all in citysim and in
            // replay - so a run with nobody watching asks the provider
            // for no stream and behaves exactly as it always did.
            let mut watched = |said: &str| {
                if let Some(onto) = watching.as_ref() {
                    onto(channels::Delta {
                        run: run_id,
                        text: said.to_owned(),
                    });
                }
            };
            let mut hooks = RunHooks {
                now: &mut now,
                interrupt: &mut interrupt,
                fence: Some(&mut fence),
                invoke: &mut invoke,
                deltas: watching.is_some().then_some(&mut watched),
            };
            drive(
                plan,
                &mut self.ledger,
                adapter.as_mut(),
                &mut hooks,
                handoff,
            )
        };
        self.interrupts = source;
        Ok(Driven {
            outcome: driven,
            adapter,
            fenced: fenced.borrow().clone(),
            ran: *ran.borrow(),
            raised: raised.borrow().clone(),
        })
    }
}

#[cfg(test)]
mod tests;
