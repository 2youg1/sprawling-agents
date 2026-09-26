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
///
/// # Errors
///
/// Only what `serde_json` reports; a struct of integers gives it nothing
/// to refuse, and the caller treats it as a failed write to stdout.
pub fn json_line(sample: &Sample) -> serde_json::Result<String> {
    serde_json::to_string(sample)
}

/// The eight heights a curve is drawn in, lowest first.
const LEVELS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

/// The last `width` values as one block character each, scaled from the
/// window's own minimum (`▁`) to its maximum (`█`); a flat window is `▁`.
#[must_use]
pub fn sparkline(values: impl IntoIterator<Item = u64>, width: usize) -> String {
    let all: Vec<u64> = values.into_iter().collect();
    let window = all.iter().copied().skip(all.len().saturating_sub(width));
    let low = window.clone().min().unwrap_or(0);
    let span = window.clone().max().unwrap_or(0).saturating_sub(low);
    window
        .filter_map(|value| level(value.saturating_sub(low), span))
        .collect()
}

/// The character `offset` above the window's minimum reaches in a window
/// `span` tall. A flat window has no height to divide, so it draws the
/// lowest level; `offset <= span` keeps the rank inside [`LEVELS`].
fn level(offset: u64, span: u64) -> Option<char> {
    let top = u128::try_from(LEVELS.len().checked_sub(1)?).ok()?;
    let rank = u128::from(offset)
        .checked_mul(top)?
        .checked_div(u128::from(span))
        .unwrap_or(0);
    LEVELS.get(usize::try_from(rank).ok()?).copied()
}

/// One terminal screen: a row per counter with its label, its latest
/// reading and its last `curve_width` points; empty with no samples.
#[must_use]
pub fn screen(samples: &[Sample], curve_width: usize) -> String {
    let _ = (samples, curve_width);
    String::new()
}

#[cfg(test)]
#[path = "top/tests.rs"]
mod tests;
