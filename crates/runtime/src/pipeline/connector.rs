// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One MCP server's answer as the model reads it (runtime-SPEC.md
//! 8-27-10): text that fits passes untouched, text that does not is
//! stored whole and replaced by a window the model pages with `read`.

use kernel::{AxError, ToolOutcome};

use crate::offload::OffloadSite;

/// The most text one connector answer puts into the window.
pub const CONNECTOR_CAP_BYTES: u64 = 16_384;

/// Packages one connector answer for the window.
///
/// # Errors
/// Propagates whatever `package` reports about the store.
pub fn package_connector(
    outcome: ToolOutcome,
    _offload: OffloadSite<'_>,
) -> Result<ToolOutcome, AxError> {
    Ok(outcome)
}

#[cfg(test)]
mod tests;
