// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Editing the user search path everywhere else: nothing is written,
//! and the line a person adds themselves travels back with the outcome.

use super::{PathOutcome, on_search_path};
use kernel::AxError;

/// Nothing here writes a shell startup file. Which one to write is a
/// guess between `.profile`, `.bashrc`, `.zshrc` and fish's own
/// syntax, and guessing wrong leaves a line in somebody's login
/// script that does nothing and that they have to find to remove.
/// `~/.local/bin` is already on the search path on current
/// distributions; when it is not, the exact line travels back with
/// the outcome and the person places it where they keep such lines.
fn line(dir: &str) -> String {
    format!("export PATH=\"{dir}:$PATH\"")
}

fn current() -> String {
    std::env::var("PATH").unwrap_or_default()
}

pub(super) fn extend(dir: &str) -> Result<(PathOutcome, Option<String>), AxError> {
    if on_search_path(&current(), dir) {
        return Ok((PathOutcome::Unchanged, None));
    }
    Ok((PathOutcome::SelfService(line(dir)), None))
}

/// Nothing was written, so nothing is taken back: removing the copy
/// is the whole of an uninstall on this platform.
pub(super) fn retract(_dir: &str) -> Result<PathOutcome, AxError> {
    Ok(PathOutcome::Unchanged)
}
