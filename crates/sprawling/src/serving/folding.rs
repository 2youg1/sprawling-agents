// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The thread the views are folded on, so the writer never waits for a
//! reader (sprawling-SPEC.md 8-89).

use std::sync::{Arc, Mutex, mpsc};

use kernel::{AxCode, AxError, EventRecord};

use crate::views::Views;

/// The two places the writer thread hands the views what it wrote, and
/// the thread that folds it.
pub(crate) struct Folding {
    pub(crate) observer: Box<dyn FnMut(&EventRecord) + Send>,
    pub(crate) machine: Arc<dyn Fn(channels::DoctorAnswer) + Send + Sync>,
    pub(crate) thread: std::thread::JoinHandle<()>,
}

/// What the writer thread hands the view fold, in the order it wrote it.
enum Fold {
    Committed(EventRecord),
    Examined(channels::DoctorAnswer),
}

/// Where a folded record goes next: the clients watching the city, and
/// the head every welcome names, moved before the record is sent so a
/// session that reads the head has already subscribed to what follows it.
pub(crate) struct Broadcast {
    pub(crate) to_clients: tokio::sync::broadcast::Sender<channels::Committed>,
    pub(crate) head: Arc<channels::LedgerHead>,
}

/// Whether the views still follow the ledger. Once a panic has poisoned
/// the lock the fold stops, because a half-applied record leaves state
/// no later record can be trusted to correct.
enum Following {
    Live,
    Stopped,
}

/// Starts the view fold.
///
/// Both halves only queue: the writer's cost per record is one channel
/// send, whatever a reader is doing with the views.
///
/// # Errors
/// `StorageFatal` when the thread cannot be started.
pub(crate) fn spawn_folding(
    views: Arc<Mutex<Views>>,
    broadcast: Broadcast,
) -> Result<Folding, AxError> {
    let (committed, arriving) = mpsc::channel::<Fold>();
    let examined = committed.clone();
    let thread = std::thread::Builder::new()
        .name("sprawling-views".to_owned())
        .spawn(move || fold_until_closed(&views, &arriving, &broadcast))
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
        machine: Arc::new(move |found: channels::DoctorAnswer| {
            if examined.send(Fold::Examined(found)).is_err() {
                eprintln!(
                    "the view fold has ended; this machine's check is not shown until the server restarts"
                );
            }
        }),
        thread,
    })
}

/// Folds each arrival, then broadcasts it, until every sender is gone.
///
/// The broadcast follows the fold so a client that queries on hearing a
/// record finds it already folded.
fn fold_until_closed(views: &Mutex<Views>, arriving: &mpsc::Receiver<Fold>, broadcast: &Broadcast) {
    let mut following = Following::Live;
    for fold in arriving {
        following = match following {
            Following::Live => fold_one(views, &fold),
            Following::Stopped => Following::Stopped,
        };
        // The frame is spelled here, once, whatever the number of
        // sockets that will write it (channels-SPEC.md 8-47).
        if let Fold::Committed(record) = fold {
            broadcast.head.advance(record.seq());
            match channels::Committed::new(record) {
                // A send with no subscribers is not a failure: a city
                // with no browser open is a city doing its work.
                Ok(committed) => drop(broadcast.to_clients.send(committed)),
                // The next record's seq gap makes every live session
                // send `Lagged` for this one (channels-SPEC.md 8-41).
                Err(unframed) => {
                    eprintln!("a committed record has no frame and reaches no client: {unframed}");
                }
            }
        }
    }
}

fn fold_one(views: &Mutex<Views>, fold: &Fold) -> Following {
    let Ok(mut held) = views.lock() else {
        let at = match fold {
            Fold::Committed(record) => format!("record {}", record.seq().value()),
            Fold::Examined(_) => "this machine's check".to_owned(),
        };
        eprintln!(
            "the view lock is poisoned; the views stop before {at} and no longer follow the ledger - restart the server to rebuild them from it"
        );
        return Following::Stopped;
    };
    match fold {
        // A record the views refuse to fold is reported and skipped:
        // the ledger already has it, and a view that stopped the fold
        // would make history hostage to a projection.
        Fold::Committed(record) => {
            if let Err(err) = held.apply(record) {
                eprintln!("view fold refused {}: {err}", record.seq().value());
            }
        }
        Fold::Examined(found) => held.found_on_this_machine(found.clone()),
    }
    Following::Live
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
