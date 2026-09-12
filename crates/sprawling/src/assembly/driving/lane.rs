// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One drive as a lane runs it: what it borrows from the city, the
//! three hooks that touch the ledger while it goes, and who may
//! interrupt it.
//!
//! It is a free function rather than a method because a lane is a
//! thread that holds no worker (sprawling-SPEC.md 8-46-1): everything a
//! drive needs from the city arrives in [`DriveContext`], which is four
//! handles that clone, and the ledger arrives as a parameter — the
//! accounting thread hands its own, and a lane hands a
//! [`Relay`](crate::serving::Relay).

use kernel::{AxCode, AxError, Ledger};
use kernel::{RunId, TimeMs};
use runtime::Interrupt;
use runtime::bench::BenchOutcome;
use runtime::run::{RunHooks, SafePoint, drive};

use super::{Driven, Driving};
use crate::assembly::now_ms;

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
    pub(crate) watching: Option<std::sync::Arc<dyn Fn(channels::Delta) + Send + Sync>>,
    /// What the person asked of this run, read at its safe points. One
    /// handle per drive, all of them reading the same desk by run id:
    /// a steer and a cancel reach the run they name and no other
    /// (sprawling-SPEC.md 8-42-1).
    pub(crate) person: Option<std::sync::Arc<dyn Fn(RunId) -> Interrupt + Send + Sync>>,
    /// What is still running while the runs go on, so a halt on a scope
    /// reaches a run inside it.
    pub(crate) backlog: runtime::Backlog,
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
}

impl Interrupting {
    fn ask(&mut self) -> Interrupt {
        // A stopped scope outranks anything a person or a neighbour
        // still has to say to the run.
        if self
            .member
            .is_some_and(|id| self.backlog.stopping(id).unwrap_or(false))
        {
            return Interrupt::Cancel;
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
            Some(steer) => Interrupt::Steer {
                source: steer.source().to_owned(),
                text: steer.text().to_owned(),
            },
            None => Interrupt::None,
        }
    }
}

/// Runs the plan, and hands back what the drive left behind.
///
/// The three hooks live here because they are the only code that
/// touches the ledger while the driver owns it: one interrupt source
/// merging the person and the residents, one fence going up before each
/// wave, and one invocation point deriving the key for a call.
/// Everything they collect - the commits a wave fenced against, what
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
        mut bench,
        signals,
        write_root,
        fence_scope,
        who,
        run_id,
        of,
        mut sieving,
        member,
        plan,
        handoff,
    } = driving;
    let DriveContext {
        watching,
        person,
        backlog,
    } = context;
    let mut now = || now_ms();
    let bench_who = who.clone();
    let mut fence_point =
        memory::Checkpoint::open(&write_root).map_err(memory::MemoryError::into_ax)?;
    // What the bench fenced, so the sweep afterwards knows which commit
    // a deleted file can be restored from.
    let fenced: std::rc::Rc<std::cell::RefCell<Vec<String>>> =
        std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let fenced_by_bench = std::rc::Rc::clone(&fenced);
    let mut asking = Interrupting {
        run_id,
        member,
        backlog,
        person,
        steers: signals,
    };
    // What the run's own commands did. It is the only evidence of "the
    // tests passed" the city can observe without being told, and being
    // told is what a mode is supposed to check.
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
        let mut interrupt = |_: SafePoint| asking.ask();
        // Where this call sits in this run. The key used to derive from
        // the turn's millisecond stamp and the tool's name, which broke
        // twice over: it took a clock, which determinism rule 7 forbids
        // outright, and it ignored the arguments - so two `read`s of two
        // different files in one turn were one key, and the second came
        // back "this call was already made". A model reads that as a
        // fault in itself.
        let placed = std::cell::Cell::new(0u64);
        let mut invoke = |call: &kernel::ToolCall, t: TimeMs| {
            let at = placed.get();
            placed.set(at.saturating_add(1));
            // What the action is, is the tool face's to say (kernel-SPEC
            // 8-23). Two identical calls at two positions are two keys
            // and both run; the same position replayed is one key, which
            // is what deduplication is for.
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
                    if call.name.as_str() != "exec" {
                        return Ok(outcome);
                    }
                    let failed = outcome
                        .result
                        .as_map()
                        .get("exit_code")
                        .and_then(serde_json::Value::as_i64)
                        .is_some_and(|code| code != 0);
                    {
                        let mut counts = ran.borrow_mut();
                        if failed {
                            counts.1 = counts.1.saturating_add(1);
                        } else {
                            counts.0 = counts.0.saturating_add(1);
                        }
                    }
                    sieving.package(call, outcome)
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
                            .with_recovery("answer the approval in the inbox, then dispatch again"),
                    )
                }
                // A replay is answered with what the first call
                // answered, sieved the same way. An error here would tell
                // the model its call failed when it succeeded
                // (runtime-SPEC.md 8-35). The command counters are not
                // touched: nothing ran this time.
                BenchOutcome::Duplicate { outcome } => {
                    if call.name.as_str() == "exec" {
                        sieving.package(call, outcome)
                    } else {
                        Ok(outcome)
                    }
                }
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
        // Where text goes while the model is still saying it. The sink
        // is whatever the assembly layer was given - a socket task in a
        // running city, nothing at all in citysim and in replay - so a
        // run with nobody watching asks the provider for no stream and
        // behaves exactly as it always did.
        let mut watched = |held: &kernel::Increment| {
            if let Some(onto) = watching.as_ref() {
                onto(channels::Delta {
                    run: run_id,
                    increment: held.clone(),
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
        drive(plan, ledger, adapter.as_mut(), &mut hooks, &handoff)
    };
    Ok(Driven {
        outcome: driven,
        adapter,
        fenced: fenced.borrow().clone(),
        ran: *ran.borrow(),
        raised: raised.borrow().clone(),
    })
}
