// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The one writer thread: what it opens, and the three mouths it
//! serves.
//!
//! **The ledger is opened inside this thread and never leaves**, so the
//! type never has to cross a thread boundary to prove that a city has
//! one writer (ARCHITECTURE section 10). Everything a socket does
//! reaches it as a `Command` on a desk, one at a time.
//!
//! The loop looks at three things in this order: every relay request a
//! lane is blocked on, then at most one run home from a lane, then the
//! desk (sprawling-SPEC.md 8-42-4). The crossing is first because a run
//! that has already been paid for must not queue behind one that has
//! not started.
//!
//! Which properties the loop must hold - every message served in
//! finitely many steps, append order equal to seq order, no wake
//! without work - is decided by the Lean model
//! `adversary/design/Attending.lean`; this loop polls and so does not
//! yet hold the third (sprawling-SPEC.md 8-42-4).

use std::sync::Arc;
use std::sync::mpsc;
use std::time::Duration;

use kernel::{AxCode, AxError, EventRecord, Payload, RunId};

use super::desk::{CommandDesk, DeskWait, SCHEDULE_TICK_MS};
use super::relay::Patience;
use super::{RunWorker, Serving};
use crate::serving::folding::{Broadcast, Copies, Folding, spawn_folding};
use crate::serving::output_ring::OutputRing;
use crate::serving::setting_telling_a_refusal;
use crate::views::{Published, Views};

/// What a worker is opened with: where the city is, whose keys it may
/// redeem, what the vault turned out to be, where its diagnostics go,
/// and what the history already says.
///
/// Five values that always travel together and are never chosen
/// independently - `listen` settles all five before it has a thread to
/// hand them to - so they travel as one, as `Reporter` does.
pub(super) struct Opening {
    pub(super) city_root: std::path::PathBuf,
    pub(super) vault: gateway::Custodian,
    /// What the vault probe found, on its way to the ledger as a
    /// disclosure. Consumed by the first `open_for_service`.
    pub(super) notice: Option<Payload>,
    pub(super) log: runtime::diagnostics::Diagnostics,
    /// The opened ledger and what its history already says, folded on the
    /// serve thread under its writer lock in the same pass as the views,
    /// so the worker neither opens nor reads it again.
    pub(super) held: (memory::JsonlLedger, memory::OpenReport, super::Standing),
    /// The chain audit's own voice: the same sink and the same floor as
    /// `log`, held apart because the audit thread never touches the
    /// writer (sprawling-SPEC.md 8-90).
    pub(super) audit_log: runtime::diagnostics::Diagnostics,
}

/// Where a worker's work goes, and where it comes from.
///
/// The desk is the mouth and the other three are the ears: a record
/// reaches the fold every query is answered from, the clients watching
/// the city, and - while a model is still speaking - the watchers of one
/// run's increments.
pub(super) struct Outward {
    pub(super) desk: Arc<CommandDesk>,
    pub(super) views: Arc<Published>,
    /// The unpublished twin of `views`, folded over the same records
    /// (sprawling-SPEC.md 8-93).
    pub(super) spare: Views,
    pub(super) to_clients: tokio::sync::broadcast::Sender<channels::Committed>,
    pub(super) to_watchers: tokio::sync::broadcast::Sender<channels::Delta>,
    pub(super) head: Arc<channels::LedgerHead>,
    pub(super) to_readers: tokio::sync::broadcast::Sender<channels::LiveOutput>,
    /// What running commands already wrote, for a page opening late.
    pub(super) kept: Arc<OutputRing>,
}

/// The thread, and the one thing it opens that something else needs.
///
/// The vault is opened inside the worker like the ledger is, and unlike
/// the ledger it is shared: the transcription route resolves a
/// credential on a socket task, and a second vault handle would be a
/// second door onto the same secrets.
pub(super) struct Started {
    pub(super) thread: std::thread::JoinHandle<()>,
    pub(super) vault: Arc<std::sync::Mutex<gateway::Custodian>>,
    /// The accounting queue's counts, for the monitor (sprawling-SPEC.md 8-98).
    pub(super) health: crate::monitor::health::Health,
}

pub(super) fn spawn_worker(opening: Opening, outward: Outward) -> Result<Started, AxError> {
    type Opened = Result<
        (
            Arc<std::sync::Mutex<gateway::Custodian>>,
            crate::monitor::health::Health,
        ),
        AxError,
    >;
    let (ready_tx, ready_rx) = mpsc::sync_channel::<Opened>(0);
    let Opening {
        city_root: worker_root,
        vault,
        notice: vault_notice,
        log,
        held,
        audit_log,
    } = opening;
    let Outward {
        desk: worker_desk,
        views,
        spare,
        to_clients,
        to_watchers,
        head,
        to_readers,
        kept,
    } = outward;
    // The views thread stands above the commands the city dispatches
    // (sprawling-SPEC.md 8-93).
    let setting = setting_telling_a_refusal();
    // The views are folded beside the writer rather than on it, so a
    // reader holding them never delays the next record
    // (sprawling-SPEC.md 8-93).
    let Folding {
        observer,
        machine,
        lend,
        thread: fold_thread,
    } = spawn_folding(
        Copies {
            published: views,
            spare,
        },
        Broadcast { to_clients, head },
        setting,
        crate::serving::standing::monotonic_now,
    )?;
    // The one sanctioned thread besides the runtime's own. The ledger was
    // opened, its writer lock taken, before the history was folded; it
    // moves into this thread and never leaves: a city has one writer.
    let worker_thread = std::thread::Builder::new()
        .name("sprawling-runs".to_owned())
        .spawn(move || {
            let mut worker = match RunWorker::holding(&worker_root, vault, log, held) {
                Ok(mut worker) => {
                    // Before the banner, so a torn tail is the first
                    // thing the person running the city reads.
                    if let Some(notice) = worker.opening().notice() {
                        eprintln!("{notice}");
                    }
                    worker.open_for_service(vault_notice);
                    // Detached: the audit holds no part of the writer,
                    // and what it finds reaches the writer through the
                    // halt it attached (sprawling-SPEC.md 8-90).
                    if let Err(err) = worker.audit_chain_in_background(audit_log) {
                        drop(ready_tx.send(Err(err)));
                        return;
                    }
                    drop(ready_tx.send(Ok((worker.vault_handle(), worker.flight.gate.health()))));
                    worker
                }
                Err(err) => {
                    drop(ready_tx.send(Err(err)));
                    return;
                }
            };
            // Somebody is watching, so runs ask their provider to
            // stream, the machine's look has a place to land, and a run
            // in progress can be interrupted. One call, because a
            // worker that streamed to a page unable to interrupt it
            // would be the state this type exists to make unsayable.
            let interrupt_desk = Arc::clone(&worker_desk);
            let keeping = Arc::clone(&kept);
            worker.serve(Serving {
                deltas: std::sync::Arc::new(move |delta: channels::Delta| {
                    // No subscribers is not a failure: a city with no
                    // browser open is a city doing its work.
                    drop(to_watchers.send(delta));
                }),
                outputs: std::sync::Arc::new(move |piece: channels::LiveOutput| {
                    keeping.keep(&piece);
                    drop(to_readers.send(piece));
                }),
                machine,
                interrupts: Arc::new(move |run: RunId| interrupt_desk.interrupt_for(run)),
            });
            // The tail is emptied on this thread, right after the result
            // is written, so no piece of that call can arrive after it.
            let mut folding = observer;
            worker.observe(Box::new(move |record: &EventRecord| {
                kept.settle(record);
                folding(record);
            }));
            attend(&mut worker, &worker_desk);
            // Dropping the worker drops the observer, which closes the
            // fold's channel; what is still in it is folded and
            // broadcast before the thread ends.
            drop(worker);
            if fold_thread.join().is_err() {
                eprintln!(
                    "the view fold ended in a panic; restart the server to rebuild the views"
                );
            }
        })
        .map_err(|source| {
            AxError::failure(
                AxCode::StorageFatal,
                "start the run worker",
                source.to_string(),
            )
            .with_recovery("check process thread limits")
        })?;
    let (vault, health) = match ready_rx.recv() {
        // Lent rather than opened a second time: "a credential is
        // redeemed at the last moment, through one door" stops being
        // true the moment there are two handles on the same secrets.
        Ok(Ok((vault, health))) => {
            lend(Arc::clone(&vault));
            (vault, health)
        }
        Ok(Err(err)) => return Err(err),
        Err(_) => {
            return Err(AxError::failure(
                AxCode::StorageFatal,
                "start the run worker",
                "the worker ended before reporting",
            )
            .with_recovery("check the city directory and rerun"));
        }
    };
    Ok(Started {
        thread: worker_thread,
        vault,
        health,
    })
}

/// The accounting thread's loop: every relay request the one queue
/// holds, then at most one run home, then the desk, in that order
/// (sprawling-SPEC.md 8-42-4), until the desk closes.
///
/// The thread sleeps on the one queue and on nothing else, so a relay
/// request, a run home, a posted command and a close each wake it the
/// moment they are queued; an idle city wakes only at the schedule's
/// deadline (`adversary/design/Attending.lean`).
///
/// A function of its own rather than the body of the thread's closure,
/// because the instruments that time a relay round trip and the gap two
/// dispatches leave in a run drive this loop and not a copy of it
/// (sprawling-SPEC.md 8-84): a copy that waited differently would be
/// measured instead of the city.
pub(crate) fn attend(worker: &mut RunWorker, desk: &CommandDesk) {
    desk.ring_through(worker.bell());
    // When the schedule was last read against. Kept by the loop rather
    // than measured from it, because a city with lanes driving comes
    // back here on every relay request, and one that opened its
    // schedule file that often would spend its time opening a file
    // (sprawling-SPEC.md 8-46-2).
    let mut read_schedule_at = kernel::TimeMs::new(0);
    // The desk is read before the first wait, because what was posted
    // before this thread attended rang nobody.
    let mut patience = Patience::Now;
    loop {
        // The first two of the three mouths: every relay request
        // queued, then at most one run home. The crossing is served
        // first, because a run that has already been paid for must not
        // queue behind one that has not started.
        if let Err(err) = worker.serve_flight(patience) {
            eprintln!("a run could not be landed: {err}");
        }
        // The third mouth, which also decides how long the next look
        // may sleep.
        patience = match desk.next(|run| worker.drives(run)) {
            // `carrying` holds this command's key in flight for the
            // length of the arm, so a frame that repeats it while the
            // work is going adds no second run. The desk may hold more,
            // so the next look does not sleep.
            DeskWait::Command(posted, carrying) => {
                worker.serve_one(*posted);
                drop(carrying);
                Patience::Now
            }
            // The refusal is written inside `tick`; a schedule that
            // cannot be read must not stop the city from answering the
            // person.
            DeskWait::Idle => {
                let since = match worker.clock.now() {
                    Ok(now) => {
                        let since = now.value().saturating_sub(read_schedule_at.value());
                        if since >= SCHEDULE_TICK_MS {
                            read_schedule_at = now;
                            drop(worker.tick(now));
                            0
                        } else {
                            since
                        }
                    }
                    Err(_) => 0,
                };
                // A kept keep-warm door that falls due before the next
                // schedule read wakes the loop for itself
                // (sprawling-SPEC.md 8-93); none is kept by default.
                let until_warm = match (worker.warm_due(), worker.clock.now()) {
                    (Some(_), Ok(now)) => {
                        worker.renew_warm(now);
                        worker
                            .warm_due()
                            .map_or(u64::MAX, |due| due.saturating_sub(now.value()))
                    }
                    (None, _) | (Some(_), Err(_)) => u64::MAX,
                };
                Patience::For(Duration::from_millis(
                    SCHEDULE_TICK_MS.saturating_sub(since).min(until_warm),
                ))
            }
            DeskWait::Close(why) => {
                // The lanes are waited for rather than abandoned: a lane
                // left blocked on an append loses lines this city had
                // already told it were durable.
                if let Err(err) = worker.land_the_rest() {
                    eprintln!("a run could not be landed as the city closed: {err}");
                }
                if let Err(err) = worker.close_city(why) {
                    eprintln!("the city could not write its handoff: {err}");
                }
                break;
            }
            DeskWait::Gone => break,
        };
    }
}
