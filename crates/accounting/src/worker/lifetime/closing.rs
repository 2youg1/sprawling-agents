// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Why the city is closing, and the line that records it: the UNLOADING
//! end of one worker's lifetime (`crates/accounting/spec/Worker.lean` §8-11).
//!
//! It sits beside [`super`], which holds the LOADING end, because a
//! reader asking "what does a restart find" and "what does a close
//! leave" is asking one question from two ends.

use super::super::{RunWorker, city_segment};
use kernel::{AxError, EventKind, Locator};

/// Why the city is closing, carried from whoever decided it to the
/// line that records it.
///
/// Exhaustive rather than a flag, because the handoff says a different
/// thing for each: a close the person chose and a close serving forced
/// are different facts for the next session, and a record that spelled
/// both as the first would claim a choice nobody made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Closing {
    /// The person stopped the city from the keyboard.
    Chosen,
    /// Serving failed and took the city down; `cause` is what failed.
    Broken { cause: String },
}

impl Closing {
    /// The one reading of how serving ended: a serve that returned cleanly
    /// was ended by the person's Ctrl-C, and a serve that failed names its
    /// failure, so a failed serve is never recorded as the person's choice.
    pub fn of(served: &Result<(), AxError>) -> Self {
        match served {
            Ok(()) => Self::Chosen,
            Err(failure) => Self::Broken {
                cause: failure.to_string(),
            },
        }
    }
}

impl RunWorker {
    /// Closes the city in the record, so a stop somebody chose, a stop
    /// serving forced, and a crash are three different records rather
    /// than one line and one silence.
    ///
    /// The five sections are the city's own: what the next session must
    /// read is the city's norms, and where it left off is the position
    /// the ledger stands at. Written through `runtime::handoff`, which
    /// is the one construction point for the shape - a hand-built
    /// payload here would be a second one.
    ///
    /// # Errors
    /// Propagates the handoff's refusal of an empty must-read list, and
    /// the ledger's refusal to take the line.
    pub(crate) fn close_city(&mut self, why: &Closing) -> Result<(), AxError> {
        // The city's own norm, not a building's: `city::norms` answers
        // for a run at an address, and this line belongs to the city.
        // Through the same reader the prefix uses. What this city's
        // norms are has one answer, and hashing zero bytes here made the
        // must-read locator point at nothing while the handoff went on
        // saying the next session must read them.
        let mut must_read = Vec::new();
        let bytes = city_segment(&self.city_root)?.bytes;
        let hash = self
            .cas
            .put(&bytes)
            .map_err(storage::StorageError::into_ax)?;
        must_read.push(Locator::cas(hash));
        let standing = self.ledger.position();
        let (overview, context, next_step) = match why {
            Closing::Chosen => (
                "the city was closed by the User running it".to_owned(),
                "an orderly close, not a crash: nothing was interrupted mid-command".to_owned(),
                "`sprawling serve` on this directory continues from here".to_owned(),
            ),
            Closing::Broken { cause } => (
                format!("the city stopped because serving failed: {cause}"),
                "not a choice: serving failed; the ledger holds every line that landed before it"
                    .to_owned(),
                "fix what the failure names, then `sprawling serve` on this directory".to_owned(),
            ),
        };
        let handoff = runtime::handoff::Handoff::new(
            must_read,
            overview,
            format!("the ledger stands at {}", standing.value()),
            context,
            next_step,
        )?;
        self.note(
            runtime::diagnostics::Level::Effect,
            "accounting::worker",
            "the city is closing; its handoff is on the ledger",
        );
        self.record(EventKind::HandoffWritten, handoff.payload()?)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::Closing;
    use kernel::{AxCode, AxError};

    #[test]
    fn a_failed_serve_closes_broken_and_a_clean_one_closes_chosen() {
        let failure = AxError::failure(AxCode::StorageFatal, "serve the city", "port taken")
            .with_recovery("choose another port");
        assert_eq!(
            [Closing::of(&Err(failure.clone())), Closing::of(&Ok(()))],
            [
                Closing::Broken {
                    cause: failure.to_string()
                },
                Closing::Chosen
            ]
        );
    }
}
