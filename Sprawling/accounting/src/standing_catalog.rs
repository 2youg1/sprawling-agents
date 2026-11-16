// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::config::SearchSupplier;
use kernel::{AxError, ToolDef};

pub(crate) fn builtins(
    search: Option<&SearchSupplier>,
) -> Result<Vec<(&'static str, ToolDef)>, AxError> {
    crate::worker::catalog_builtins(search)
}
