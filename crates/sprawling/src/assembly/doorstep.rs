// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What reached the city's door and has not yet become a run
//! (sprawling-SPEC.md 8-91).

use super::{Entrance, Knock};

/// The work that has arrived and is still outside: the commands this
/// city has answered by key, and the residents spoken to while nobody
/// was home.
pub(in crate::assembly) struct Doorstep {
    /// Every command key this city has answered, and what it answered.
    /// Folded from the history, so a client retrying across a restart
    /// is still asking for one thing.
    pub(in crate::assembly) entrance: Entrance,
    /// Residents who were spoken to while nobody was home. Held between
    /// the run that spoke and the runs that answer, because delivery
    /// happens after the speaker has frozen.
    pub(in crate::assembly) knocks: Vec<Knock>,
}

impl Doorstep {
    /// The doorstep of a worker that has just opened: the keys its
    /// history already answered, and nothing else yet.
    pub(in crate::assembly) fn opened(entrance: Entrance) -> Doorstep {
        Doorstep {
            entrance,
            knocks: Vec::new(),
        }
    }
}
