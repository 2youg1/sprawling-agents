// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Built-in tools outside `sprawling` that a catalog page may list before
//! a run. Lives beside `views` and `worker` so the page can ask without
//! naming the writer (`docs/ARCHITECTURE.md` §3 directions).

use kernel::config::SearchSupplier;
use kernel::{AxError, ToolDef};

/// Playback, proposal, and `web_search` when `[search]` names a supplier.
///
/// # Errors
/// Propagates a definition whose schema does not build.
pub(crate) fn builtins(
    search: Option<&SearchSupplier>,
) -> Result<Vec<(&'static str, ToolDef)>, AxError> {
    crate::worker::catalog_builtins(search)
}
