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
mod tests;
