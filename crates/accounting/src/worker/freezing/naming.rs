// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The names one session is frozen with (accounting-SPEC.md 8-15): what
//! the city calls the person and the Mayor, in the city slot and at the
//! head of the resident slot.

use std::path::Path;

use kernel::{Address, AxError, B3Hash};

use super::{Assembled, NEWLINE, city_segment};

/// The names a run is frozen with, and the version a page reads them
/// back by.
pub(super) struct Frozen {
    naming: city::Naming,
    pub(super) version: B3Hash,
}

/// The names the session at `room` runs under.
///
/// # Errors
/// Propagates an identity area that does not read and a store that will
/// not take the bytes.
pub(super) fn session_naming(
    city_root: &Path,
    cas: &mut storage::Cas,
    _room: &Address,
) -> Result<Frozen, AxError> {
    let naming = city::Naming::read(city_root)?;
    let version = cas
        .put(&naming.to_bytes()?)
        .map_err(storage::StorageError::into_ax)?;
    Ok(Frozen { naming, version })
}

impl Frozen {
    /// The city slot: the city's norms, then who works here, which is the
    /// same for every resident of the city and so stays in the one slot
    /// cached across all of them.
    ///
    /// # Errors
    /// As `city_segment`.
    pub(super) fn city_slot(&self, city_root: &Path) -> Result<Assembled, AxError> {
        let mut slot = city_segment(city_root)?;
        if let Some(context) = self.naming.context() {
            if !slot.bytes.is_empty() {
                slot.bytes.push(NEWLINE);
                slot.bytes.push(NEWLINE);
            }
            slot.bytes.extend_from_slice(context.as_bytes());
        }
        Ok(slot)
    }

    /// The line the resident slot opens with: what this resident is
    /// called. It opens the resident slot rather than the city one,
    /// because a name per agent in the city slot would make one copy of
    /// the largest stable block in the prompt per agent.
    pub(super) fn called(&self, addr: &Address) -> Vec<u8> {
        format!("Your name: {}\n\n", self.naming.called(addr)).into_bytes()
    }
}
