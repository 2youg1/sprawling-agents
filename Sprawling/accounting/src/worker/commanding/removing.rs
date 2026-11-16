// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Removing a building: the refusal while a run works in it, and the
//! record that says where its files went.
//!
//! The move itself is `city::remove_building`'s, which keeps every file
//! under the reserved subtree. What the city owns is the two facts that
//! module cannot know: whether a run is working in one of the building's
//! rooms, and what the Ledger records. Everything the building wrote
//! before stays in that history; `building_removed` is appended after it
//! and deletes nothing (`crates/city/Spec.lean` §8-3).

use kernel::{Address, AxCode, AxError, EventKind};

use super::super::RunWorker;

impl RunWorker {
    /// Takes the building at `addr` out of the city.
    ///
    /// # Errors
    /// Refuses `E_BUSY` while a run is driving in one of the building's
    /// rooms, naming the room and the run so the caller can stop it;
    /// propagates what `city::remove_building` refuses and a ledger that
    /// refuses the record. The files move before the record is written,
    /// because the record says they moved.
    pub(in crate::worker) fn remove_building(&mut self, addr: &Address) -> Result<(), AxError> {
        if let Some((room, working)) = self.collaborating.rooms.worked_within(addr) {
            return Err(AxError::failure(
                AxCode::Busy,
                "remove a building",
                format!(
                    "{}: {working} is running in {}",
                    addr.as_str(),
                    room.as_str()
                ),
            )
            .with_recovery(
                "stop that run first, then remove the building; a building cannot move out \
                 from under the run writing in it",
            ));
        }
        let removed = city::remove_building(&self.city_root, addr)?;
        self.note(
            runtime::diagnostics::Level::Effect,
            "city::building",
            &format!(
                "{} removed; its files are kept at {}",
                addr.as_str(),
                removed.kept()
            ),
        );
        let payload = city::building_removed_payload(&removed)?;
        self.record(EventKind::BuildingRemoved, payload)
    }
}

#[cfg(test)]
mod tests;
