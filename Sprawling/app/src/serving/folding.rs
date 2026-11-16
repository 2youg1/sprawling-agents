// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

use kernel::{AxCode, AxError, EventRecord};

use super::standing::{CoreThread, monotonic_now};
use accounting::person::CorePriority;
use accounting::views::{Published, Views};

pub(crate) struct Folding {
    pub(crate) observer: Box<dyn FnMut(&EventRecord) + Send>,
    pub(crate) machine: Arc<dyn Fn(wire::DoctorAnswer) + Send + Sync>,
    pub(crate) lend: Box<dyn FnOnce(Arc<Mutex<gateway::Custodian>>) + Send>,

    pub(crate) keep_slices: Box<dyn FnOnce(storage::Sessions) + Send>,

    pub(crate) backlog: Backlog,
    pub(crate) thread: std::thread::JoinHandle<()>,
}

#[derive(Clone, Default)]
pub(crate) struct Backlog(Arc<AtomicU64>);

impl Backlog {
    pub(crate) fn records(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }

    fn sent(&self) {
        self.moved(|held| held.saturating_add(1));
    }

    fn broadcast(&self, records: u64) {
        self.moved(|held| held.saturating_sub(records));
    }

    fn moved(&self, step: impl Fn(u64) -> u64) {
        let mut held = self.0.load(Ordering::Relaxed);
        while let Err(now) =
            self.0
                .compare_exchange_weak(held, step(held), Ordering::Relaxed, Ordering::Relaxed)
        {
            held = now;
        }
    }
}

pub(crate) const CUT_WAITS_ABOVE: u64 = 256;

enum Fold {
    Committed(EventRecord),
    Examined(wire::DoctorAnswer),
    Lent(Arc<Mutex<gateway::Custodian>>),
    Slices(storage::Sessions),
}

pub(crate) struct Copies {
    pub(crate) published: Arc<Published>,
    pub(crate) spare: Views,
}

pub(crate) struct Broadcast {
    pub(crate) to_clients: tokio::sync::broadcast::Sender<wire::Committed>,
    pub(crate) head: Arc<wire::LedgerHead>,
}

pub(crate) fn spawn_folding(
    copies: Copies,
    broadcast: Broadcast,
    setting: CorePriority,
    clock: fn() -> Instant,
) -> Result<Folding, AxError> {
    let (committed, arriving) = mpsc::channel::<Fold>();
    let examined = committed.clone();
    let lent = committed.clone();
    let handed = committed.clone();
    let backlog = Backlog::default();
    let behind = backlog.clone();
    let shown = backlog.clone();
    let thread = std::thread::Builder::new()
        .name("sprawling-views".to_owned())
        .spawn(move || {
            let _seat = crate::serving::placement::seat_this_thread(
                "sprawling-views",
                crate::serving::placement::Role::Serial,
            );
            fold_until_closed(copies, &arriving, (&broadcast, &behind), (setting, clock));
        })
        .map_err(|source| {
            AxError::failure(
                AxCode::StorageFatal,
                "start the view fold",
                source.to_string(),
            )
            .with_recovery("check process thread limits")
        })?;
    Ok(Folding {
        observer: Box::new(move |record: &EventRecord| {
            backlog.sent();
            if committed.send(Fold::Committed(record.clone())).is_err() {
                eprintln!(
                    "the view fold has ended; record {} reaches neither the views nor the clients until the server restarts",
                    record.seq().value()
                );
            }
        }),
        machine: Arc::new(move |found: wire::DoctorAnswer| {
            if examined.send(Fold::Examined(found)).is_err() {
                eprintln!(
                    "the view fold has ended; this machine's check is not shown until the server restarts"
                );
            }
        }),
        lend: Box::new(move |vault: Arc<Mutex<gateway::Custodian>>| {
            if lent.send(Fold::Lent(vault)).is_err() {
                eprintln!(
                    "the view fold has ended; a tool server that needs a credential is not reached until the server restarts"
                );
            }
        }),
        keep_slices: Box::new(move |slices: storage::Sessions| {
            if handed.send(Fold::Slices(slices)).is_err() {
                eprintln!(
                    "the view fold has ended; the session slices are not written until the server restarts"
                );
            }
        }),
        backlog: shown,
        thread,
    })
}

fn fold_until_closed(
    copies: Copies,
    arriving: &mpsc::Receiver<Fold>,
    (broadcast, backlog): (&Broadcast, &Backlog),
    (setting, clock): (CorePriority, fn() -> Instant),
) {
    let mut core = CoreThread::raise("sprawling-views", setting, monotonic_now());
    let mut cadence = Cadence::default();
    let mut slices: Option<storage::Sessions> = None;
    let Copies {
        published,
        mut spare,
    } = copies;
    while let Ok(first) = arriving.recv() {
        let woke = monotonic_now();
        let batch: Vec<Fold> = std::iter::once(first).chain(arriving.try_iter()).collect();
        let fold_started = clock();
        let verdict = fold_batch(&mut spare, &batch);
        let fold_cost = clock().saturating_duration_since(fold_started);
        let retired = published.replace(Arc::new(spare));
        for fold in &batch {
            if let Fold::Committed(record) = fold {
                send_committed(broadcast, record);
            }
        }
        spare = reclaim(retired);
        fold_batch(&mut spare, &batch);
        cadence.folded(fold_cost, last_committed(&batch), verdict);
        backlog.broadcast(file_slices(&mut slices, batch));
        if let Some(record) = cadence.due(backlog.records()) {
            cut_views_snapshot(&spare, &record, clock, &mut cadence);
        }
        core.record_turn_lowering_when_busy(woke, monotonic_now());
    }
    if let Some(record) = cadence.uncut() {
        cut_views_snapshot(&spare, &record, clock, &mut cadence);
    }
}

const CUT_SHARE_INVERSE: u32 = 10;

#[derive(Default)]
struct Cadence {
    folded_since_cut: Duration,
    last_cut_cost: Option<Duration>,

    uncut: Option<EventRecord>,
    cutting: Cutting,
}

#[derive(Default)]
enum Cutting {
    #[default]
    Open,
    StoppedByRefusal,
}

enum Folded {
    Clean,
    Refused,
}

impl Cadence {
    fn folded(&mut self, cost: Duration, last: Option<&EventRecord>, verdict: Folded) {
        self.folded_since_cut = self.folded_since_cut.saturating_add(cost);
        if let Some(record) = last {
            self.uncut = Some(record.clone());
        }
        match verdict {
            Folded::Clean => {}
            Folded::Refused => self.cutting = Cutting::StoppedByRefusal,
        }
    }

    fn due(&self, backlog: u64) -> Option<EventRecord> {
        let spent_enough = self
            .last_cut_cost
            .is_none_or(|cut| self.folded_since_cut >= cut.saturating_mul(CUT_SHARE_INVERSE));
        let caught_up = backlog <= CUT_WAITS_ABOVE;
        self.uncut().filter(|_| spent_enough && caught_up)
    }

    fn uncut(&self) -> Option<EventRecord> {
        match self.cutting {
            Cutting::Open => self.uncut.clone(),
            Cutting::StoppedByRefusal => None,
        }
    }

    fn cut(&mut self, cost: Duration) {
        self.last_cut_cost = Some(cost);
        self.folded_since_cut = Duration::ZERO;
        self.uncut = None;
    }
}

fn cut_views_snapshot(
    views: &Views,
    record: &EventRecord,
    clock: fn() -> Instant,
    cadence: &mut Cadence,
) {
    let started = clock();
    if let Err(fault) = views.cut_snapshot_at(record) {
        eprintln!(
            "the views snapshot was not cut at record {}: {fault}; serving goes on, and the next start folds a longer tail",
            record.seq().value()
        );
    }
    cadence.cut(clock().saturating_duration_since(started));
}

fn last_committed(batch: &[Fold]) -> Option<&EventRecord> {
    batch.iter().rev().find_map(|fold| match fold {
        Fold::Committed(record) => Some(record),
        Fold::Examined(_) | Fold::Lent(_) | Fold::Slices(_) => None,
    })
}

fn file_slices(slices: &mut Option<storage::Sessions>, batch: Vec<Fold>) -> u64 {
    let mut committed: u64 = 0;
    for fold in batch {
        match fold {
            Fold::Committed(record) => {
                committed = committed.saturating_add(1);
                if let Some(Err(refused)) = slices.as_mut().map(|kept| kept.absorb(&record)) {
                    eprintln!("a session slice was refused and skipped: {refused}");
                }
            }
            Fold::Slices(handed) => *slices = Some(handed),
            Fold::Examined(_) | Fold::Lent(_) => {}
        }
    }
    committed
}

fn send_committed(broadcast: &Broadcast, record: &EventRecord) {
    broadcast.head.advance(record.seq());
    match wire::Committed::new(record.clone()) {
        Ok(committed) => drop(broadcast.to_clients.send(committed)),

        Err(unframed) => {
            eprintln!("a committed record has no frame and reaches no client: {unframed}");
        }
    }
}

fn fold_batch(views: &mut Views, batch: &[Fold]) -> Folded {
    batch
        .iter()
        .fold(Folded::Clean, |verdict, fold| match fold {
            Fold::Committed(record) => match views.apply(record) {
                Ok(()) => verdict,
                Err(err) => {
                    eprintln!("view fold refused {}: {err}", record.seq().value());
                    Folded::Refused
                }
            },
            Fold::Examined(found) => {
                views.found_on_this_machine(found.clone());
                verdict
            }
            Fold::Lent(vault) => {
                views.lend_the_vault(Arc::clone(vault));
                verdict
            }
            Fold::Slices(_) => verdict,
        })
}

fn reclaim(mut retired: Arc<Views>) -> Views {
    loop {
        match Arc::try_unwrap(retired) {
            Ok(views) => return views,
            Err(held) => {
                retired = held;
                std::thread::yield_now();
            }
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
