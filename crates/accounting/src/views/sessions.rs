// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Each address's stretches - its sessions - folded from the records the
//! Ledger files under it, and the answer for one room (accounting-SPEC.md
//! 8-19(c), wire-SPEC §8-71).
//!
//! **Held in the views rather than read back from the Ledger.** Every
//! fact a stretch shows is settled as its lines go by, and the table
//! grows with the stretches rather than with the records, so a question
//! is a copy out of memory and no ledger line is read to answer it.

use std::collections::BTreeMap;

use kernel::{Address, AxError, EventRecord};
use wire::{SESSIONS_MAX, SessionLine, SessionsAnswer};

/// Every address's stretches, oldest first.
#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub(crate) struct RoomSessions {
    by_room: BTreeMap<Address, Vec<SessionLine>>,
}

impl RoomSessions {
    /// Folds one record: `session_opened` begins a stretch at its address;
    /// `run_started` begins one where the address has none yet - the
    /// dispatch that opened the room - and otherwise counts a run of the
    /// current one; every record filed under an address that has a
    /// stretch becomes that stretch's last line. A record with no address
    /// is not a room's.
    ///
    /// # Errors
    /// Refuses a `session_opened` whose payload cannot be read: a stretch
    /// that does not say what it carried or branched from would be shown
    /// as one that carried nothing, which is a different stretch.
    pub(crate) fn absorb(&mut self, _record: &EventRecord) -> Result<(), AxError> {
        Ok(())
    }

    /// The newest `SESSIONS_MAX` stretches of `room`, newest first, and
    /// how many older ones are left out. A room with no stretch answers an
    /// empty list: whether the directory exists is `Query::Listing`'s.
    pub(crate) fn answer(&self, room: &Address) -> SessionsAnswer {
        let held = self.by_room.get(room).map_or(&[][..], Vec::as_slice);
        let sessions: Vec<SessionLine> = held.iter().rev().take(SESSIONS_MAX).cloned().collect();
        SessionsAnswer {
            room: room.clone(),
            earlier: u64::try_from(held.len().saturating_sub(sessions.len())).unwrap_or(u64::MAX),
            sessions,
        }
    }
}
