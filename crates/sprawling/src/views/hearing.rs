// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The facility that turns a recording into text, as this city's own
//! choice describes it.
//!
//! It is here rather than beside the answers because it answers
//! nothing: a query is read and returned in one call, and this hands
//! back something the caller then spends seconds on. What it shares
//! with the answers is where the choice lives, which is why it reads
//! the book through the same door they do.

use kernel::AxError;

use super::holding::Views;

impl Views {
    /// The facility that turns a recording into text, as the endpoint
    /// chosen for `transcribe` describes it.
    ///
    /// Built here and called outside: the choice is city state and is
    /// read under this lock, while the provider round trip is seconds
    /// long and must not hold the answer to every other read for them.
    ///
    /// # Errors
    /// Propagates the book's refusal when no endpoint was chosen to
    /// transcribe, and the HTTP client's refusal to be built.
    pub(crate) fn transcriber(
        &self,
        secrets: gateway::SecretResolver,
    ) -> Result<gateway::Transcriber, AxError> {
        let chosen = self.book.select(
            kernel::ModelTag::Transcribe,
            &kernel::BuildingPolicy::default(),
        )?;
        gateway::transcriber_for(&chosen, secrets)
    }
}
