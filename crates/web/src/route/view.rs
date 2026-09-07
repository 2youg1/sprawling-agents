// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The view: which page the content region shows, and its lens.

use channels::{Address, RunId};

use crate::lang::Msg;

///
/// **Six destinations, and two shapes that are reached from them.** The
/// previous set had eleven entries and no page for the one object this
/// product has: a session. Eight of those eleven were a list of
/// sessions, a history of sessions, or settings, and each had a nav
/// entry of its own — so the interface asked a person to choose between
/// eleven answers to a question they had not asked yet.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum View {
    /// The list, and the box that starts work. Default because the
    /// action a person arrives to take is on it.
    #[default]
    Sessions,
    /// One session: a work line in one room, named by the name a person
    /// gave it. The object page this interface never had.
    Session(Address),
    /// Everything that cannot move until a person answers.
    Waiting,
    /// One history, in three lenses. They were three pages with three
    /// nav entries, and no reader ever had to choose between them
    /// before knowing what they were looking for.
    Record(Lens),
    /// Spend, in five cuts.
    Cost,
    /// Where a provider is registered, and which language this reads in.
    /// A region rather than a modal: registering is work, and work that
    /// can be interrupted needs a place to return to.
    Setup,
    /// One building's own files and archive. Reached from a session and
    /// from the city drawing, not from the nav: it is where sessions
    /// live rather than a sixth thing to check.
    Building(Address),
    /// A run named by a link written before sessions had addresses.
    /// Held as its own view because the router is pure and the room a
    /// run is in is a fact only the snapshot has; the page resolves it
    /// and moves on, so this is a state the address bar passes through.
    Run(RunId),
}

/// Which lens the record is read through.
///
/// One page, three lenses, because they are three questions about one
/// history and a person picks the lens after deciding to look — not
/// before. `Bin` is spelled short in the fragment and long on screen:
/// the address bar is typed and the heading is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Lens {
    /// Every record, newest first.
    #[default]
    Ledger,
    /// What was filed on the shelves, and what went there lately.
    Archive,
    /// What was discarded, and the way each row comes back.
    Bin,
}

impl Lens {
    /// Every lens, in the order the page offers them: the whole history,
    /// then what was kept, then what was thrown away.
    pub const ALL: [Lens; 3] = [Lens::Ledger, Lens::Archive, Lens::Bin];

    /// What this lens is called on screen.
    #[must_use]
    pub fn word(self) -> Msg {
        match self {
            Self::Ledger => Msg::RecordLensLedger,
            Self::Archive => Msg::RecordLensArchive,
            Self::Bin => Msg::RecordLensBin,
        }
    }
}
