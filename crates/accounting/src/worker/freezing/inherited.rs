// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a session that branched off another starts with.

//!
//! Two facts and one act. The origin is the session's, folded from the
//! ledger when it opened (`crate::worker::folds::session`); the conversation
//! is the mother's, rebuilt from her own records
//! (`runtime::fork::inherited_indexed`, through the worker's resident
//! index); and what this writes is the lineage of
//! the run that inherits, which is also what marks the origin spent.
//!
//! **The cut that was actually used is the one written down.** A branch
//! that asked for a line inside a wave of tool calls settles on an
//! earlier one, and a page showing the line the person asked for would
//! be showing a cut the model was never given.

use kernel::{AxError, ChatMessage, RunId};

use super::super::{Assignment, RunWorker};

impl RunWorker {
    /// The conversation this run opens with, and the line it was
    /// rebuilt through - written to the ledger as this run's lineage.
    ///
    /// # Errors
    /// Propagates a ledger that does not verify, a cut the mother's
    /// history does not hold, and a lineage line the ledger refuses.
    /// The last of those leaves a frozen run with no record of where it
    /// came from, which is why it is raised rather than noted.
    pub(in crate::worker) fn inherited(
        &mut self,
        at: &Assignment,
        run: RunId,
    ) -> Result<Vec<ChatMessage>, AxError> {
        let Some(origin) = at.origin else {
            return Ok(Vec::new());
        };
        let dir = kernel::layout::CityLayout::new(&self.city_root).ledger();
        self.index
            .refresh(&dir)
            .map_err(storage::StorageError::into_ax)?;
        let rebuilt = runtime::fork::inherited_indexed(&self.index, &dir, origin.at_seq)?;
        self.note_lineage(
            &at.addr,
            run,
            kernel::Origin {
                run: origin.run,
                at_seq: rebuilt.at,
            },
        )?;
        Ok(rebuilt.messages)
    }
}
