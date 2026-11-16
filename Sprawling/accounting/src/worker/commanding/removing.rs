// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{Address, AxCode, AxError, EventKind};

use super::super::RunWorker;

impl RunWorker {

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
