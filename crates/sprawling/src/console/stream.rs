// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The event stream the console prints: one line per committed record
//! by default, the record whole only when the User asks
//! (`crates/sprawling/spec/Console.lean` §8-11, sprawling D44).

use kernel::Address;

/// How much of each committed record the console prints.
///
/// A record carries what the User typed and what a model answered, and a
/// terminal is read over a shoulder, scrolled into a recording and kept in
/// a scrollback file, so the default is the line that says what happened
/// and where, and the payload is shown only when the User asks for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Records {
    /// One line per record: its seq, its kind and its address.
    Summary,
    /// The record whole, payload included, as `sprawling call` prints it.
    Whole,
}

/// Prints every record committed from now on, on a thread of its own,
/// because the receiver blocks while the reactor serves the city.
pub(super) fn print(
    records: Records,
    mut watching: tokio::sync::broadcast::Receiver<wire::Committed>,
) {
    std::thread::spawn(move || {
        while let Ok(committed) = watching.blocking_recv() {
            match printed(committed.record(), records) {
                Ok(text) => println!("{text}"),
                // The record is on the ledger and reached every socket as
                // its frame; only this printout lacks it, so the gap is
                // named rather than left silent.
                Err(error) => eprintln!(
                    "  seq {} is in the history but could not be printed here: {error}",
                    committed.record().seq().value()
                ),
            }
        }
    });
}

/// One committed record, as the event stream prints it.
///
/// The kind is spelled by its serde name, the one the ledger and the wire
/// use, so the summary line names no kind a second way; the whole record
/// is the shape `sprawling call` prints.
pub(super) fn printed(
    record: &kernel::EventRecord,
    records: Records,
) -> Result<String, serde_json::Error> {
    match records {
        Records::Whole => serde_json::to_string(record),
        Records::Summary => {
            let kind = serde_json::to_value(record.kind())?;
            let kind = kind.as_str().unwrap_or("?");
            let at = record.addr().map_or("city", Address::as_str);
            Ok(format!("  seq {}  {kind}  {at}", record.seq().value()))
        }
    }
}
