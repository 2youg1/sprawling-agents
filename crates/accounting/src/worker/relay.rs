// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The third adapter of `kernel::Ledger`: a write from a driving thread,
//! carried to the accounting thread and waited for.
//!
//! The first two adapters own a medium — `storage::jsonl` owns segments on disk, citysim's
//! owns a `Vec`. This one owns neither. It owns a crossing: the driving pool holds `Relay`
//! and nothing else that can write, so "a city has one writer" is held by the types rather
//! than by discipline (ARCHITECTURE section 10, sprawling-SPEC.md 8-42).

use std::collections::VecDeque;
use std::sync::mpsc;
use std::time::Duration;

use kernel::{AxCode, AxError, EventDraft, EventRef, Ledger};

use super::booking::{ClaimAsk, ClaimBook};
use super::health::Health;
use super::pool::Arrival;
use super::registering::GoalAsk;

/// One append, and the address its answer goes back to.
///
/// The two travel together because a driving thread is blocked on the
/// second until the first has been written: an append with no way back
/// is a thread that never wakes.
pub(crate) struct RelayRequest {
    draft: EventDraft,
    /// A rendezvous channel, so there is no third state between "the
    /// accounting thread wrote it" and "the driving thread knows".
    back: mpsc::SyncSender<Result<EventRef, AxError>>,
}

/// Everything that wakes the accounting thread, on the one queue it
/// blocks on.
///
/// One queue, because a thread blocks on one thing at a time: a queue
/// per mouth is polled with timeouts that every request waits out
/// (sprawling-SPEC.md 8-42-4, `crates/accounting/spec/Worker/Attend.lean`).
pub(crate) enum Wake {
    /// A lane's append, waiting for its answer.
    Relay(RelayRequest),
    /// A lane's claim on a plan node, decided in queue order.
    Claim(ClaimAsk),
    /// A lane's goal registration, decided against the city's register.
    Goal(GoalAsk),
    /// A run home from its lane, boxed because it carries a whole
    /// drive's outcome.
    Home(Box<Arrival>),
    /// A command was posted; it stays on the desk for a run's safe points.
    Command,
    /// The city is stopping; the desk says so too.
    Close,
}

/// What one look at the queue leaves the accounting thread.
pub(crate) struct Drained {
    /// Every line the ledger took, in ledger order: a line a lane wrote
    /// is history as much as one the accounting thread wrote, so the
    /// same folds are shown it (sprawling-SPEC.md 8-110).
    pub written: Vec<EventDraft>,
    /// Registrations in arrival order, left to the thread that holds the
    /// goal register: a copy of the register here would be a second
    /// answer to who holds the ground (sprawling-SPEC.md 8-42-8).
    pub goals: Vec<GoalAsk>,
}

/// How long one look at the queue may wait for its first wake.
pub(crate) enum Patience {
    /// Not at all: serve what is already queued.
    Now,
    /// At most this long: an idle city's one unasked wake, the schedule.
    For(Duration),
    /// Until something is queued.
    Unbounded,
}

/// The face a driving thread writes history through, and the only
/// `kernel::Ledger` it is given.
///
/// Cloned per driving thread, and once for the remote door, whose five
/// lines the assembly writes from outside any run (sprawling-SPEC.md
/// 8-139). Nothing here decides anything: seq, prev and the bytes stay
/// with the adapter on the accounting side, which is what keeps one city
/// to one writer.
#[derive(Clone)]
pub struct Relay {
    asking: mpsc::Sender<Wake>,
    health: Health,
}

impl Ledger for Relay {
    /// Sends the draft to the accounting thread and **blocks** until it
    /// answers.
    ///
    /// Blocking is the contract rather than a cost: `kernel::ledger`
    /// says `Ok(ref)` means the record is durable, so a relay that
    /// answered on hand-off would let a run's `model_called` reach the
    /// history before its own `run_started`.
    ///
    /// # Errors
    /// Refuses when the accounting thread is gone, which is the one
    /// failure this crossing adds: a driving thread must be able to
    /// freeze its run rather than wait for a writer that ended.
    fn append(&mut self, draft: EventDraft) -> Result<EventRef, AxError> {
        let (back, answer) = mpsc::sync_channel(0);
        self.health.asked();
        self.asking
            .send(Wake::Relay(RelayRequest { draft, back }))
            .map_err(|_| {
                self.health.withdrawn();
                gone("the accounting thread is no longer taking writes")
            })?;
        answer
            .recv()
            .map_err(|_| gone("the accounting thread ended before answering"))?
    }
}

/// The face the accounting thread serves the one queue through.
///
/// It holds a sender of its own so the channel stays connected while a
/// city is served: a gate that had handed out every sender would report
/// "nobody is writing" as "the writer is gone".
pub(crate) struct RelayGate {
    wakes: mpsc::Receiver<Wake>,
    issuing: mpsc::Sender<Wake>,
    pub booked: ClaimBook,
    /// How many appends wait and how many are not yet durable
    /// (sprawling-SPEC.md 8-98).
    health: Health,
}

impl RelayGate {
    pub fn open() -> RelayGate {
        let (issuing, wakes) = mpsc::channel();
        RelayGate {
            wakes,
            issuing,
            booked: ClaimBook::default(),
            health: Health::default(),
        }
    }

    /// A handle onto this gate's counts, for the sampler's thread.
    pub fn health(&self) -> Health {
        self.health.clone()
    }

    /// One handle for one driving thread.
    pub(crate) fn issue(&self) -> Relay {
        Relay {
            asking: self.issuing.clone(),
            health: self.health.clone(),
        }
    }

    /// A sender for the lanes coming home and for the desk.
    pub(crate) fn bell(&self) -> mpsc::Sender<Wake> {
        self.issuing.clone()
    }

    /// Waits as `patience` allows for the first wake, then takes every
    /// wake already queued: relay requests are written before anything
    /// lands (sprawling-SPEC.md 8-42-2), claims are answered in queue
    /// order, runs home go on `homes` and goal registrations into the
    /// returned [`Drained`] in arrival order, and a command or a close
    /// only ends the wait.
    ///
    /// **Everything queued rides one disk barrier**, which costs the
    /// same for fifty records as for one. Each answer still goes back
    /// only after the write is durable, because that is what makes an
    /// `EventRef` a reference to a history that exists.
    pub fn serve(
        &mut self,
        patience: Patience,
        ledger: &mut impl Ledger,
        homes: &mut VecDeque<Arrival>,
    ) -> Drained {
        // The gate's own sender keeps the queue connected: empty is "not yet".
        let first = match patience {
            Patience::Now => self.wakes.try_recv().ok(),
            Patience::For(wait) => self.wakes.recv_timeout(wait).ok(),
            Patience::Unbounded => self.wakes.recv().ok(),
        };
        let mut written = Vec::new();
        let mut goals = Vec::new();
        let mut drafts = Vec::new();
        let mut senders = Vec::new();
        let queued = std::iter::from_fn(|| self.wakes.try_recv().ok());
        for wake in first.into_iter().chain(queued) {
            match wake {
                Wake::Relay(RelayRequest { draft, back }) => {
                    drafts.push(draft);
                    senders.push(back);
                }
                Wake::Claim(ask) => written.extend(self.booked.answer(ask, ledger)),
                Wake::Goal(ask) => goals.push(ask),
                Wake::Home(arrival) => homes.push_back(*arrival),
                Wake::Command | Wake::Close => {}
            }
        }
        if senders.is_empty() {
            return Drained { written, goals };
        }
        let kept = drafts.clone();
        let batch = u64::try_from(senders.len()).unwrap_or(u64::MAX);
        self.health.taken(batch);
        match ledger.append_all(drafts) {
            Ok(echoes) => {
                // Positional, the port's own promise: a sender with no echo
                // would be a caller left waiting, never one told a lie.
                for (back, echo) in senders.into_iter().zip(echoes) {
                    // A driving thread that stopped listening does not undo the line: the
                    // history is what the city believes, and it was written before this send.
                    drop(back.send(Ok(echo)));
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
        Drained { written, goals }
    }
}

/// The one refusal this crossing owns, in the three parts every refusal
/// here is made of.
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

    /// The relay, with the accounting thread it writes through.
    ///
    /// `LedgerInspect` is the suite's verification-only read-back, and `kernel::ledger` says
    /// production never reads through a Ledger handle — so the read-back is answered here, off
    /// the segments the accounting thread wrote, rather than by opening a hole in the relay.
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
            let mut gate = RelayGate::open();
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
            // Read off the segments rather than through a second handle:
            // the accounting thread still holds the one that is
            // appending, and a city has one writer.
            storage::read_raw_lines_at(self.dir.path()).map_err(storage::StorageError::into_ax)
        }
    }

    /// **The property the card exists for.** The suite is the contract of
    /// the port, and it runs here one word unchanged: an adapter that
    /// hands the seq, the prev and the bytes to another thread is still
    /// held to the same six assertions as the one that writes them
    /// itself.
    #[test]
    fn the_relay_passes_the_ledger_conformance_suite() {
        assert_ledger_conformance(Relayed::open);
    }

    /// A relay whose accounting thread is gone refuses rather than
    /// blocking for ever. A driving thread waiting on a writer that
    /// ended is the one failure this crossing adds, so it has to be an
    /// error a run can be frozen with.
    #[test]
    fn a_relay_whose_accounting_thread_is_gone_refuses() {
        let mut orphan = {
            let gate = RelayGate::open();
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

    /// A lane's append counts as queued until the accounting thread takes
    /// it, then as not yet durable until its batch is written, then as
    /// neither (sprawling-SPEC.md 8-98).
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

        let mut gate = RelayGate::open();
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

    /// **The measurement this batching exists for, expressed as a
    /// count.** A drain that holds four drafts must reach the store
    /// once: a disk barrier costs the same for four records as for one,
    /// and a barrier per record would pay four. The counting store
    /// is the second adapter the port already allows for.
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

        // **"Already waiting" is arranged, not waited for.** Four driving threads racing to
        // queue before a wave begins makes the scheduler decide what is being measured: on a
        // busy machine the server answers the first before the fourth has sent, four waves of
        // one is then correct behaviour, and a test that retried the arrangement until it saw
        // batching failed on CI for being unlucky rather than for being wrong. The requests are
        // therefore put on the channel directly, which is the state the property is about -
        // `serve` drains what it finds, and what it finds here is four.
        let mut gate = RelayGate::open();
        let mut answers = Vec::new();
        for stamp in 1..=4 {
            let (back, answer) = mpsc::sync_channel(1);
            answers.push(answer);
            gate.issuing
                .send(Wake::Relay(RelayRequest {
                    draft: EventDraft {
                        run: kernel::RunId::CITY,
                        t: kernel::TimeMs::new(stamp),
                        who: "city".to_owned(),
                        addr: None,
                        kind: kernel::EventKind::CityInitialized,
                        data: kernel::Payload::empty(),
                        ig: false,
                    },
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
}
