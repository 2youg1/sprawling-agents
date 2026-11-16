// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::sync::Arc;
use std::sync::mpsc;

use kernel::{AxCode, AxError, EventRecord, Payload, RunId};

use crate::serving::folding::{Broadcast, Copies, Folding, spawn_folding};
use crate::serving::output_ring::OutputRing;
use accounting::person::CorePriority;
use accounting::views::{Published, Views};
use accounting::worker::health::Health;
use accounting::worker::{CommandDesk, RunWorker, Serving, Standing};

pub(super) struct Opening {
    pub(super) city_root: std::path::PathBuf,
    pub(super) vault: gateway::Custodian,

    pub(super) notice: Option<Payload>,
    pub(super) log: runtime::diagnostics::Diagnostics,

    pub(super) held: (storage::JsonlLedger, storage::OpenReport, Standing),

    pub(super) audit_log: runtime::diagnostics::Diagnostics,

    pub(super) began: std::time::Instant,

    pub(super) core: CorePriority,

    pub(super) halt: storage::ChainHalt,
}

pub(super) struct Outward {
    pub(super) desk: Arc<CommandDesk>,
    pub(super) views: Arc<Published>,

    pub(super) spare: Views,
    pub(super) to_clients: tokio::sync::broadcast::Sender<wire::Committed>,
    pub(super) to_watchers: tokio::sync::broadcast::Sender<wire::Delta>,
    pub(super) head: Arc<wire::LedgerHead>,
    pub(super) to_readers: tokio::sync::broadcast::Sender<wire::LiveOutput>,

    pub(super) kept: Arc<OutputRing>,
}

pub(super) struct Started {
    pub(super) thread: std::thread::JoinHandle<()>,
    pub(super) vault: Arc<std::sync::Mutex<gateway::Custodian>>,

    pub(super) health: Health,

    pub(super) backlog: crate::serving::folding::Backlog,

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

    let warm_ups: Vec<gateway::WarmUp> = held.2.book.warm_ups().collect();

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

        setting,
        crate::serving::standing::monotonic_now,
    )?;

    let audit_proof = halt.clone();
    let worker_thread = std::thread::Builder::new()
        .name("sprawling-runs".to_owned())
        .spawn(move || {

            let _seat = crate::serving::placement::seat_this_thread(
                "sprawling-runs",
                crate::serving::placement::Role::Serial,
            );
            let mut worker = match RunWorker::holding(&worker_root, log, super::hands(vault), held)
            {
                Ok(worker) => worker,
                Err(err) => {
                    drop(ready_tx.send(Err(err)));
                    return;
                }
            };

            if let Some(notice) = worker.opening().notice() {
                eprintln!("{notice}");
            }

            let interrupt_desk = Arc::clone(&worker_desk);
            let keeping = Arc::clone(&kept);
            worker.serve(Serving {
                deltas: std::sync::Arc::new(move |delta: wire::Delta| {

                    drop(to_watchers.send(delta));
                }),
                outputs: std::sync::Arc::new(move |piece: wire::LiveOutput| {
                    keeping.keep(&piece);
                    drop(to_readers.send(piece));
                }),
                machine,
                interrupts: Arc::new(move |run: RunId| interrupt_desk.interrupt_for(run)),
            });

            match worker.hand_off_session_slices() {
                Some(slices) => keep_slices(slices),
                None => drop(keep_slices),
            }

            let audits =
                match super::skill_audit::start(worker_root.clone(), worker.relay(), audit_proof) {
                    Ok(sender) => Some(sender),
                    Err(err) => {
                        eprintln!("skill audits unavailable: {err}");
                        None
                    }
                };
            let mut folding = observer;
            worker.observe(Box::new(move |record: &EventRecord| {
                if matches!(
                    record.kind(),
                    kernel::EventKind::SkillShelved | kernel::EventKind::RunStarted
                ) && let Some(sender) = &audits
                    && let Err(err) = sender.send(record.clone())
                {
                    eprintln!("skill audit notification failed: {err}");
                }
                kept.settle(record);
                folding(record);
            }));

            worker.open_for_service(vault_notice);

            if let Err(err) = super::chain_watch::audit_in_background(
                worker.chain_under_audit(halt),
                audit_log,
                began,
            ) {
                drop(ready_tx.send(Err(err)));
                return;
            }
            drop(ready_tx.send(Ok((worker.vault_handle(), worker.health(), worker.relay()))));
            worker.take_back_the_lost();
            accounting::worker::attend::attend(&mut worker, &worker_desk);

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

fn start_warm_ups(warm_ups: Vec<gateway::WarmUp>) {
    for warm_up in warm_ups {
        drop(
            std::thread::Builder::new()
                .name("sprawling-warm-up".to_owned())
                .spawn(move || warm_up.open()),
        );
    }
}
