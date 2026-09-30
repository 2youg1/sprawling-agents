// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The two answers that read the Ledger back: a slice of the one
//! history, and one slice of a named range of it.

use kernel::{EventRecord, Seq};
use serde::{Deserialize, Serialize};

/// A slice of the one history, oldest first - the order the ledger
/// wrote them and the order a fold expects. A reader that wants the
/// newest first reverses a list it already has, and a server that
/// reversed it would make the fold the caller's problem.
// No `Eq`: an `EventRecord` carries a payload whose numbers may be
// floats, and the wire's other answers derive it only because none of
// them holds one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct HistoryAnswer {
    pub records: Vec<EventRecord>,
    /// Where to ask next to go further back. `None` means this slice
    /// reaches the first record the city ever wrote.
    pub earlier: Option<Seq>,
}

/// One slice of a named range of ledger records, and where the slice
/// continues.
///
/// **The endpoints come back with the records.** The question that asked
/// for them carried no cursor a caller holds on to - not a `before` to
/// walk back from, but two seq numbers that may have arrived in a frame
/// the caller has already dropped - so an answer that did not name its
/// own slice could not be told from one for a different range, and a page
/// filling a gap while its record view is open would file the wrong
/// answer.
// No `Eq`, for the reason `HistoryAnswer` gives: a record's payload is
// arbitrary JSON.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct HistoryRangeAnswer {
    /// The first sequence this slice could have held, echoed from the
    /// question.
    pub from: Seq,
    /// The last sequence the question asked for, echoed. A `next` of
    /// `None` says these records are everything the Ledger holds between
    /// the two.
    pub to: Seq,
    pub records: Vec<EventRecord>,
    /// The sequence to ask from to finish the range, or `None` when the
    /// range is answered and there is nothing more to ask for.
    pub next: Option<Seq>,
}
