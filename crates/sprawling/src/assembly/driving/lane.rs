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

use kernel::{AxError, Ledger};
use kernel::{RunId, TimeMs};
use runtime::bench::BenchOutcome;
use runtime::run::{RunHooks, SafePoint, drive};
use runtime::{Interrupt, NextCall};

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
    /// One fence at a time per city.
    ///
    /// **A repository has one index, and a fence stages and commits it.**
    /// Two nodes of one ready set drive at once by design
    /// (`assembly::plans::pursuing`), so their fences can overlap, and
    /// libgit2's `.git/index.lock` refuses the second one: a run that lost
    /// that race ended as cancelled and its node was handed back as
    /// though its own done check had failed. The gate is the width of the
    /// act, not of the run - a lane waits out the few milliseconds
    /// another lane's commit takes, and nothing else about two runs is
    /// serialized.
    ///
    /// `memory::checkpoint::scan::write_index` still waits out a lock,
    /// and that is a different contender: another sprawling process on
    /// the same city, which no mutex here can see.
    pub(crate) fence_gate: std::sync::Arc<std::sync::Mutex<()>>,
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
    /// has to say to the run. A backlog that cannot answer counts as
    /// stopped: it cannot promise the scope is still open, and a run
    /// that carried on would be running inside a scope a person may
    /// already have shut (sprawling-SPEC.md 8-73).
    fn scope_stopping(&self) -> bool {
        self.member
            .is_some_and(|id| self.backlog.stopping(id).unwrap_or(true))
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
            // gives an empty steer (serving::desk). What must not happen is
            // the two arriving here as one case; they do not.
            Err(_) => Interrupt::None,
        }
    }
}

/// A lock nobody can take, as a refusal rather than a panic.
///
/// The gate is held for one commit's length and by a lane that cannot
/// panic while holding it (its own failures are values), so this arm is
/// unreachable; inventing a panic for it would cost the whole city for a
/// fact it can report instead.
fn poison(what: &str) -> AxError {
    AxError::failure(
        kernel::AxCode::StorageFatal,
        "take the fence gate",
        what.to_owned(),
    )
    .with_recovery("restart this city: a lock left by a dead thread cannot be trusted")
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
        fence_gate,
    } = context;
    let mut now = || now_ms();
    let mut fence_point =
        memory::Checkpoint::open(&write_root).map_err(memory::MemoryError::into_ax)?;
    // What the bench fenced, so the sweep afterwards knows which commit
    // a deleted file can be restored from.
    let fenced: std::rc::Rc<std::cell::RefCell<Vec<String>>> =
        std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let fenced_by_bench = std::rc::Rc::clone(&fenced);
    // What the calls since the last fence said they wrote, which is what
    // the next fence stages (runtime-SPEC 8-45). It opens as the whole
    // domain: no commit of this run vouches yet for the tree it opened on.
    let wrote = std::cell::RefCell::new(kernel::Writes::Domain);
    let asking = std::cell::RefCell::new(Interrupting {
        run_id,
        member,
        backlog,
        person,
        steers: signals,
        held: None,
    });
    // What the run's own commands did. It is the only evidence of "the
    // tests passed" the city can observe without being told, and being
    // told is what a mode is supposed to check.
    let ran: std::rc::Rc<std::cell::RefCell<(u32, u32)>> =
        std::rc::Rc::new(std::cell::RefCell::new((0, 0)));
    let ran_by_bench = std::rc::Rc::clone(&ran);
    let driven = {
        let fenced = fenced_by_bench;
        let ran = ran_by_bench;
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
            let Ok(at) = now_ms() else {
                return NextCall::Allowed;
            };
            let left = until.value().saturating_sub(at.value());
            if left == 0 {
                return NextCall::Allowed;
            }
            std::thread::sleep(std::time::Duration::from_millis(left.min(HALT_SLICE_MS)));
        };
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
            // A call that failed may have failed partway through its
            // writes, and its own account of them no longer holds, so the
            // next fence walks the whole domain (runtime-SPEC 8-45).
            let answered = bench.invoke(call, &key, t).inspect_err(|_| {
                let so_far = wrote.replace(kernel::Writes::Nothing);
                wrote.replace(so_far.and(kernel::Writes::Domain));
            })?;
            match answered {
                BenchOutcome::Ran {
                    outcome,
                    fenced: at,
                    wrote: this,
                } => {
                    if let Some(oid) = at {
                        fenced.borrow_mut().push(oid);
                    }
                    let so_far = wrote.replace(kernel::Writes::Nothing);
                    wrote.replace(so_far.and(this));
                    if call.name.as_str() != "exec" {
                        return Ok(outcome);
                    }
                    // Absence of `exit_code` is a failure, not a
                    // success: a command a signal stopped returns no
                    // code at all, and reading that as zero would let
                    // a halted build count as tests that passed.
                    let result = outcome.result.as_map();
                    let failed = match result.get("exit_code").and_then(serde_json::Value::as_i64) {
                        Some(code) => code != 0,
                        None => true,
                    };
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
            // Held for the whole of `wave_pre`: staging, committing and
            // reading back are one act over one index.
            let _one_at_a_time = fence_gate
                .lock()
                .map_err(|_| poison("this city's fence gate"))?;
            // `Nothing` means only reads ran, and the domain is what the
            // fence staged before this rule existed; skipping that wave
            // is the read-only rule's to decide.
            let scope = match wrote.replace(kernel::Writes::Nothing) {
                kernel::Writes::Paths(paths) => {
                    paths.iter().map(|path| path.as_str().to_owned()).collect()
                }
                kernel::Writes::Nothing | kernel::Writes::Domain => fence_scope.clone(),
            };
            let payload = fence_point
                .wave_pre(&scope, t, &of)
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
            wait: &mut wait,
            deltas: watching.is_some().then_some(&mut watched),
        };
        drive(plan, ledger, adapter.as_mut(), &mut hooks, &handoff)
    };
    Ok(Driven {
        outcome: driven,
        adapter,
        fenced: fenced.borrow().clone(),
        ran: *ran.borrow(),
        // Nothing a door answers reaches a person any more: a door
        // answers Allow or Deny. The sweep is the one thing that still
        // raises a question, and it raises it after this returns.
        raised: Vec::new(),
    })
}
