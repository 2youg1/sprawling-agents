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
//! no trace in memory (sprawling-SPEC.md 8-46-9).
//!
//! Three facts make that unrepresentable here. A lent queue is still in
//! the table, marked with the run that took it, so a second borrower is
//! answered rather than served. Only the run named on the mark may give
//! a queue back. And what is delivered to a room while its queue is out
//! waits in the same entry, so the holder takes it home rather than
//! somebody discovering it was never held.

use std::collections::BTreeMap;

use kernel::{Address, Admission, AxCode, AxError, RunId, ShedReason};

use super::{INBOX_CAPACITY, new_inbox};

/// One room's queue, and where it is.
enum RoomQueue {
    /// Nobody is working in this room, so the queue is here.
    Home(collab::Inbox),
    /// One run holds this room's queue at its signal desk, and what
    /// arrived since waits to be handed over when that run lands.
    Lent {
        to: RunId,
        waiting: Vec<collab::Signal>,
    },
}

/// Whether the run that asked holds its room's queue.
///
/// A dispatch keeps this receipt for as long as it drives and shows it
/// when it lands: the run named on the queue is the only one that may
/// give it back, and a run that never held it has nothing to return.
pub(in crate::assembly) enum Holding {
    /// This run holds the room's queue.
    TheRoomQueue,
    /// Another run in the same room holds it. This one reads a queue of
    /// its own that nothing is delivered into: a room has one reader,
    /// and two runs pulling from one queue would each see half the mail.
    ASpare { held_by: RunId },
}

/// What a dispatch was given when it asked for its room's queue.
pub(in crate::assembly) struct Lent {
    pub(in crate::assembly) inbox: collab::Inbox,
    pub(in crate::assembly) holding: Holding,
}

/// What is waiting for each room, and which run is reading it.
pub(in crate::assembly) struct RoomQueues {
    rooms: BTreeMap<Address, RoomQueue>,
}

impl RoomQueues {
    /// The queues a history folds to. Nothing is lent when a worker
    /// opens: a run that was driving when the process stopped is not
    /// driving now.
    pub(in crate::assembly) fn folded(rooms: BTreeMap<Address, collab::Inbox>) -> RoomQueues {
        RoomQueues {
            rooms: rooms
                .into_iter()
                .map(|(addr, inbox)| (addr, RoomQueue::Home(inbox)))
                .collect(),
        }
    }

    /// Lends `to` the queue of the room at `addr`, or a spare when
    /// another run is already reading there.
    pub(in crate::assembly) fn lend(&mut self, addr: &Address, to: RunId) -> Lent {
        match self.rooms.get_mut(addr) {
            Some(RoomQueue::Lent { to: holder, .. }) => Lent {
                inbox: new_inbox(),
                holding: Holding::ASpare { held_by: *holder },
            },
            Some(entry) => {
                let lent = std::mem::replace(
                    entry,
                    RoomQueue::Lent {
                        to,
                        waiting: Vec::new(),
                    },
                );
                Lent {
                    inbox: match lent {
                        RoomQueue::Home(inbox) => inbox,
                        // Unreachable by the arm above, and answered
                        // with a queue rather than a panic: the cost of
                        // being wrong here is one room's mail, and the
                        // cost of being wrong the other way is the city.
                        RoomQueue::Lent { .. } => new_inbox(),
                    },
                    holding: Holding::TheRoomQueue,
                }
            }
            None => {
                self.rooms.insert(
                    addr.clone(),
                    RoomQueue::Lent {
                        to,
                        waiting: Vec::new(),
                    },
                );
                Lent {
                    inbox: new_inbox(),
                    holding: Holding::TheRoomQueue,
                }
            }
        }
    }

    /// Takes the queue of the room at `addr` back from `from`, and
    /// delivers into it everything that arrived while it was out.
    ///
    /// # Errors
    /// Refuses a return from a run that was not holding the queue, and
    /// propagates the queue's own refusal of a signal that waited.
    pub(in crate::assembly) fn give_back(
        &mut self,
        addr: &Address,
        from: RunId,
        mut returned: collab::Inbox,
    ) -> Result<(), AxError> {
        let waiting = match self.rooms.get_mut(addr) {
            Some(RoomQueue::Lent { to, waiting }) if *to == from => std::mem::take(waiting),
            Some(RoomQueue::Lent { to, .. }) => {
                return Err(not_the_holder(addr, from, &format!("{to} is")));
            }
            Some(RoomQueue::Home(_)) | None => {
                return Err(not_the_holder(addr, from, "nobody is"));
            }
        };
        // The queue comes home whatever the signals that waited do to
        // it: this method exists because a queue that failed on its way
        // back used to be a queue the city forgot it had.
        let mut refused = None;
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
    pub(in crate::assembly) fn deliver(&mut self, signal: &collab::Signal) -> Result<(), AxError> {
        let admission = match self
            .rooms
            .entry(signal.room().clone())
            .or_insert_with(|| RoomQueue::Home(new_inbox()))
        {
            RoomQueue::Home(inbox) => inbox.deliver(signal)?,
            RoomQueue::Lent { waiting, .. } => {
                if u64::try_from(waiting.len()).unwrap_or(u64::MAX) >= INBOX_CAPACITY {
                    Admission::Shed {
                        reason: ShedReason::CapacityExhausted,
                    }
                } else {
                    waiting.push(signal.clone());
                    Admission::Admit
                }
            }
        };
        admitted(admission, signal)
    }

    /// How many signals the room at `addr` is holding for a reader that
    /// is not already in it.
    ///
    /// A room whose queue is lent reports nothing: what is waiting is in
    /// the holder's own desk, and counting it here would tell a
    /// neighbour that mail is unread when somebody is reading it.
    pub(in crate::assembly) fn pending(&self, addr: &Address) -> u32 {
        match self.rooms.get(addr) {
            Some(RoomQueue::Home(inbox)) => inbox.pending(),
            Some(RoomQueue::Lent { .. }) | None => 0,
        }
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
    pub(in crate::assembly) fn at_home(&self, addr: &Address) -> Option<&collab::Inbox> {
        match self.rooms.get(addr) {
            Some(RoomQueue::Home(inbox)) => Some(inbox),
            Some(RoomQueue::Lent { .. }) | None => None,
        }
    }

    /// Takes up to one pull's worth out of the queue at `addr`.
    ///
    /// # Errors
    /// Propagates a queued payload that does not read back as a signal.
    pub(in crate::assembly) fn pull_at_home(
        &mut self,
        addr: &Address,
    ) -> Result<Vec<collab::Signal>, AxError> {
        match self.rooms.get_mut(addr) {
            Some(RoomQueue::Home(inbox)) => inbox.pull(),
            Some(RoomQueue::Lent { .. }) | None => Ok(Vec::new()),
        }
    }

    /// How much is waiting in every room that has anything waiting.
    pub(in crate::assembly) fn queued(&self) -> BTreeMap<Address, u32> {
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
