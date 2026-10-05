// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Read-only privacy status (`crates/sprawling/spec/Privacy/Cli.lean`).

use accounting::home::Home;
use kernel::{AxCode, AxError};

/// Reads local operation receipts without creating or settling history.
///
/// # Errors
/// Home detection, busy or malformed history, IO, and JSON encoding failures.
pub fn status() -> Result<String, AxError> {
    let history = super::journal::read(&Home::detect()?.privacy_history())
        .map_err(super::state::HistoryFault::into_ax)?;
    serde_json::to_string(history.statuses()).map_err(|source| {
        AxError::failure(
            AxCode::InvalidArgs,
            "encode privacy status",
            source.to_string(),
        )
        .with_recovery("keep the history unchanged and report the encoding failure")
    })
}
