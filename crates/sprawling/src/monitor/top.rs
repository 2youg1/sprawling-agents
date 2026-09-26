// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What `sprawling top` prints: one JSON line per sample when stdout is
//! not a terminal, and one curve per counter when it is
//! (sprawling-SPEC.md 8-91). Pure: the caller owns the terminal.

use super::Sample;

/// The line printed each second when stdout is not a terminal: one JSON
/// object keyed by the [`Sample`] field names, without the newline.
#[must_use]
pub fn json_line(sample: &Sample) -> String {
    let _ = sample;
    String::new()
}

/// The last `width` values as one block character each, scaled from the
/// window's own minimum (`▁`) to its maximum (`█`).
#[must_use]
pub fn sparkline(values: impl IntoIterator<Item = u64>, width: usize) -> String {
    let _ = (values.into_iter(), width);
    String::new()
}

#[cfg(test)]
#[path = "top/tests.rs"]
mod tests;
