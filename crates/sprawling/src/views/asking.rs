// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One query answered out of a city's own history without serving it:
//! the whole chain audited, the views folded, asked and thrown away.

use std::path::Path;

use kernel::AxError;

use super::holding::Views;
use super::snapshot::start::start_audited;

/// Answers one query out of a city's own history, without serving it.
///
/// The views are folded, asked, and thrown away: from the snapshot a
/// served city cut when one fits, from genesis otherwise, and nothing is
/// left behind. **It is the same
/// [`Views::prepare`] a served city answers from**: a command line that
/// read the history its own way would be a second answer to one
/// question, and the one that drifted would be the one nobody was
/// looking at.
///
/// # Errors
/// Propagates a chain the whole-ledger audit finds broken or cannot
/// read, folded lines that do not verify, and a record that will not
/// parse. A city whose chain is broken is not one whose views should be
/// handed to anybody.
pub fn ask(city_root: &Path, query: &wire::Query) -> Result<wire::Answer, AxError> {
    Ok(
        Views::rebuild(&kernel::layout::CityLayout::new(city_root).ledger())?
            .prepare(query)
            .finish(),
    )
}

impl Views {
    /// The views of the ledger on disk, from its snapshot when one fits
    /// and from genesis otherwise (sprawling-SPEC 8-91). A reader that
    /// serves nothing: it cuts no snapshot, so a one-shot query writes
    /// nothing to disk, and the answer is the same as if the process had
    /// been running all along.
    ///
    /// The whole chain is audited first, because no background audit runs
    /// beside a one-shot read and the snapshot's fit checks only the line
    /// at its seq: without the audit a line edited before that seq would
    /// be answered from.
    ///
    /// # Errors
    /// The audit's reason when the chain is broken or cannot be read, and
    /// the verification failures of the lines it folds; a city whose
    /// history does not verify is not one whose views should be served.
    pub(crate) fn rebuild(ledger_dir: &Path) -> Result<Views, AxError> {
        start_audited::<Views>(ledger_dir).map(|started| started.folded)
    }
}
