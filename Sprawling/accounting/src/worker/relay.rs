// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::VecDeque;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use kernel::{AxCode, AxError, EventDraft, EventRef, Ledger};

use super::booking::{ClaimAsk, ClaimBook};
use super::errand::Answer;
use super::health::Health;
use super::pool::Arrival;
use super::registering::GoalAsk;

pub(crate) struct RelayRequest {
    drafts: Vec<EventDraft>,

    back: mpsc::SyncSender<Result<Vec<EventRef>, AxError>>,

    queued: Instant,
}

pub(crate) enum Wake {

    Relay(RelayRequest),

    Claim(ClaimAsk),

    Goal(GoalAsk),

    Home(Box<Arrival>),

    Answered(Box<Answer>),

    Command,

    Close,
}

pub(crate) struct Drained {

    pub written: Vec<EventDraft>,

    pub goals: Vec<GoalAsk>,

    pub answers: Vec<Answer>,
}

pub(crate) enum Patience {

    Now,

    For(Duration),

    Unbounded,
}

#[derive(Clone)]
pub struct Relay {
    asking: mpsc::Sender<Wake>,
    health: Health,
    monotonic: fn() -> Instant,
}

impl Ledger for Relay {

    fn append(&mut self, draft: EventDraft) -> Result<EventRef, AxError> {
        self.append_all(vec![draft])?
            .into_iter()
            .next()
            .ok_or_else(|| gone("the accounting thread answered one draft with no ref"))
    }

    fn append_all(&mut self, drafts: Vec<EventDraft>) -> Result<Vec<EventRef>, AxError> {
        if drafts.is_empty() {
            return Ok(Vec::new());
        }
        let (back, answer) = mpsc::sync_channel(0);

        let queued = (self.monotonic)();
        self.health.asked();
        self.asking
            .send(Wake::Relay(RelayRequest {
                drafts,
                back,
                queued,
            }))
            .map_err(|_| {
                self.health.withdrawn();
                gone("the accounting thread is no longer taking writes")
            })?;
        answer
            .recv()
            .map_err(|_| gone("the accounting thread ended before answering"))?
    }
}

pub(crate) struct RelayGate {
    wakes: mpsc::Receiver<Wake>,
    issuing: mpsc::Sender<Wake>,
    pub booked: ClaimBook,

    health: Health,

    monotonic: fn() -> Instant,
}

impl RelayGate {
    pub fn open(monotonic: fn() -> Instant) -> RelayGate {
        let (issuing, wakes) = mpsc::channel();
        RelayGate {
            wakes,
            issuing,
            booked: ClaimBook::default(),
            health: Health::default(),
            monotonic,
        }
    }

    pub fn health(&self) -> Health {
        self.health.clone()
    }

    pub(crate) fn issue(&self) -> Relay {
        Relay {
            asking: self.issuing.clone(),
            health: self.health.clone(),
            monotonic: self.monotonic,
        }
    }

    pub(crate) fn bell(&self) -> mpsc::Sender<Wake> {
        self.issuing.clone()
    }

    pub fn serve(
        &mut self,
        patience: Patience,
        ledger: &mut impl Ledger,
        homes: &mut VecDeque<Arrival>,
    ) -> Drained {

        let asleep = (self.monotonic)();
        let first = match patience {
            Patience::Now => self.wakes.try_recv().ok(),
            Patience::For(wait) => self.wakes.recv_timeout(wait).ok(),
            Patience::Unbounded => self.wakes.recv().ok(),
        };
        self.health
            .slept((self.monotonic)().saturating_duration_since(asleep));
        let mut written = Vec::new();
        let mut goals = Vec::new();
        let mut answers = Vec::new();
        let mut drafts = Vec::new();
        let mut senders = Vec::new();
        let mut sizes = Vec::new();
        let queued = std::iter::from_fn(|| self.wakes.try_recv().ok());
        for wake in first.into_iter().chain(queued) {
            match wake {
                Wake::Relay(RelayRequest {
                    drafts: batch,
                    back,
                    queued,
                }) => {
                    self.health
                        .queue_waited((self.monotonic)().saturating_duration_since(queued));
                    sizes.push(batch.len());
                    drafts.extend(batch);
                    senders.push(back);
                }
                Wake::Claim(ask) => written.extend(self.booked.answer(ask, ledger)),
                Wake::Goal(ask) => goals.push(ask),
                Wake::Home(arrival) => homes.push_back(*arrival),
                Wake::Answered(answer) => answers.push(*answer),
                Wake::Command | Wake::Close => {}
            }
        }
        if senders.is_empty() {
            return Drained {
                written,
                goals,
                answers,
            };
        }
        let kept = drafts.clone();
        let batch = u64::try_from(senders.len()).unwrap_or(u64::MAX);
        self.health.taken(batch);
        match ledger.append_all(drafts) {
            Ok(echoes) => {

                let mut echoes = echoes.into_iter();
                for (back, size) in senders.into_iter().zip(sizes) {
                    let theirs: Vec<EventRef> = echoes.by_ref().take(size).collect();
                    let answer = if theirs.len() == size {
                        Ok(theirs)
                    } else {
                        Err(gone(
                            "the ledger answered a batch with fewer refs than drafts",
                        ))
                    };

                    drop(back.send(answer));
                }
                written.extend(kept);
            }
            Err(refused) => {
                for back in senders {
                    drop(back.send(Err(refused.clone())));
                }
            }
        }
        self.health.answered(batch);
        Drained {
            written,
            goals,
            answers,
        }
    }
}

fn gone(why: &str) -> AxError {
    AxError::failure(AxCode::StorageFatal, "append through the relay", why)
        .with_recovery("the city is closing; resume it and the run replays from its last line")
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
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
                let (mut ledger, _opened) =
                    storage::JsonlLedger::open(&root, kernel::TimeMs::new(0))
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
}
