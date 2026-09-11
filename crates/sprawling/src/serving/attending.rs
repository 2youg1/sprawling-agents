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

use std::sync::Arc;
use std::sync::mpsc;

use kernel::{AxCode, AxError, EventRecord, RunId};

use super::desk::{CommandDesk, DeskWait, SCHEDULE_TICK, SCHEDULE_TICK_MS};
use super::serve::Opening;
use crate::assembly::{LOOK_AGAIN, RunWorker, now_ms};
use crate::views::Views;

/// Where a worker's work goes, and where it comes from.
///
/// The desk is the mouth and the other three are the ears: a record
/// reaches the fold every query is answered from, the clients watching
/// the city, and - while a model is still speaking - the watchers of one
/// run's increments.
pub(super) struct Outward {
    pub(super) desk: Arc<CommandDesk>,
    pub(super) views: Arc<std::sync::Mutex<Views>>,
    pub(super) to_clients: tokio::sync::broadcast::Sender<EventRecord>,
    pub(super) to_watchers: tokio::sync::broadcast::Sender<channels::Delta>,
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
}

pub(super) fn spawn_worker(opening: Opening, outward: Outward) -> Result<Started, AxError> {
    type Opened = Result<Arc<std::sync::Mutex<gateway::Custodian>>, AxError>;
    let (ready_tx, ready_rx) = mpsc::sync_channel::<Opened>(0);
    let Opening {
        city_root: worker_root,
        vault,
        notice: vault_notice,
        log,
    } = opening;
    let Outward {
        desk: worker_desk,
        views,
        to_clients,
        to_watchers,
    } = outward;
    // The one sanctioned thread besides the runtime's own. The ledger is
    // opened *inside* it and never leaves: a city has one writer, and the
    // type never has to cross a thread boundary to prove it.
    let worker_thread = std::thread::Builder::new()
        .name("sprawling-runs".to_owned())
        .spawn(move || {
            let mut worker = match RunWorker::new(&worker_root, vault, log) {
                Ok(mut worker) => {
                    worker.open_for_service(vault_notice);
                    let _ = ready_tx.send(Ok(worker.vault_handle()));
                    worker
                }
                Err(err) => {
                    let _ = ready_tx.send(Err(err));
                    return;
                }
            };
            // Somebody is watching, so runs ask their provider to
            // stream. A worker without this call never installs a sink,
            // and its runs take the blocking path unchanged.
            worker.watch(std::sync::Arc::new(move |delta: channels::Delta| {
                // No subscribers is not a failure: a city with no browser
                // open is a city doing its work.
                let _ = to_watchers.send(delta);
            }));
            worker.observe(Box::new(move |record: &EventRecord| {
                if let Ok(mut views) = views.lock() {
                    // A record the views refuse to fold is reported and
                    // skipped: the ledger already has it, and a view that
                    // crashed the writer would make history hostage to a
                    // projection.
                    if let Err(err) = views.apply(record) {
                        eprintln!("view fold refused {}: {err}", record.seq().value());
                    }
                }
                // A send with no subscribers is not a failure: a city with
                // no browser open is a city doing its work.
                let _ = to_clients.send(record.clone());
            }));
            // A run in progress asks the same desk what arrived, so a
            // Cancel does not have to wait for the run it cancels.
            let interrupt_desk = Arc::clone(&worker_desk);
            worker.attach_interrupts(Arc::new(move |run: RunId| {
                interrupt_desk.interrupt_for(run)
            }));
            // When the schedule was last read against. Kept by the
            // loop rather than measured from it, because a city with
            // lanes driving comes back here every millisecond and one
            // that opened its schedule file that often would spend its
            // time opening a file (sprawling-SPEC.md 8-46-2).
            let mut read_schedule_at = kernel::TimeMs::new(0);
            loop {
                // The first two of the three mouths: every relay request
                // already waiting, then at most one run home. The
                // crossing is served first, because a run that has
                // already been paid for must not queue behind one that
                // has not started.
                if let Err(err) = worker.serve_flight(LOOK_AGAIN) {
                    eprintln!("a run could not be landed: {err}");
                }
                // The third mouth. A city with lanes driving looks at
                // the desk in short steps, because a lane makes progress
                // only while this thread is serving the crossing.
                let patience = if worker.driving() {
                    LOOK_AGAIN
                } else {
                    SCHEDULE_TICK
                };
                match worker_desk.wait(patience) {
                    // `carrying` holds this command's key in flight for
                    // the length of the arm, so a frame that repeats it
                    // while the work is going adds no second run.
                    DeskWait::Command(posted, carrying) => {
                        worker.serve_one(*posted);
                        drop(carrying);
                    }
                    // The refusal is written inside `tick`; a schedule
                    // that cannot be read must not stop the city from
                    // answering the person.
                    DeskWait::Idle => {
                        if let Ok(now) = now_ms()
                            && now.value().saturating_sub(read_schedule_at.value())
                                >= SCHEDULE_TICK_MS
                        {
                            read_schedule_at = now;
                            let _ = worker.tick(now);
                        }
                    }
                    DeskWait::Close => {
                        // The lanes are waited for rather than
                        // abandoned: a lane left blocked on an append
                        // loses lines this city had already told it
                        // were durable.
                        if let Err(err) = worker.land_the_rest() {
                            eprintln!("a run could not be landed as the city closed: {err}");
                        }
                        if let Err(err) = worker.close_city() {
                            eprintln!("the city could not write its handoff: {err}");
                        }
                        break;
                    }
                    DeskWait::Gone => break,
                }
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
    let vault = match ready_rx.recv() {
        Ok(Ok(vault)) => vault,
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
    })
}
