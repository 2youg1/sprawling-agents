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
//! The thread runs `attend::attend`, the loop, until the desk closes;
//! this file starts it, hands out what it opened, and once it reports
//! ready starts one warm-up per attached endpoint.

use std::sync::Arc;
use std::sync::mpsc;

use kernel::{AxCode, AxError, EventRecord, Payload, RunId};

use crate::serving::folding::{Broadcast, Copies, Folding, spawn_folding};
use crate::serving::output_ring::OutputRing;
use accounting::person::CorePriority;
use accounting::views::{Published, Views};
use accounting::worker::health::Health;
use accounting::worker::{CommandDesk, RunWorker, Serving, Standing};

/// What a worker is opened with: where the city is, whose keys it may
/// redeem, what the vault turned out to be, where its diagnostics go,
/// and what the history already says.
///
/// Values that always travel together and are never chosen
/// independently - `listen` settles them all before it has a thread to
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
    pub(super) held: (storage::JsonlLedger, storage::OpenReport, Standing),
    /// The chain audit's own voice: the same sink and the same floor as
    /// `log`, held apart because the audit thread never touches the
    /// writer (`crates/sprawling/spec/Assembly/Listening.lean` §8-90).
    pub(super) audit_log: runtime::diagnostics::Diagnostics,
    /// When opening the city began, which the proof measures the moment
    /// commands are taken from (`crates/sprawling/spec/Assembly/Listening.lean` §8-122).
    pub(super) began: std::time::Instant,
    /// The person's `[core] priority`, the reading the socket's workers
    /// already stand on.
    pub(super) core: CorePriority,
    /// The halt the writer waits on until the history is proved, made
    /// before the views so they watch the same verdict
    /// (`crates/sprawling/spec/Assembly.lean` §8-134).
    pub(super) halt: storage::ChainHalt,
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
    /// (`crates/sprawling/spec/Assembly/Listening.lean` §8-99).
    pub(super) spare: Views,
    pub(super) to_clients: tokio::sync::broadcast::Sender<wire::Committed>,
    pub(super) to_watchers: tokio::sync::broadcast::Sender<wire::Delta>,
    pub(super) head: Arc<wire::LedgerHead>,
    pub(super) to_readers: tokio::sync::broadcast::Sender<wire::LiveOutput>,
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
    /// The accounting queue's counts, for the monitor (`crates/sprawling/spec/Accounting/Worker.lean` §8-98).
    pub(super) health: Health,
    /// The view fold's backlog, for the monitor (`crates/sprawling/spec/Serving.lean` §8-123).
    pub(super) backlog: crate::serving::folding::Backlog,
    /// The crossing the remote door writes its lines through
    /// (`crates/sprawling/spec/Outside/Conduit.lean` §8-139).
    pub(super) relay: accounting::worker::Relay,
}

pub(super) fn spawn_worker(opening: Opening, outward: Outward) -> Result<Started, AxError> {
    type Opened = Result<
        (
            Arc<std::sync::Mutex<gateway::Custodian>>,
            Health,
            accounting::worker::Relay,
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
        began,
        core: setting,
        halt,
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
    // Taken before the worker takes the book: each shares its endpoint's
    // client slot with the book the worker calls through.
    let warm_ups: Vec<gateway::WarmUp> = held.2.book.warm_ups().collect();
    // The views are folded beside the writer rather than on it, so a
    // reader holding them never delays the next record
    // (`crates/sprawling/spec/Assembly/Listening.lean` §8-99).
    let Folding {
        observer,
        machine,
        lend,
        keep_slices,
        backlog,
        thread: fold_thread,
    } = spawn_folding(
        Copies {
            published: views,
            spare,
        },
        Broadcast { to_clients, head },
        // The views thread stands above the commands the city
        // dispatches (`crates/sprawling/spec/Serving/Standing.lean` §8-93).
        setting,
        crate::serving::standing::monotonic_now,
    )?;
    // The one sanctioned thread besides the runtime's own. The ledger was
    // opened, its writer lock taken, before the history was folded; it
    // moves into this thread and never leaves: a city has one writer.
    let worker_thread = std::thread::Builder::new()
        .name("sprawling-runs".to_owned())
        .spawn(move || {
            // Held to the closure's end: the seat goes back as the
            // thread exits (`crates/sprawling/spec/Serving/Placement.lean`).
            let _seat = crate::serving::placement::seat_this_thread(
                "sprawling-runs",
                crate::serving::placement::Role::Serial,
            );
            let mut worker = match RunWorker::holding(&worker_root, log, super::hands(vault), held)
            {
                Ok(mut worker) => {
                    // Before the banner, so a torn tail is the first
                    // thing the person running the city reads.
                    if let Some(notice) = worker.opening().notice() {
                        eprintln!("{notice}");
                    }
                    worker.open_for_service(vault_notice);
                    // Detached: the proof holds no part of the writer,
                    // and what it finds reaches the writer through the
                    // halt it attached; until then the writer takes no
                    // line (`crates/sprawling/spec/Assembly/Listening.lean` §8-90).
                    if let Err(err) = super::chain_watch::audit_in_background(
                        worker.chain_under_audit(halt),
                        audit_log,
                        began,
                    ) {
                        drop(ready_tx.send(Err(err)));
                        return;
                    }
                    drop(ready_tx.send(Ok((
                        worker.vault_handle(),
                        worker.health(),
                        worker.relay(),
                    ))));
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
                deltas: std::sync::Arc::new(move |delta: wire::Delta| {
                    // No subscribers is not a failure: a city with no
                    // browser open is a city doing its work.
                    drop(to_watchers.send(delta));
                }),
                outputs: std::sync::Arc::new(move |piece: wire::LiveOutput| {
                    keeping.keep(&piece);
                    drop(to_readers.send(piece));
                }),
                machine,
                interrupts: Arc::new(move |run: RunId| interrupt_desk.interrupt_for(run)),
            });
            // The session slices go to the view thread before the
            // observer is attached, so every record from here on is filed
            // there and none on the accounting thread (`crates/sprawling/spec/Serving.lean`
            // §8-123).
            // The sender is consumed either way: the fold thread ends only
            // once every sender is gone, and this thread joins it.
            match worker.hand_off_session_slices() {
                Some(slices) => keep_slices(slices),
                None => drop(keep_slices),
            }
            // The tail is emptied on this thread, right after the result
            // is written, so no piece of that call can arrive after it.
            let mut folding = observer;
            worker.observe(Box::new(move |record: &EventRecord| {
                kept.settle(record);
                folding(record);
            }));
            accounting::worker::attend::attend(&mut worker, &worker_desk);
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
    let (vault, health, relay) = match ready_rx.recv() {
        // Lent rather than opened a second time: "a credential is
        // redeemed at the last moment, through one door" stops being
        // true the moment there are two handles on the same secrets.
        Ok(Ok((vault, health, relay))) => {
            lend(Arc::clone(&vault));
            start_warm_ups(warm_ups);
            (vault, health, relay)
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
        backlog,
        relay,
    })
}

/// Starts one detached `sprawling-warm-up` thread per attached endpoint,
/// once the city is open, each ending with its one request
/// (`crates/gateway/spec/Endpoint/Transport.lean` D26). Nothing waits on
/// them, and a warm-up that cannot start leaves its endpoint's first
/// call to pay for the connection, as it would with no warm-up at all.
fn start_warm_ups(warm_ups: Vec<gateway::WarmUp>) {
    for warm_up in warm_ups {
        drop(
            std::thread::Builder::new()
                .name("sprawling-warm-up".to_owned())
                .spawn(move || warm_up.open()),
        );
    }
}
