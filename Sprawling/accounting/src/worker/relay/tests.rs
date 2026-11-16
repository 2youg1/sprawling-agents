// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::sync::mpsc;

use super::{Patience, Relay, RelayGate, RelayRequest, Wake};
use kernel::ledger::conformance::{LedgerInspect, assert_ledger_conformance};
use kernel::{AxError, EventDraft, EventRef, Ledger};

struct Relayed {
    relay: Relay,
    dir: tempfile::TempDir,
    closing: mpsc::Sender<()>,
    bell: mpsc::Sender<Wake>,
    accounting: Option<std::thread::JoinHandle<()>>,
}

impl Relayed {
    fn open() -> Relayed {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let root = dir.path().to_path_buf();
        let mut gate = RelayGate::open(crate::worker::fixture::monotonic);
        let relay = gate.issue();
        let bell = gate.bell();
        let (closing, closed) = mpsc::channel::<()>();
        let accounting = std::thread::spawn(move || {
            let (mut ledger, _opened) = storage::JsonlLedger::open(&root, kernel::TimeMs::new(0))
                .expect("a fresh ledger opens");
            let mut homes = std::collections::VecDeque::new();
            while closed.try_recv().is_err() {
                gate.serve(Patience::Unbounded, &mut ledger, &mut homes);
            }
        });
        Relayed {
            relay,
            dir,
            closing,
            bell,
            accounting: Some(accounting),
        }
    }
}

impl Drop for Relayed {
    fn drop(&mut self) {
        self.closing
            .send(())
            .expect("the accounting thread still listens");
        drop(self.bell.send(Wake::Close));
        if let Some(accounting) = self.accounting.take() {
            drop(accounting.join());
        }
    }
}

impl Ledger for Relayed {
    fn append(&mut self, draft: EventDraft) -> Result<EventRef, AxError> {
        self.relay.append(draft)
    }
}

impl LedgerInspect for Relayed {
    fn raw_lines(&self) -> Result<Vec<Vec<u8>>, AxError> {
        storage::read_raw_lines_at(self.dir.path()).map_err(storage::StorageError::into_ax)
    }
}

#[test]
fn the_relay_passes_the_ledger_conformance_suite() {
    assert_ledger_conformance(Relayed::open);
}

#[test]
fn a_relay_whose_accounting_thread_is_gone_refuses() {
    let mut orphan = {
        let gate = RelayGate::open(crate::worker::fixture::monotonic);
        gate.issue()
    };
    let refused = orphan.append(EventDraft {
        run: kernel::RunId::CITY,
        t: kernel::TimeMs::new(1),
        who: "city".to_owned(),
        addr: None,
        kind: kernel::EventKind::CityInitialized,
        data: kernel::Payload::empty(),
        ig: false,
    });
    assert!(refused.is_err(), "a writer that is gone is not a success");
}

#[test]
fn the_accounting_queue_counts_what_waits_and_what_is_not_yet_durable() {
    use crate::worker::health::Health;
    fn standing(health: &Health) -> (u64, u64) {
        let read = health.read(wire::Sample::default());
        (read.ledger_queue_depth, read.durable_lag)
    }
    struct Watching {
        health: Health,
        during: Option<(u64, u64)>,
    }
    impl Ledger for Watching {
        fn append(&mut self, draft: EventDraft) -> Result<EventRef, AxError> {
            self.append_all(vec![draft])?
                .into_iter()
                .next()
                .ok_or_else(|| super::gone("a wave of one answered with nothing"))
        }

        fn append_all(&mut self, drafts: Vec<EventDraft>) -> Result<Vec<EventRef>, AxError> {
            self.during = Some(standing(&self.health));
            Ok(drafts
                .into_iter()
                .map(|draft| {
                    kernel::EventRecord::from_draft(
                        draft,
                        kernel::Seq::new(1),
                        kernel::GENESIS_PREV,
                    )
                    .to_ref()
                })
                .collect())
        }
    }

    let mut gate = RelayGate::open(crate::worker::fixture::monotonic);
    let health = gate.health();
    let mut relay = gate.issue();
    let lane = std::thread::spawn(move || {
        relay.append(EventDraft {
            run: kernel::RunId::CITY,
            t: kernel::TimeMs::new(1),
            who: "city".to_owned(),
            addr: None,
            kind: kernel::EventKind::CityInitialized,
            data: kernel::Payload::empty(),
            ig: false,
        })
    });
    let mut before = standing(&health);
    for _ in 0..500 {
        if before == (1, 0) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
        before = standing(&health);
    }
    let mut store = Watching {
        health: health.clone(),
        during: None,
    };
    gate.serve(
        Patience::Unbounded,
        &mut store,
        &mut std::collections::VecDeque::new(),
    );
    assert!(lane.join().unwrap().is_ok(), "the lane is answered");
    assert_eq!(
        (before, store.during, standing(&health)),
        ((1, 0), Some((0, 1)), (0, 0))
    );
}

#[test]
fn a_relay_request_waits_on_the_gates_clock() {
    #[allow(
        clippy::disallowed_methods,
        reason = "test code: the counted clock's origin"
    )]
    fn counted() -> std::time::Instant {
        static ORIGIN: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
        static READ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        *ORIGIN.get_or_init(std::time::Instant::now)
            + std::time::Duration::from_micros(
                READ.fetch_add(1, std::sync::atomic::Ordering::SeqCst),
            )
    }
    let mut gate = RelayGate::open(counted);
    let health = gate.health();
    let mut relay = gate.issue();
    let lane = std::thread::spawn(move || {
        relay.append(EventDraft {
            run: kernel::RunId::CITY,
            t: kernel::TimeMs::new(1),
            who: "city".to_owned(),
            addr: None,
            kind: kernel::EventKind::CityInitialized,
            data: kernel::Payload::empty(),
            ig: false,
        })
    });
    for _ in 0..500 {
        if health.read(wire::Sample::default()).ledger_queue_depth == 1 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let dir = tempfile::tempdir().unwrap();
    let (mut ledger, _opened) =
        storage::JsonlLedger::open(dir.path(), kernel::TimeMs::new(0)).unwrap();
    gate.serve(
        Patience::Unbounded,
        &mut ledger,
        &mut std::collections::VecDeque::new(),
    );
    assert!(lane.join().unwrap().is_ok(), "the lane is answered");
    assert_eq!(health.relay_queue_us(), vec![3]);
}

#[test]
fn everything_already_waiting_reaches_the_store_in_one_wave() {
    struct Counting {
        waves: usize,
        records: usize,
        seq: u64,
    }
    impl Ledger for Counting {
        fn append(&mut self, draft: EventDraft) -> Result<EventRef, AxError> {
            self.append_all(vec![draft])?
                .into_iter()
                .next()
                .ok_or_else(|| super::gone("a wave of one answered with nothing"))
        }

        fn append_all(&mut self, drafts: Vec<EventDraft>) -> Result<Vec<EventRef>, AxError> {
            self.waves = self.waves.saturating_add(1);
            self.records = self.records.saturating_add(drafts.len());
            let mut refs = Vec::new();
            for draft in drafts {
                self.seq = self.seq.saturating_add(1);
                refs.push(
                    kernel::EventRecord::from_draft(
                        draft,
                        kernel::Seq::new(self.seq),
                        kernel::GENESIS_PREV,
                    )
                    .to_ref(),
                );
            }
            Ok(refs)
        }
    }

    let mut gate = RelayGate::open(crate::worker::fixture::monotonic);
    let mut answers = Vec::new();
    for stamp in 1..=4 {
        let (back, answer) = mpsc::sync_channel(1);
        answers.push(answer);
        gate.issuing
            .send(Wake::Relay(RelayRequest {
                queued: crate::worker::fixture::monotonic(),
                drafts: vec![EventDraft {
                    run: kernel::RunId::CITY,
                    t: kernel::TimeMs::new(stamp),
                    who: "city".to_owned(),
                    addr: None,
                    kind: kernel::EventKind::CityInitialized,
                    data: kernel::Payload::empty(),
                    ig: false,
                }],
                back,
            }))
            .expect("the gate holds a sender of its own");
    }

    let mut store = Counting {
        waves: 0,
        records: 0,
        seq: 0,
    };
    gate.serve(
        Patience::Now,
        &mut store,
        &mut std::collections::VecDeque::new(),
    );
    assert_eq!(store.records, 4, "every draft reached the store");
    assert_eq!(
        store.waves, 1,
        "four drafts waiting together must ride one disk barrier, not four"
    );
    for answer in answers {
        let echo = answer
            .try_recv()
            .expect("an answer is already there, because the write came first");
        assert!(echo.is_ok(), "every waiting draft is answered");
    }
}

#[test]
fn tf1_relay_answers_a_batch_in_one_round_trip() {
    struct Waves(Vec<usize>);
    impl Ledger for Waves {
        fn append(&mut self, draft: EventDraft) -> Result<EventRef, AxError> {
            self.append_all(vec![draft])?
                .into_iter()
                .next()
                .ok_or_else(|| super::gone("a wave of one answered with nothing"))
        }

        fn append_all(&mut self, drafts: Vec<EventDraft>) -> Result<Vec<EventRef>, AxError> {
            self.0.push(drafts.len());
            Ok((1..)
                .zip(drafts)
                .map(|(seq, draft)| {
                    kernel::EventRecord::from_draft(
                        draft,
                        kernel::Seq::new(seq),
                        kernel::GENESIS_PREV,
                    )
                    .to_ref()
                })
                .collect())
        }
    }

    let mut gate = RelayGate::open(crate::worker::fixture::monotonic);
    let mut relay = gate.issue();
    let draft = |stamp| EventDraft {
        run: kernel::RunId::CITY,
        t: kernel::TimeMs::new(stamp),
        who: "city".to_owned(),
        addr: None,
        kind: kernel::EventKind::CityInitialized,
        data: kernel::Payload::empty(),
        ig: false,
    };
    let lane = std::thread::spawn(move || relay.append_all(vec![draft(1), draft(2), draft(3)]));
    let mut store = Waves(Vec::new());
    gate.serve(
        Patience::Unbounded,
        &mut store,
        &mut std::collections::VecDeque::new(),
    );
    let seqs: Vec<u64> = lane
        .join()
        .unwrap()
        .unwrap()
        .iter()
        .map(|echo| echo.seq().value())
        .collect();
    assert_eq!((store.0, seqs), (vec![3], vec![1, 2, 3]));
}
