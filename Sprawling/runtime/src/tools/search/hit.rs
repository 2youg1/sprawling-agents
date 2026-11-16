// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use crate::elision::{self, Elided};

use super::MATCH_CAP;
use super::predicate::Lines;

pub(super) const LINE_CAP: usize = 512;

#[derive(Clone, Copy)]
pub(super) enum Limit {

    Hits,

    Bytes,

    Files,
}

impl Limit {

    pub(super) fn said(self) -> String {
        let narrow = "narrow `path` to one directory or file, add a `glob`, or make `pattern` \
                      more specific";
        match self {
            Limit::Hits => {
                format!("stopped at {MATCH_CAP} matches, the most one search returns; {narrow}")
            }
            Limit::Bytes => format!(
                "stopped at {} bytes of match text, the most one search returns; {narrow}, or ask for fewer lines of context",
                kernel::consts_policy::INTERVAL_CAP_BYTES
            ),
            Limit::Files => {
                format!("stopped at {MATCH_CAP} files, the most one search lists; {narrow}")
            }
        }
    }
}

pub(super) fn context_block(lines: &[&str], at: usize, asked: &Lines) -> (usize, String) {
    let first = at.saturating_sub(asked.context.before);
    let last = at
        .saturating_add(asked.context.after)
        .min(lines.len().saturating_sub(1));
    let block = (first..=last)
        .filter_map(|index| {
            let line = lines.get(index)?;
            let centre = if index == at {
                asked.pattern.find(line).map_or(0, |found| found.start())
            } else {
                0
            };
            Some(windowed(line, centre))
        })
        .collect::<Vec<String>>()
        .join("\n");
    (first, block)
}

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
