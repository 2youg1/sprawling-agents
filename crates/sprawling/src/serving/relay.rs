// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The third adapter of `kernel::Ledger`: a write from a driving thread,
//! carried to the accounting thread and waited for.
//!
//! The first two adapters own a medium — `memory::jsonl` owns segments on
//! disk, citysim's owns a `Vec`. This one owns neither. It owns a
//! crossing: the driving pool holds `Relay` and nothing else that can
//! write, so "a city has one writer" is held by the types rather than by
//! discipline (ARCHITECTURE section 10, sprawling-SPEC.md 8-42).

use std::sync::mpsc;

use kernel::{AxCode, AxError, EventDraft, EventRef, Ledger};

/// One append, and the address its answer goes back to.
///
/// The two travel together because a driving thread is blocked on the
/// second until the first has been written: an append with no way back
/// is a thread that never wakes.
struct RelayRequest {
    draft: EventDraft,
    /// A rendezvous channel, so there is no third state between "the
    /// accounting thread wrote it" and "the driving thread knows".
    back: mpsc::SyncSender<Result<EventRef, AxError>>,
}

/// The face a driving thread writes history through, and the only
/// `kernel::Ledger` it is given.
///
/// Cloned per driving thread. Nothing here decides anything: seq, prev
/// and the bytes stay with the adapter on the accounting side, which is
/// what keeps one city to one writer.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the driving pool of card 3.3 is the only production caller, and this expectation is unfulfilled the moment that pool lands"
    )
)]
#[derive(Clone)]
pub(crate) struct Relay {
    asking: mpsc::Sender<RelayRequest>,
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
        self.asking
            .send(RelayRequest { draft, back })
            .map_err(|_| gone("the accounting thread is no longer taking writes"))?;
        answer
            .recv()
            .map_err(|_| gone("the accounting thread ended before answering"))?
    }
}

/// The face the accounting thread serves relay requests through.
///
/// It holds a sender of its own so the channel stays connected while a
/// city is served: a gate that had handed out every sender would report
/// "nobody is writing" as "the writer is gone".
pub(crate) struct RelayGate {
    asks: mpsc::Receiver<RelayRequest>,
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "the driving pool of card 3.3 is the only production caller, and this expectation is unfulfilled the moment that pool lands"
        )
    )]
    issuing: mpsc::Sender<RelayRequest>,
}

impl RelayGate {
    pub(crate) fn open() -> RelayGate {
        let (issuing, asks) = mpsc::channel();
        RelayGate { asks, issuing }
    }

    /// One handle for one driving thread.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "the driving pool of card 3.3 is the only production caller, and this expectation is unfulfilled the moment that pool lands"
        )
    )]
    pub(crate) fn issue(&self) -> Relay {
        Relay {
            asking: self.issuing.clone(),
        }
    }

    /// Writes every request already waiting, and returns how many.
    ///
    /// It never waits for one. The accounting thread's loop serves these
    /// before it looks at the command desk, so a run that has already
    /// been paid for does not queue behind a command that has not
    /// started (sprawling-SPEC.md 8-42-2).
    pub(crate) fn serve_waiting(&self, ledger: &mut impl Ledger) -> usize {
        let mut served = 0usize;
        while let Ok(RelayRequest { draft, back }) = self.asks.try_recv() {
            let echo = ledger.append(draft);
            // A driving thread that stopped listening does not undo the
            // line: the history is what the city believes, and it was
            // written before this send was tried.
            let _ = back.send(echo);
            served = served.saturating_add(1);
        }
        served
    }
}

/// The one refusal this crossing owns, in the three parts every refusal
/// here is made of.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the driving pool of card 3.3 is the only production caller, and this expectation is unfulfilled the moment that pool lands"
    )
)]
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
    use super::{Relay, RelayGate};
    use kernel::conformance::{LedgerInspect, assert_ledger_conformance};
    use kernel::{AxError, EventDraft, EventRef, Ledger};

    /// The relay, with the accounting thread it writes through.
    ///
    /// `LedgerInspect` is the suite's verification-only read-back, and
    /// `kernel::ledger` says production never reads through a Ledger
    /// handle — so the read-back is answered here, off the segments the
    /// accounting thread wrote, rather than by opening a hole in the
    /// relay's own face.
    struct Relayed {
        relay: Relay,
        dir: tempfile::TempDir,
        stopping: std::sync::Arc<std::sync::atomic::AtomicBool>,
        accounting: Option<std::thread::JoinHandle<()>>,
    }

    impl Relayed {
        fn open() -> Relayed {
            let dir = tempfile::tempdir().expect("a temporary directory");
            let root = dir.path().to_path_buf();
            let gate = RelayGate::open();
            let relay = gate.issue();
            let stopping = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let closing = std::sync::Arc::clone(&stopping);
            let accounting = std::thread::spawn(move || {
                let (mut ledger, _opened) =
                    memory::JsonlLedger::open(&root, kernel::TimeMs::new(0))
                        .expect("a fresh ledger opens");
                while !closing.load(std::sync::atomic::Ordering::Acquire) {
                    if gate.serve_waiting(&mut ledger) == 0 {
                        std::thread::sleep(std::time::Duration::from_millis(1));
                    }
                }
            });
            Relayed {
                relay,
                dir,
                stopping,
                accounting: Some(accounting),
            }
        }
    }

    impl Drop for Relayed {
        fn drop(&mut self) {
            self.stopping
                .store(true, std::sync::atomic::Ordering::Release);
            if let Some(accounting) = self.accounting.take() {
                let _ = accounting.join();
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
            memory::read_raw_lines_at(self.dir.path()).map_err(memory::MemoryError::into_ax)
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
}
