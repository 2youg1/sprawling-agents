// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, PoisonError};

use kernel::{Address, Admission, AxCode, AxError, RunId};

use super::{INBOX_CAPACITY, new_inbox};

enum RoomQueue {

    Home(collab::Inbox),

    Lent {
        to: RunId,
        slot: collab::Mailslot,
        policy: PolicySlot,
    },
}

#[derive(Clone, Default)]
pub(in crate::worker) struct PolicySlot(Arc<Mutex<Option<kernel::RunPolicy>>>);

impl PolicySlot {

    fn post(&self, policy: kernel::RunPolicy) {

        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = Some(policy);
    }

    pub(in crate::worker) fn take(&self) -> Option<kernel::RunPolicy> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).take()
    }
}

pub(in crate::worker) enum QueueTenure {

    TheRoomQueue,

    ASpare { held_by: RunId },
}

pub(in crate::worker) struct Lent {
    pub(in crate::worker) inbox: collab::Inbox,

    pub(in crate::worker) slot: collab::Mailslot,

    pub(in crate::worker) policy: PolicySlot,
    pub(in crate::worker) tenure: QueueTenure,
}

pub(in crate::worker) struct RoomQueues {
    rooms: BTreeMap<Address, RoomQueue>,
}

impl RoomQueues {

    pub(in crate::worker) fn worked_by(&self, addr: &Address) -> Option<RunId> {
        match self.rooms.get(addr) {
            Some(RoomQueue::Lent { to, .. }) => Some(*to),
            Some(RoomQueue::Home(_)) | None => None,
        }
    }

    pub(in crate::worker) fn worked_within(&self, building: &Address) -> Option<(Address, RunId)> {
        self.rooms.iter().find_map(|(addr, queue)| match queue {
            RoomQueue::Lent { to, .. } if addr.is_within(building) => Some((addr.clone(), *to)),
            RoomQueue::Lent { .. } | RoomQueue::Home(_) => None,
        })
    }

    pub(in crate::worker) fn folded(rooms: BTreeMap<Address, collab::Inbox>) -> RoomQueues {
        RoomQueues {
            rooms: rooms
                .into_iter()
                .map(|(addr, inbox)| (addr, RoomQueue::Home(inbox)))
                .collect(),
        }
    }

    pub(in crate::worker) fn lend(&mut self, addr: &Address, to: RunId) -> Lent {
        match self.rooms.get_mut(addr) {
            Some(RoomQueue::Lent { to: holder, .. }) => Lent {
                inbox: new_inbox(),
                slot: collab::Mailslot::default(),
                policy: PolicySlot::default(),
                tenure: QueueTenure::ASpare { held_by: *holder },
            },
            Some(entry) => {
                let slot = collab::Mailslot::default();
                let policy = PolicySlot::default();
                let lent = std::mem::replace(
                    entry,
                    RoomQueue::Lent {
                        to,
                        slot: slot.clone(),
                        policy: policy.clone(),
                    },
                );
                Lent {
                    slot,
                    policy,
                    inbox: match lent {
                        RoomQueue::Home(inbox) => inbox,

                        RoomQueue::Lent { .. } => new_inbox(),
                    },
                    tenure: QueueTenure::TheRoomQueue,
                }
            }
            None => {
                let slot = collab::Mailslot::default();
                let policy = PolicySlot::default();
                self.rooms.insert(
                    addr.clone(),
                    RoomQueue::Lent {
                        to,
                        slot: slot.clone(),
                        policy: policy.clone(),
                    },
                );
                Lent {
                    inbox: new_inbox(),
                    slot,
                    policy,
                    tenure: QueueTenure::TheRoomQueue,
                }
            }
        }
    }

    pub(in crate::worker) fn give_back(
        &mut self,
        addr: &Address,
        from: RunId,
        mut returned: collab::Inbox,
    ) -> Result<(), AxError> {
        let slot = match self.rooms.get(addr) {
            Some(RoomQueue::Lent { to, slot, .. }) if *to == from => slot.clone(),
            Some(RoomQueue::Lent { to, .. }) => {
                return Err(not_the_holder(addr, from, &format!("{to} is")));
            }
            Some(RoomQueue::Home(_)) | None => {
                return Err(not_the_holder(addr, from, "nobody is"));
            }
        };

        let (waiting, mut refused) = match slot.take() {
            Ok(waiting) => (waiting, None),
            Err(err) => (Vec::new(), Some(err)),
        };
        for signal in &waiting {
            let delivered = returned
                .deliver(signal)
                .and_then(|admission| admitted(admission, signal));
            if let Err(err) = delivered {
                refused = Some(err);
                break;
            }
        }
        self.rooms.insert(addr.clone(), RoomQueue::Home(returned));
        match refused {
            Some(err) => Err(err),
            None => Ok(()),
        }
    }

    pub(in crate::worker) fn deliver(&mut self, signal: &collab::Signal) -> Result<(), AxError> {
        let admission = match self
            .rooms
            .entry(signal.room().clone())
            .or_insert_with(|| RoomQueue::Home(new_inbox()))
        {
            RoomQueue::Home(inbox) => inbox.deliver(signal)?,
            RoomQueue::Lent { slot, .. } => slot.drop_in(signal.clone(), INBOX_CAPACITY)?,
        };
        admitted(admission, signal)
    }

    pub(in crate::worker) fn post_policy(&self, addr: &Address, policy: kernel::RunPolicy) {
        match self.rooms.get(addr) {
            Some(RoomQueue::Lent { policy: slot, .. }) => slot.post(policy),
            Some(RoomQueue::Home(_)) | None => {}
        }
    }

    pub(in crate::worker) fn pending(&self, addr: &Address) -> u32 {
        match self.rooms.get(addr) {
            Some(RoomQueue::Home(inbox)) => inbox.pending(),
            Some(RoomQueue::Lent { .. }) | None => 0,
        }
    }

    pub(in crate::worker) fn waiting(&self) -> BTreeMap<Address, u32> {
        self.rooms
            .keys()
            .map(|addr| (addr.clone(), self.pending(addr)))
            .filter(|(_, pending)| *pending > 0)
            .collect()
    }
}

#[cfg(test)]
impl RoomQueues {

    pub(in crate::worker) fn at_home(&self, addr: &Address) -> Option<&collab::Inbox> {
        match self.rooms.get(addr) {
            Some(RoomQueue::Home(inbox)) => Some(inbox),
            Some(RoomQueue::Lent { .. }) | None => None,
        }
    }

    pub(in crate::worker) fn pull_at_home(
        &mut self,
        addr: &Address,
    ) -> Result<Vec<collab::Signal>, AxError> {
        match self.rooms.get_mut(addr) {
            Some(RoomQueue::Home(inbox)) => inbox.pull(),
            Some(RoomQueue::Lent { .. }) | None => Ok(Vec::new()),
        }
    }

    pub(in crate::worker) fn queued(&self) -> BTreeMap<Address, u32> {
        self.rooms
            .iter()
            .filter_map(|(addr, queue)| match queue {
                RoomQueue::Home(inbox) if inbox.pending() > 0 => {
                    Some((addr.clone(), inbox.pending()))
                }
                RoomQueue::Home(_) | RoomQueue::Lent { .. } => None,
            })
            .collect()
    }
}

fn admitted(admission: Admission, signal: &collab::Signal) -> Result<(), AxError> {
    match admission {
        Admission::Admit => Ok(()),
        Admission::Shed { .. } => Err(AxError::failure(
            AxCode::BackpressureShed,
            "deliver a signal that waited for a room",
            format!("room {} shed the signal", signal.room()),
        )
        .with_recovery("the queue is full; the speaker retries when the room drains")),
    }
}

#[cfg(test)]
mod tests;

fn not_the_holder(addr: &Address, from: RunId, holder: &str) -> AxError {
    AxError::failure(
        AxCode::StorageFatal,
        "give a room's queue back",
        format!("{from} offered the queue of {addr}, and {holder} holding it"),
    )
    .with_recovery("report this: a room's queue is lent to one run and returned by that run")
}
