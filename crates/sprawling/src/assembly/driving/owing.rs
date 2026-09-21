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
//! Nothing here reaches the ledger or a lane. It is the vocabulary the
//! table in `flight` carries and the landing in `settling` spends, so
//! neither of those has to know why a run exists.

use std::sync::Arc;

use kernel::{Address, NodeId};

/// What the city still owes once a run has landed.
///
/// **Every entrance puts its run in a lane, and they differ only
/// here** — which is what makes "a dispatch is prepared, driven and
/// landed" one path rather than seven copies of one (sprawling-SPEC.md
/// 8-46-2). Exhaustive: an entrance added without deciding what its
/// landing owes is a compile error rather than a run nobody collects.
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
/// The two travel together because an obligation outlives the run that
/// carried it: a successor answers to whoever asked for the run it
/// replaced, and a run the city started itself answers to nobody —
/// which `channels::Reply::nowhere` already spells, so there is one
/// reply path rather than one for people and one for the city.
pub(in crate::assembly) struct Owing {
    owed: Owed,
    /// Shared rather than moved: the landing hands a refusal back after
    /// it has already given the obligation to a successor.
    reply: Arc<channels::Reply>,
}

impl Owing {
    /// A person asked, and a refusal goes back to them.
    pub(in crate::assembly) fn asked(reply: channels::Reply) -> Owing {
        Owing {
            owed: Owed::Asked,
            reply: Arc::new(reply),
        }
    }

    /// The city started this run itself, so a refusal has nobody to go
    /// back to and is noted where the landing happens.
    pub(in crate::assembly) fn unasked(because: Unasked) -> Owing {
        Owing {
            owed: Owed::Unasked(because),
            reply: Arc::new(channels::Reply::nowhere()),
        }
    }

    /// A pursuit took this plan row.
    pub(in crate::assembly) fn row(addr: Address, node: NodeId) -> Owing {
        Owing {
            owed: Owed::Row { addr, node },
            reply: Arc::new(channels::Reply::nowhere()),
        }
    }

    /// A run handed this work down, and the reply address of whoever
    /// asked for the parent carries on to the child.
    pub(in crate::assembly) fn child(&self, parent: Address) -> Owing {
        Owing {
            owed: Owed::Child { parent },
            reply: Arc::clone(&self.reply),
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
}
