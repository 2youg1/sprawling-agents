// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The thread the views are folded on and published from, so neither
//! the writer nor the fold waits for a reader (sprawling-SPEC.md 8-99).

use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

use kernel::{AxCode, AxError, EventRecord};

use super::standing::{CoreThread, monotonic_now};
use accounting::person::CorePriority;
use accounting::views::{Published, Views};

/// The places the writer thread hands the views what it wrote, and the
/// thread that folds it.
pub(crate) struct Folding {
    pub(crate) observer: Box<dyn FnMut(&EventRecord) + Send>,
    pub(crate) machine: Arc<dyn Fn(wire::DoctorAnswer) + Send + Sync>,
    pub(crate) lend: Box<dyn FnOnce(Arc<Mutex<gateway::Custodian>>) + Send>,
    pub(crate) thread: std::thread::JoinHandle<()>,
}

/// What the writer thread hands the view fold, in the order it wrote it.
enum Fold {
    Committed(EventRecord),
    Examined(wire::DoctorAnswer),
    Lent(Arc<Mutex<gateway::Custodian>>),
}

/// The two copies of the views the fold alternates between: the one
/// readers are handed, and its unpublished twin, which must have folded
/// the same records (sprawling-SPEC.md 8-99).
pub(crate) struct Copies {
    pub(crate) published: Arc<Published>,
    pub(crate) spare: Views,
}

/// Where a folded record goes next: the clients watching the city, and
/// the head every welcome names, moved before the record is sent so a
/// session that reads the head has already subscribed to what follows it.
pub(crate) struct Broadcast {
    pub(crate) to_clients: tokio::sync::broadcast::Sender<wire::Committed>,
    pub(crate) head: Arc<wire::LedgerHead>,
}

/// Starts the view fold over both copies.
///
/// Every half only queues: the writer's cost per record is one channel
/// send, whatever a reader is doing with the views.
///
/// # Errors
/// `StorageFatal` when the thread cannot be started.
pub(crate) fn spawn_folding(
    copies: Copies,
    broadcast: Broadcast,
    setting: CorePriority,
    clock: fn() -> Instant,
) -> Result<Folding, AxError> {
    let (committed, arriving) = mpsc::channel::<Fold>();
    let examined = committed.clone();
    let lent = committed.clone();
    let thread = std::thread::Builder::new()
        .name("sprawling-views".to_owned())
        .spawn(move || fold_until_closed(copies, &arriving, &broadcast, (setting, clock)))
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
        thread,
    })
}

/// Raises this thread above the commands the city dispatches, then
/// folds every arrival already queued into the spare copy, publishes it,
/// broadcasts the records, and folds the same batch into the copy it
/// replaced, until every sender is gone; the thread is lowered if it
/// keeps a core busy (sprawling-SPEC.md 8-93). A views snapshot is cut
/// from the spare copy when the [`Cadence`] says one is due, and once
/// more when the channel closes (sprawling-SPEC.md 8-91).
///
/// The broadcast follows the publication so a client that queries on
/// hearing a record finds it already folded.
fn fold_until_closed(
    copies: Copies,
    arriving: &mpsc::Receiver<Fold>,
    broadcast: &Broadcast,
    (setting, clock): (CorePriority, fn() -> Instant),
) {
    let mut core = CoreThread::raise("sprawling-views", setting, monotonic_now());
    let mut cadence = Cadence::default();
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
        if let Some(record) = cadence.due() {
            cut_views_snapshot(&spare, &record, clock, &mut cadence);
        }
        core.record_turn_lowering_when_busy(woke, monotonic_now());
    }
    if let Some(record) = cadence.uncut() {
        cut_views_snapshot(&spare, &record, clock, &mut cadence);
    }
}

/// How many times the cost of one cut the fold must spend before the
/// next: cutting takes at most a tenth of the fold thread's time
/// (sprawling-SPEC.md 8-91).
const CUT_SHARE_INVERSE: u32 = 10;

/// When the fold thread cuts the next views snapshot, from the fold time
/// spent since the last cut and what that cut cost, both measured here
/// (sprawling-SPEC.md 8-91).
#[derive(Default)]
struct Cadence {
    folded_since_cut: Duration,
    last_cut_cost: Option<Duration>,
    /// The last record folded and not yet under a snapshot.
    uncut: Option<EventRecord>,
    cutting: Cutting,
}

/// Whether the views may still be cut: a snapshot of views that refused
/// a record would let a start accept history a whole fold refuses.
#[derive(Default)]
enum Cutting {
    #[default]
    Open,
    StoppedByRefusal,
}

/// How the views took one batch.
enum Folded {
    Clean,
    Refused,
}

impl Cadence {
    /// Counts a batch folded in `cost`, ending at `last` when it carried
    /// a committed record.
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

    /// The record a snapshot is due at now, when one is.
    fn due(&self) -> Option<EventRecord> {
        let spent_enough = self
            .last_cut_cost
            .is_none_or(|cut| self.folded_since_cut >= cut.saturating_mul(CUT_SHARE_INVERSE));
        self.uncut().filter(|_| spent_enough)
    }

    /// The last record folded since the last cut, while cutting is open.
    fn uncut(&self) -> Option<EventRecord> {
        match self.cutting {
            Cutting::Open => self.uncut.clone(),
            Cutting::StoppedByRefusal => None,
        }
    }

    /// Starts counting afresh after a cut that took `cost`.
    fn cut(&mut self, cost: Duration) {
        self.last_cut_cost = Some(cost);
        self.folded_since_cut = Duration::ZERO;
        self.uncut = None;
    }
}

/// Cuts a snapshot of `views` at `record` and tells `cadence` what it
/// cost. A cut that fails is reported and serving goes on: the snapshot
/// only shortens the next start.
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
        Fold::Examined(_) | Fold::Lent(_) => None,
    })
}

/// Moves the head past `record` and sends it to every client. The frame
/// is spelled here, once, whatever the number of sockets that will write
/// it (wire-SPEC.md 8-47).
fn send_committed(broadcast: &Broadcast, record: &EventRecord) {
    broadcast.head.advance(record.seq());
    match wire::Committed::new(record.clone()) {
        // A send with no subscribers is not a failure: a city with no
        // browser open is a city doing its work.
        Ok(committed) => drop(broadcast.to_clients.send(committed)),
        // The next record's seq gap makes every live session send
        // `Lagged` for this one (wire-SPEC.md 8-41).
        Err(unframed) => {
            eprintln!("a committed record has no frame and reaches no client: {unframed}");
        }
    }
}

fn fold_batch(views: &mut Views, batch: &[Fold]) -> Folded {
    batch
        .iter()
        .fold(Folded::Clean, |verdict, fold| match fold {
            // A record the views refuse to fold is reported and skipped:
            // the ledger already has it, and a view that stopped the fold
            // would make history hostage to a projection.
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
        })
}

/// Takes the replaced copy back once the readers that took it before the
/// swap have let go, which they do as soon as `prepare` has copied out
/// what a query needs.
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
