// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! `sprawling top`: watch a served city's monitor over the wire and
//! write each reading to stdout (sprawling-SPEC.md 8-93).

use std::collections::VecDeque;

use sprawling::monitor::Sample;

/// How a reading is written: redrawn on a terminal, one JSON line a
/// second for anything else, an agent included.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Output {
    Screen,
    Lines,
}

/// The text one received frame puts on stdout, keeping the reading in
/// `history`; `None` for a frame that is not a monitor reading.
pub(crate) fn shown(
    text: &str,
    history: &mut VecDeque<Sample>,
    output: Output,
    curve_width: usize,
) -> Option<String> {
    let _unused = (text, history, output, curve_width);
    None
}

#[cfg(test)]
#[path = "watching/tests.rs"]
mod tests;
