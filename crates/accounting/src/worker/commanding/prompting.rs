// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A whole-document step on one prompt document
//! (`crates/accounting/Spec.lean` §8-22), not taken yet.

use kernel::{Address, AxCode, AxError, B3Hash};

use super::super::RunWorker;

impl RunWorker {
    /// Refuses every step until the city's list exists.
    ///
    /// # Errors
    /// Always `E_INVALID_ARGS`.
    pub(in crate::worker) fn put_prompt(
        &mut self,
        at: &Address,
        _base: B3Hash,
        _step: wire::PromptStep,
    ) -> Result<(), AxError> {
        Err(AxError::failure(
            AxCode::InvalidArgs,
            "change a prompt document",
            at.as_str().to_owned(),
        )
        .with_recovery("not built"))
    }
}
