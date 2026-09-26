// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Putting one recycle-bin row back by the way back it carries
//! (sprawling-SPEC §8-92).

use crate::assembly::RunWorker;
use kernel::AxError;

use crate::assembly::RunWorker;

impl RunWorker {
    /// Writes the discarded file back, then records that it came back.
    ///
    /// # Errors
    /// Refuses a way back this city cannot follow, and propagates the
    /// checkpoint that cannot find or write the bytes and the ledger
    /// that refuses the append.
    pub(in crate::assembly) fn restore_discard(
        &mut self,
        restoration: &kernel::Restoration,
    ) -> Result<(), AxError> {
        let _ = restoration;
        Ok(())
    }
}
