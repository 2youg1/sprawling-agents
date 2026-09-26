// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

use kernel::EventRecord;

use super::{Broadcast, spawn_folding};
use crate::assembly::init_city;
use crate::views::Views;

/// A reader holding the views - the changes page running `git status`
/// under the lock - must not hold up the writer: the observer returns
/// while the lock is still held, and the record is folded and broadcast
/// once the reader lets go.
#[test]
fn the_writer_does_not_wait_for_a_reader_holding_the_views() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    let verified = runtime::replay::verify_ledger_dir(&report.ledger_dir).unwrap();
    let genesis = EventRecord::parse_line(verified.raw_lines().first().unwrap()).unwrap();
    let views = Arc::new(Mutex::new(Views::new(dir.path())));
    let (to_clients, mut heard) = tokio::sync::broadcast::channel(8);
    let head = Arc::new(channels::LedgerHead::default());
    let broadcast = Broadcast {
        to_clients,
        head: Arc::clone(&head),
    };
    let mut folding = spawn_folding(Arc::clone(&views), broadcast).unwrap();

    let reader = views.lock().unwrap();
    let (written, returned) = mpsc::channel();
    let sent = genesis.clone();
    let writer = std::thread::spawn(move || {
        (folding.observer)(&sent);
        written.send(()).unwrap();
        folding
    });
    let waited = returned.recv_timeout(Duration::from_secs(2));
    drop(reader);
    assert_eq!(waited, Ok(()), "the writer waited for the reader");

    let folding = writer.join().unwrap();
    drop((folding.observer, folding.machine));
    folding.thread.join().unwrap();
    assert_eq!(heard.try_recv().unwrap(), genesis);
    assert_eq!(head.read(), Some(genesis.seq()));
    let mut folded_here = Views::new(dir.path());
    folded_here.apply(&genesis).unwrap();
    let city = channels::Query::CityView;
    assert_eq!(
        views.lock().unwrap().answer(&city),
        folded_here.answer(&city)
    );
}
