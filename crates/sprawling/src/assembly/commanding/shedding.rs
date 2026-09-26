// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Whether the city takes on new work while its volume is close to full
//! (sprawling-SPEC.md 8-94).

use std::path::Path;

use kernel::degradation::VolumeSpace;

use super::super::RunWorker;

impl RunWorker {
    /// Replaces the volume reader, so a test can inject a volume close
    /// to full.
    #[cfg(test)]
    pub(in crate::assembly) fn read_volume_with(
        &mut self,
        read: fn(&Path) -> Option<VolumeSpace>,
    ) {
        self.read_volume = read;
    }
}
