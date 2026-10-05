// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Seatbelt command placement; specified by
//! `crates/runtime/spec/Tools/Exec/NativeMacos.lean`.

use std::path::Path;
use std::process::Command;
use kernel::AxError;

pub(super) fn wrap(wrapper: &Path, copy: &Path, command: &Command) -> Result<Command, AxError> {
    let mut wrapped = Command::new(wrapper);
    wrapped.current_dir(copy).arg(command.get_program()).args(command.get_args());
    Ok(wrapped)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing, reason = "test code")]
mod tests;
