// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::event::record::Lane;
use kernel::{Admission, AxCode, AxError, IdemKey, RunId, Seq};
use storage::{EventQueue, QueueLane};

mod signal;
pub use signal::{SenderState, Signal};

pub struct Inbox {
    urgent: EventQueue,
    ordinary: EventQueue,

    returned: std::collections::VecDeque<Signal>,
    bandwidth: u32,
}

impl std::fmt::Debug for Inbox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Inbox")
            .field("urgent", &self.urgent.len())
            .field("ordinary", &self.ordinary.len())
            .field("returned", &self.returned.len())
            .field("bandwidth", &self.bandwidth)
            .finish()
    }
}

impl Inbox {

    #[must_use]
    pub fn new(capacity: u64, bandwidth: u32) -> Inbox {
        Inbox {
            urgent: EventQueue::new(QueueLane::Signal, capacity),
            ordinary: EventQueue::new(QueueLane::Signal, capacity),
            returned: std::collections::VecDeque::new(),
            bandwidth: bandwidth.max(1),
        }
    }

    pub fn deliver(&mut self, signal: &Signal) -> Result<Admission, AxError> {
        let key = IdemKey::derive(&RunId::CITY, Seq::FIRST, signal.id().as_str().as_bytes());
        let payload = signal.queued_payload()?;
        let queue = match signal.lane() {
            Lane::Urgent => &mut self.urgent,
            Lane::Ordinary => &mut self.ordinary,
        };
        queue
            .enqueue(key, payload, signal.at())
            .map_err(storage::StorageError::into_ax)
    }

    pub fn give_back(&mut self, signals: Vec<Signal>) {
        for signal in signals.into_iter().rev() {
            self.returned.push_front(signal);
        }
    }

    pub fn pull(&mut self) -> Result<Vec<Signal>, AxError> {
        let mut out = Vec::new();
        while u32::try_from(out.len()).unwrap_or(u32::MAX) < self.bandwidth {
            if let Some(signal) = self.returned.pop_front() {
                out.push(signal);
                continue;
            }
            let item = match self.urgent.consume() {
                Some(item) => item,
                None => match self.ordinary.consume() {
                    Some(item) => item,
                    None => break,
                },
            };
            out.push(Signal::from_queued(&item.payload)?);
        }
        Ok(out)
    }

    pub fn take_steer(&mut self) -> Option<Signal> {
        let given_back = self
            .returned
            .iter()
            .position(|signal| matches!(signal.lane(), Lane::Urgent));
        if let Some(signal) = given_back.and_then(|at| self.returned.remove(at)) {
            return Some(signal);
        }
        let item = self.urgent.consume()?;
        Signal::from_queued(&item.payload).ok()
    }

    #[must_use]
    pub fn pending(&self) -> u32 {
        let total = self.urgent.len().saturating_add(self.ordinary.len());
        u32::try_from(total)
            .unwrap_or(u32::MAX)
            .saturating_add(u32::try_from(self.returned.len()).unwrap_or(u32::MAX))
    }
}

#[derive(Debug, Clone, Default)]
pub struct Mailslot(std::sync::Arc<std::sync::Mutex<Vec<Signal>>>);

impl Mailslot {

    pub fn drop_in(&self, signal: Signal, capacity: u64) -> Result<Admission, AxError> {
        let mut waiting = self.0.lock().map_err(|_| poisoned())?;
        if u64::try_from(waiting.len()).unwrap_or(u64::MAX) >= capacity {
            return Ok(Admission::Shed {
                reason: kernel::ShedReason::CapacityExhausted,
            });
        }
        waiting.push(signal);
        Ok(Admission::Admit)
    }

    pub fn take(&self) -> Result<Vec<Signal>, AxError> {
        Ok(std::mem::take(&mut *self.0.lock().map_err(|_| poisoned())?))
    }
}

fn poisoned() -> AxError {
    AxError::failure(
        AxCode::StorageFatal,
        "reach a room's mailslot",
        "the slot was left locked by a thread that died",
    )
    .with_recovery("restart this city")
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
