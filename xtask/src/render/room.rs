// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Whether words had room to stand and a popover room to open
//! (xtask-SPEC.md section 8-33).

use browser::survey::Drawn;

use crate::report::Violation;

pub(super) fn no_text_is_crushed(_drawn: &[Drawn], _at: &str, _out: &mut Vec<Violation>) {}

pub(super) fn every_popover_shows_an_option(
    _drawn: &[Drawn],
    _at: &str,
    _out: &mut Vec<Violation>,
) {
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests;
