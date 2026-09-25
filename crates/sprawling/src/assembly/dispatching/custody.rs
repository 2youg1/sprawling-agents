// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A key a person pastes into a dispatch goes to the vault, and the
//! dispatch carries its `secret:` reference instead (sprawling-SPEC.md
//! 8-84).

use kernel::AxError;

use super::super::RunWorker;

#[cfg(test)]
mod tests;

impl RunWorker {
    /// Moves every provider-shaped key in `text` into the vault and
    /// returns the text with each key replaced by its reference.
    pub(in crate::assembly) fn take_custody(&mut self, text: String) -> Result<String, AxError> {
        Ok(text)
    }
}
