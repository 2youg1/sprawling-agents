// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Every room's queue, and who is holding it.
//!
//! **A room has exactly one queue, and this module is what enforces
//! it.** The rule was written down before it was held: a dispatch used
//! to lift its room's queue out of a plain map and put a queue back
//! when it landed, so two runs in one room each took a queue, each
//! filled it, and the one that landed second wrote over the one that
//! landed first — signals the ledger already recorded as enqueued left
//! no trace in memory (`crates/sprawling/Spec.lean` §8-46-9).
//!
//! Three facts make that unrepresentable here. A lent queue is still in
//! the table, marked with the run that took it, so a second borrower is
//! answered rather than served. Only the run named on the mark may give
//! a queue back. And what is delivered to a room while its queue is out
//! goes into the slot the entry shares with the holder's desk, so the
//! holder reads it at its next safe point (collab D7) and takes home
//! whatever it did not reach, rather than somebody discovering it was
//! never held.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, PoisonError};

use kernel::{Address, Admission, AxCode, AxError, RunId};

use super::{INBOX_CAPACITY, new_inbox};

/// One room's queue, and where it is.
enum RoomQueue {
    /// Nobody is working in this room, so the queue is here.
    Home(collab::Inbox),
    /// One run holds this room's queue at its signal desk, and what
    /// arrives meanwhile is dropped into the slot that desk empties at
    /// each safe point.
    Lent {
        to: RunId,
        slot: collab::Mailslot,
        policy: PolicySlot,
    },
}

/// Where a run policy the User chose for a room waits for the run
/// working there, beside the room's mailslot (`crates/runtime/spec/PolicyTake.lean`
/// §8-62).
///
/// One writer, [`RoomQueues::post_policy`], on the accounting thread;
/// one reader, the lane's interrupt source, which hands what it takes to
/// the run as `Interrupt::Policy` at its next safe point. It holds the
/// last change only: a later one overrides an earlier one the run has
/// not read, which is what the run's own cell would do with both.
#[derive(Clone, Default)]
pub(in crate::worker) struct PolicySlot(Arc<Mutex<Option<kernel::RunPolicy>>>);

impl PolicySlot {
    /// Puts `policy` in the slot, over whatever waited there.
    fn post(&self, policy: kernel::RunPolicy) {
        // A poisoned lock still holds a whole policy: the only write is
        // the assignment of a `Copy` value, which cannot stop halfway.
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = Some(policy);
    }

    /// Takes the change waiting for the run, if one is.
    pub(in crate::worker) fn take(&self) -> Option<kernel::RunPolicy> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).take()
    }
}

/// This run's tenure over its room's queue.
///
/// A dispatch keeps this receipt for as long as it drives and shows it
/// when it lands: the run named on the queue is the only one that may
/// give it back, and a run that never held it has nothing to return.
pub(in crate::worker) enum QueueTenure {
    /// This run holds the room's queue.
    TheRoomQueue,
    /// Another run in the same room holds it. This one reads a queue of
    /// its own that nothing is delivered into: a room has one reader,
    /// and two runs pulling from one queue would each see half the mail.
    ASpare { held_by: RunId },
}

/// What a dispatch was given when it asked for its room's queue.
pub(in crate::worker) struct Lent {
    pub(in crate::worker) inbox: collab::Inbox,
    /// Where the city drops what arrives for the room while the run
    /// reads; a spare's slot is one nothing is dropped into.
    pub(in crate::worker) slot: collab::Mailslot,
    /// Where a change of the room's run policy waits for this run; a
    /// spare's is one nothing is posted into.
    pub(in crate::worker) policy: PolicySlot,
    pub(in crate::worker) tenure: QueueTenure,
}

/// What is waiting for each room, and which run is reading it.
pub(in crate::worker) struct RoomQueues {
    rooms: BTreeMap<Address, RoomQueue>,
}

impl RoomQueues {
    /// Which run is working in this room right now, if one is.
    ///
    /// A room's queue is what says so, and this is the same fact
    /// [`RoomQueues::lend`] already acts on rather than a second book
    /// of who is busy: a dispatch lends its room's queue for as long as
    /// it drives, so a lent entry *is* a run at work there, and a room
    /// whose queue is home has nobody in it. A second run in one room
    /// holds a spare, and the holder named here is still the one that
    /// answers - the entry it took is what the spare was issued against
    /// (`crates/sprawling/Spec.lean` §8-46-9).
    pub(in crate::worker) fn worked_by(&self, addr: &Address) -> Option<RunId> {
        match self.rooms.get(addr) {
            Some(RoomQueue::Lent { to, .. }) => Some(*to),
            Some(RoomQueue::Home(_)) | None => None,
        }
    }

    /// A room inside `building` whose queue is lent, and the run it is
    /// lent to: the one fact that says a run is working in the
    /// building. Every room is looked at, because address order puts
    /// `lab-2` between `lab` and `lab/room1`.
    pub(in crate::worker) fn worked_within(&self, building: &Address) -> Option<(Address, RunId)> {
        self.rooms.iter().find_map(|(addr, queue)| match queue {
            RoomQueue::Lent { to, .. } if addr.is_within(building) => Some((addr.clone(), *to)),
            RoomQueue::Lent { .. } | RoomQueue::Home(_) => None,
        })
    }

    /// The queues a history folds to. Nothing is lent when a worker
    /// opens: a run that was driving when the process stopped is not
    /// driving now.
    pub(in crate::worker) fn folded(rooms: BTreeMap<Address, collab::Inbox>) -> RoomQueues {
        RoomQueues {
            rooms: rooms
                .into_iter()
                .map(|(addr, inbox)| (addr, RoomQueue::Home(inbox)))
                .collect(),
        }
    }

    /// Lends `to` the queue of the room at `addr`, or a spare when
    /// another run is already reading there.
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
                        // Unreachable by the arm above, and answered
                        // with a queue rather than a panic: the cost of
                        // being wrong here is one room's mail, and the
                        // cost of being wrong the other way is the city.
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

    /// Takes the queue of the room at `addr` back from `from`, and
    /// delivers into it whatever is still in the slot: what arrived
    /// after the holder's desk last emptied it.
    ///
    /// # Errors
    /// Refuses a return from a run that was not holding the queue, and
    /// propagates the queue's own refusal of a signal that waited.
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
        // The queue comes home whatever the signals that waited do to
        // it: a queue that failed on its way back must not become a
        // queue the city forgets it has.
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

    /// Delivers one signal to the room it names, whether or not a run
    /// is reading there.
    ///
    /// **A shed delivery is a refusal, not a quiet drop.** The line is
    /// already on the ledger by the time this is called, and a line no
    /// queue holds is a city that disagrees with its own history.
    ///
    /// # Errors
    /// Refuses when the room is full, and when a room whose queue is
    /// out has been spoken to more times than a queue holds.
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

    /// Hands a new run policy for the room at `addr` to the run working
    /// there, if one is; a room nobody works in keeps it on the ledger
    /// alone, where the next dispatch reads it.
    pub(in crate::worker) fn post_policy(&self, addr: &Address, policy: kernel::RunPolicy) {
        match self.rooms.get(addr) {
            Some(RoomQueue::Lent { policy: slot, .. }) => slot.post(policy),
            Some(RoomQueue::Home(_)) | None => {}
        }
    }

    /// How many signals the room at `addr` is holding for a reader that
    /// is not already in it.
    ///
    /// A room whose queue is lent reports nothing: what is waiting is in
    /// the holder's own desk, and counting it here would tell a
    /// neighbour that mail is unread when somebody is reading it.
    pub(in crate::worker) fn pending(&self, addr: &Address) -> u32 {
        match self.rooms.get(addr) {
            Some(RoomQueue::Home(inbox)) => inbox.pending(),
            Some(RoomQueue::Lent { .. }) | None => 0,
        }
    }

    /// [`RoomQueues::pending`] for every room holding anything, read at
    /// once, for a bench laid out off this thread (`crates/sprawling/Spec.lean` §8-113).
    pub(in crate::worker) fn waiting(&self) -> BTreeMap<Address, u32> {
        self.rooms
            .keys()
            .map(|addr| (addr.clone(), self.pending(addr)))
            .filter(|(_, pending)| *pending > 0)
            .collect()
    }
}

/// The read faces the tests assert through.
///
/// A settled city has every queue at home, which is the state these
/// answer about; production reads what is waiting through
/// [`RoomQueues::pending`] and takes signals through the signal desk,
/// so nothing here widens what a run can reach.
#[cfg(test)]
impl RoomQueues {
    /// The queue of the room at `addr`, if nobody is holding it.
    pub(in crate::worker) fn at_home(&self, addr: &Address) -> Option<&collab::Inbox> {
        match self.rooms.get(addr) {
            Some(RoomQueue::Home(inbox)) => Some(inbox),
            Some(RoomQueue::Lent { .. }) | None => None,
        }
    }

    /// Takes up to one pull's worth out of the queue at `addr`.
    ///
    /// # Errors
    /// Propagates a queued payload that does not read back as a signal.
    pub(in crate::worker) fn pull_at_home(
        &mut self,
        addr: &Address,
    ) -> Result<Vec<collab::Signal>, AxError> {
        match self.rooms.get_mut(addr) {
            Some(RoomQueue::Home(inbox)) => inbox.pull(),
            Some(RoomQueue::Lent { .. }) | None => Ok(Vec::new()),
        }
    }

    /// How much is waiting in every room that has anything waiting.
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

/// Turns a shed admission into the refusal the caller propagates.
///
/// One spelling for both halves of a delivery — the queue's own and the
/// holding pen's — because "the room would not take it" is one fact.
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

/// What a return from a run that never borrowed is refused with.
fn not_the_holder(addr: &Address, from: RunId, holder: &str) -> AxError {
    AxError::failure(
        AxCode::StorageFatal,
        "give a room's queue back",
        format!("{from} offered the queue of {addr}, and {holder} holding it"),
    )
    .with_recovery("report this: a room's queue is lent to one run and returned by that run")
}
