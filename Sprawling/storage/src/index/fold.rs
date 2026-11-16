// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::cmp::Ordering;
use std::collections::BTreeMap;

use kernel::{RunId, Seq};

mod entries;
use entries::{Entries, Seqs};

#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct Folded {
    pub(crate) scanned: BTreeMap<String, u64>,
    entries: Entries,
    runs: RunTable,
}

impl Folded {
    pub(crate) fn empty() -> Folded {
        Folded {
            scanned: BTreeMap::new(),
            entries: Entries::empty(),
            runs: RunTable::empty(),
        }
    }

    pub(crate) fn fold_segment(&mut self, name: &str, from: u64, tail: &[u8]) -> u64 {
        let mut offset = from;
        let mut complete_bytes = 0u64;
        for line in tail.split_inclusive(|b| *b == b'\n') {
            if line.last().copied() != Some(b'\n') {
                break;
            }
            let body = line.get(..line.len().saturating_sub(1)).unwrap_or(line);
            if !body.is_empty() {
                self.insert_line(name, offset, body);
            }
            let len = u64::try_from(line.len()).unwrap_or(0);
            offset = offset.saturating_add(len);
            complete_bytes = complete_bytes.saturating_add(len);
        }
        complete_bytes
    }

    pub(crate) fn insert_line(&mut self, name: &str, offset: u64, body: &[u8]) {
        if let Some(located) = locate(body) {
            self.insert_located(name, offset, located);
        }
    }

    pub(crate) fn insert_located(&mut self, name: &str, offset: u64, located: Located) {
        self.entries.insert(located.seq, name, offset);
        if let Some(run) = located.run {
            self.runs.add(run, located.seq);
        }
    }

    pub(crate) fn loc_of(&self, seq: Seq) -> Option<(&str, u64)> {
        self.entries.loc_of(seq)
    }

    pub(crate) fn run_seqs_before(
        &self,
        run: RunId,
        before: Option<Seq>,
    ) -> impl Iterator<Item = Seq> {
        self.runs.seqs_before(run, before)
    }

    pub(crate) fn tail_seq(&self) -> Option<Seq> {
        self.entries.tail_seq()
    }

    pub(crate) fn seqs(&self) -> Seqs<'_> {
        self.entries.seqs()
    }

    pub(crate) fn seqs_from(&self, from: Seq) -> Seqs<'_> {
        self.entries.seqs_from(from)
    }

    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.entries.len() == 0
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct RunTable {
    runs: Vec<RunSpans>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct RunSpans {
    run: RunId,
    spans: Vec<Span>,
}

#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
struct Span {
    start: Seq,
    end: Seq,
}

impl RunTable {
    fn empty() -> RunTable {
        RunTable { runs: Vec::new() }
    }

    fn add(&mut self, run: RunId, seq: Seq) {
        match self.runs.binary_search_by(|held| held.run.cmp(&run)) {
            Ok(at) => {
                if let Some(held) = self.runs.get_mut(at) {
                    held.add(seq);
                }
            }
            Err(at) => self.runs.insert(
                at,
                RunSpans {
                    run,
                    spans: vec![Span {
                        start: seq,
                        end: seq,
                    }],
                },
            ),
        }
    }

    fn seqs_before(&self, run: RunId, before: Option<Seq>) -> impl Iterator<Item = Seq> {
        let spans: &[Span] = match self
            .runs
            .binary_search_by(|held| held.run.cmp(&run))
            .ok()
            .and_then(|at| self.runs.get(at))
        {
            Some(held) => &held.spans,
            None => &[],
        };

        let ceiling = match before {
            None => Some(u64::MAX),
            Some(before) => before.value().checked_sub(1),
        };
        spans
            .iter()
            .rev()
            .filter_map(move |span| {
                let top = ceiling?.min(span.end.value());
                (top >= span.start.value()).then_some((span.start.value(), top))
            })
            .flat_map(|(low, top)| (low..=top).rev().map(Seq::new))
    }
}

impl RunSpans {
    fn add(&mut self, seq: Seq) {
        let at = match self.spans.binary_search_by(|span| span.place(seq)) {
            Ok(_) => return,
            Err(at) => at,
        };
        let joins_below = self.spans.get(at).is_some_and(|span| span.abuts_after(seq));
        let joins_above = at
            .checked_sub(1)
            .and_then(|at| self.spans.get(at))
            .is_some_and(|span| span.abuts_before(seq));
        match (joins_above, joins_below) {
            (true, true) => {
                let stretch = self.spans.get(at).map(|span| span.end);
                let reach = at.checked_sub(1).and_then(|at| self.spans.get_mut(at));
                if let (Some(stretch), Some(reach)) = (stretch, reach) {
                    reach.end = stretch;
                }
                self.spans.remove(at);
            }
            (true, false) => {
                if let Some(reach) = at.checked_sub(1).and_then(|at| self.spans.get_mut(at)) {
                    reach.end = seq;
                }
            }
            (false, true) => {
                if let Some(reach) = self.spans.get_mut(at) {
                    reach.start = seq;
                }
            }
            (false, false) => self.spans.insert(
                at,
                Span {
                    start: seq,
                    end: seq,
                },
            ),
        }
    }
}

impl Span {
    fn place(&self, seq: Seq) -> Ordering {
        if seq < self.start {
            Ordering::Greater
        } else if self.end < seq {
            Ordering::Less
        } else {
            Ordering::Equal
        }
    }

    fn abuts_before(&self, seq: Seq) -> bool {
        self.end.value().checked_add(1) == Some(seq.value())
    }

    fn abuts_after(&self, seq: Seq) -> bool {
        seq.value().checked_add(1) == Some(self.start.value())
    }
}

pub struct Located {
    pub seq: Seq,

    pub run: Option<RunId>,
}

pub(crate) fn locate(line: &[u8]) -> Option<Located> {
    let value: serde_json::Value = serde_json::from_slice(line).ok()?;
    let seq = Seq::new(value.get("seq")?.as_u64()?);
    let run = value
        .get("run")
        .and_then(serde_json::Value::as_str)
        .and_then(|raw| RunId::parse(raw).ok());
    Some(Located { seq, run })
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
