// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What one search hit carries, and what a walk that stopped says
//! (`crates/runtime/spec/Tools/Search.lean` §8-30).

use crate::elision::{self, Elided};

use super::{MATCH_CAP, Predicate};

/// How much of one line a hit carries. A line past it is cut to a
/// window: the matched line around its first match, a context line from
/// its start. One JSON transcript line runs to tens of kilobytes, and a
/// hit that carried it whole spent the answer's budget in four hits; the
/// line number is where `read` takes up the rest.
pub(super) const LINE_CAP: usize = 512;

/// Which limit ended a walk before it had looked everywhere.
#[derive(Clone, Copy)]
pub(super) enum Limit {
    /// [`MATCH_CAP`] hits were already in the answer.
    Hits,
    /// The hits' text reached `INTERVAL_CAP_BYTES`.
    Bytes,
}

impl Limit {
    /// What the model is told: which limit, and how to ask a smaller
    /// question.
    pub(super) fn said(self) -> String {
        let narrow = "narrow `path` to one directory or file, or search for a longer `text`";
        match self {
            Limit::Hits => {
                format!("stopped at {MATCH_CAP} matches, the most one search returns; {narrow}")
            }
            Limit::Bytes => format!(
                "stopped at {} bytes of match text, the most one search returns; {narrow}, or ask for less `context`",
                kernel::consts_policy::INTERVAL_CAP_BYTES
            ),
        }
    }
}

/// The lines around a hit, each cut to [`LINE_CAP`], joined as they
/// were read.
pub(super) fn context_block(lines: &[&str], at: usize, looking: &Predicate) -> String {
    let first = at.saturating_sub(looking.context);
    let last = at
        .saturating_add(looking.context)
        .min(lines.len().saturating_sub(1));
    (first..=last)
        .filter_map(|index| {
            let line = lines.get(index)?;
            let centre = if index == at {
                line.find(&looking.needle).unwrap_or(0)
            } else {
                0
            };
            Some(windowed(line, centre))
        })
        .collect::<Vec<String>>()
        .join(
            "
",
        )
}

/// At most [`LINE_CAP`] bytes of `line` around `centre`, each cut end
/// marked with the count of bytes it dropped. A window that starts at
/// the line's start loses only its tail.
fn windowed(line: &str, centre: usize) -> String {
    if line.len() <= LINE_CAP {
        return line.to_owned();
    }
    let start = centre
        .saturating_sub(LINE_CAP / 2)
        .min(line.len().saturating_sub(LINE_CAP));
    let end = start.saturating_add(LINE_CAP);
    let tail_cut = elision::splice(line, end, line.len(), Elided::Tail).text;
    if start == 0 {
        return tail_cut;
    }
    elision::splice(&tail_cut, 0, start, Elided::Head).text
}
