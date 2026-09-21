// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What the city owes one run in a lane, and where a refusal goes.
//!
//! **The seven dispatch entrances differ only here.** A person's
//! command, a schedule firing, an arrival from outside, a knock, an
//! answered approval, work handed down and a succession all take the
//! same prepare / drive / land path; what each landing is owed is this
//! value, and settling it is one match (sprawling-SPEC.md 8-46-2).
//!
//! **Two chains can start a run with nobody in front of them, and both
//! are bounded here.** A knock wakes the next resident in a
//! conversation, and a succession hands one piece of work to the next
//! run; either chain can go on spending model calls for ever unless the
//! city counts the hops and refuses the one that would exceed its
//! ceiling (sprawling-SPEC.md 8-46-12). The counts ride the obligation,
//! not the worker: an obligation outlives the run that carried it, and a
//! field on the worker would describe whichever run happened to be
//! landing.
//!
//! Nothing here reaches the ledger or a lane. It is the vocabulary the
//! table in `flight` carries and the landing in `settling` spends, so
//! neither of those has to know why a run exists.

use std::sync::Arc;

use kernel::{Address, AxCode, AxError, NodeId};

/// What the city still owes once a run has landed.
///
/// **Every entrance puts its run in a lane, and they differ only
/// here** — which is what makes "a dispatch is prepared, driven and
/// landed" one path rather than seven copies of one (sprawling-SPEC.md
/// 8-46-2). Exhaustive: an entrance added without deciding what its
/// landing owes is a compile error rather than a run nobody collects.
#[derive(Clone)]
pub(in crate::assembly) enum Owed {
    /// A person asked for this dispatch, and whoever the run spoke to
    /// answers next.
    Asked,
    /// A pursuit took this plan row. If the row is still ready when the
    /// run comes home, the pursuit stops rather than taking it again.
    Row { addr: Address, node: NodeId },
    /// The city started this run itself. Nothing is owed but the
    /// landing, and the reason is carried so that a landing which fails
    /// is reported against what started it.
    Unasked(Unasked),
    /// A run handed this work down. The room that asked is told how it
    /// came back, which is the handback a `FanIn` is folded from.
    Child { parent: Address },
}

/// Why the city started a run nobody typed a command for.
///
/// Four reasons rather than one flag, because each one names a
/// different person to go back to when the run cannot be started: the
/// schedule file, the watch table, a resident, or an answered approval.
#[derive(Clone, Copy)]
pub(in crate::assembly) enum Unasked {
    /// The schedule said this job was due.
    Schedule,
    /// Something arrived from outside and the watch table routed it
    /// here.
    Arrival,
    /// A neighbour signalled a resident who was not working.
    Knock,
    /// A person answered the approval this work was waiting on.
    Unblocked,
}

impl Unasked {
    /// One clause a diagnostic line ends with.
    pub(in crate::assembly) fn because(self) -> &'static str {
        match self {
            Unasked::Schedule => "the schedule said it was due",
            Unasked::Arrival => "something arrived from outside",
            Unasked::Knock => "a neighbour signalled this resident",
            Unasked::Unblocked => "a person answered the approval it was waiting on",
        }
    }
}

/// What the city owes one run in a lane, and where a refusal goes.
///
/// The three travel together because an obligation outlives the run that
/// carried it: a successor answers to whoever asked for the run it
/// replaced, and it inherits that run's place in the chain. `reply` is
/// shared rather than moved because the landing hands a refusal back
/// after it has already given the obligation to a successor.
pub(in crate::assembly) struct Owing {
    owed: Owed,
    reply: Arc<channels::Reply>,
    relays: Relays,
}

/// How far one chain has carried a piece of work.
///
/// Two counters rather than one because the two chains answer different
/// questions: how many residents a conversation has already woken, and
/// how many times one piece of work has been handed over. A run is on
/// both journeys: it may be the fourth knock of a conversation and the
/// ninth successor of its work.
#[derive(Clone, Copy, Default)]
struct Relays {
    conversations: u32,
    successions: u32,
}

/// The most knocks one conversation may carry. A conversation that has
/// woken sixteen residents in a row is a ring rather than a dialogue,
/// and every hop is a run nobody asked for (sprawling-SPEC.md 8-46-12).
const CONVERSATION_HOPS_MAX: u32 = 16;

/// The most times one piece of work may be handed to a successor. A run
/// may ask to be replaced while it still has work to do, so the ceiling
/// is far above any real task: it exists to make the chain finite, not
/// to price the work (sprawling-SPEC.md 8-46-12).
const SUCCESSION_HOPS_MAX: u32 = 64;

impl Relays {
    /// The place of a run one knock further along, or the refusal when
    /// the conversation has already gone as far as it may.
    fn after_knock(&self) -> Result<Relays, AxError> {
        let conversations = self
            .conversations
            .checked_add(1)
            .filter(|next| *next <= CONVERSATION_HOPS_MAX)
            .ok_or_else(|| {
                AxError::failure(
                    AxCode::LoopSuspected,
                    "wake the resident a signal was sent to",
                    format!("a conversation already {} knocks deep", self.conversations),
                )
                .with_recovery(
                    "read the signal already in the room's inbox, or dispatch the work to that \
                     resident by hand; the city will not open another run in this chain",
                )
            })?;
        Ok(Relays {
            conversations,
            successions: self.successions,
        })
    }

    /// The place of the successor that takes this obligation over, or
    /// the refusal when the work has already been handed on as often as
    /// it may be.
    fn after_succession(&self) -> Result<Relays, AxError> {
        let successions = self
            .successions
            .checked_add(1)
            .filter(|next| *next <= SUCCESSION_HOPS_MAX)
            .ok_or_else(|| {
                AxError::failure(
                    AxCode::LoopSuspected,
                    "hand this work to a successor",
                    format!(
                        "a run already {} successions into one piece of work",
                        self.successions
                    ),
                )
                .with_recovery(
                    "let this run's ending stand, or dispatch the work again; the city will not \
                     hand one piece of work over for ever",
                )
            })?;
        Ok(Relays {
            conversations: self.conversations,
            successions,
        })
    }
}

impl Owing {
    /// A person asked, and a refusal goes back to them.
    pub(in crate::assembly) fn asked(reply: channels::Reply) -> Owing {
        Owing {
            owed: Owed::Asked,
            reply: Arc::new(reply),
            relays: Relays::default(),
        }
    }

    /// The city started this run itself, so a refusal has nobody to go
    /// back to and is noted where the landing happens.
    pub(in crate::assembly) fn unasked(because: Unasked) -> Owing {
        Owing {
            owed: Owed::Unasked(because),
            reply: Arc::new(channels::Reply::nowhere()),
            relays: Relays::default(),
        }
    }

    /// A knock started this run. The conversation moves one hop on, and
    /// a chain already at its ceiling is refused here rather than opened
    /// and abandoned (sprawling-SPEC.md 8-46-12).
    ///
    /// `conversations` is how many knocks deep the run that spoke was.
    ///
    /// # Errors
    /// Refuses with `E_LOOP_SUSPECTED` when the knock would exceed
    /// `CONVERSATION_HOPS_MAX`.
    pub(in crate::assembly) fn knocked(conversations: u32) -> Result<Owing, AxError> {
        Ok(Owing {
            owed: Owed::Unasked(Unasked::Knock),
            reply: Arc::new(channels::Reply::nowhere()),
            relays: Relays {
                conversations,
                successions: 0,
            }
            .after_knock()?,
        })
    }

    /// A pursuit took this plan row.
    pub(in crate::assembly) fn row(addr: Address, node: NodeId) -> Owing {
        Owing {
            owed: Owed::Row { addr, node },
            reply: Arc::new(channels::Reply::nowhere()),
            relays: Relays::default(),
        }
    }

    /// A run handed this work down, and the reply address of whoever
    /// asked for the parent carries on to the child.
    ///
    /// A delegate is a different piece of work rather than another hop
    /// of the conversation that asked for it, so the child inherits the
    /// chain's place without moving it.
    pub(in crate::assembly) fn child(&self, parent: Address) -> Owing {
        Owing {
            owed: Owed::Child { parent },
            reply: Arc::clone(&self.reply),
            relays: self.relays,
        }
    }

    /// What this run owes, read when it lands.
    pub(in crate::assembly) fn owed(&self) -> &Owed {
        &self.owed
    }

    /// Where a refusal goes back to.
    pub(in crate::assembly) fn reply(&self) -> Arc<channels::Reply> {
        Arc::clone(&self.reply)
    }

    /// How many knocks deep in its conversation this run is. Read by the
    /// landing so a signal this run sends carries the chain's place on.
    pub(in crate::assembly) fn conversations(&self) -> u32 {
        self.relays.conversations
    }

    /// The same obligation one succession further along, or the refusal
    /// when the work has been handed over as often as it may be.
    ///
    /// # Errors
    /// Refuses with `E_LOOP_SUSPECTED` when the successor would exceed
    /// `SUCCESSION_HOPS_MAX`.
    pub(in crate::assembly) fn after_succession(&self) -> Result<Owing, AxError> {
        Ok(Owing {
            owed: self.owed.clone(),
            reply: Arc::clone(&self.reply),
            relays: self.relays.after_succession()?,
        })
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests {
    use super::*;

    /// The boundary is exactly the ceiling: one below it carries on,
    /// reaching it is refused with a recovery line, and the code names
    /// the suspicion rather than a generic failure.
    #[test]
    fn a_chain_of_successions_stops_at_its_ceiling() {
        let mut owing = Owing::asked(channels::Reply::nowhere());
        for _ in 0..SUCCESSION_HOPS_MAX {
            owing = match owing.after_succession() {
                Ok(next) => next,
                Err(refusal) => panic!("a succession below the ceiling was refused: {refusal}"),
            };
        }
        let refusal = owing
            .after_succession()
            .err()
            .expect("a succession at the ceiling must be refused");
        assert_eq!(refusal.code(), &AxCode::LoopSuspected);
        assert!(!refusal.recovery().is_empty());
    }

    /// The same boundary on the conversation chain, with the speaking
    /// run's place as the input.
    #[test]
    fn a_knock_chain_stops_at_its_ceiling() {
        let owed = match Owing::knocked(CONVERSATION_HOPS_MAX - 1) {
            Ok(owed) => owed,
            Err(refusal) => panic!("a knock below the ceiling was refused: {refusal}"),
        };
        assert_eq!(owed.conversations(), CONVERSATION_HOPS_MAX);
        let refusal = Owing::knocked(CONVERSATION_HOPS_MAX)
            .err()
            .expect("a knock at the ceiling must be refused");
        assert_eq!(refusal.code(), &AxCode::LoopSuspected);
        assert!(!refusal.recovery().is_empty());
    }

    /// A delegate inherits the conversation it came from.
    #[test]
    fn a_delegated_child_inherits_the_chain_it_came_from() {
        let parent = match Owing::knocked(3) {
            Ok(parent) => parent,
            Err(refusal) => panic!("a knock below the ceiling was refused: {refusal}"),
        };
        let child = parent.child(Address::parse("market/hana").unwrap());
        assert_eq!(child.conversations(), parent.conversations());
    }
}
