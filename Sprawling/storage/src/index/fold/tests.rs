// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Observational equivalence of [`Folded`] against the map-shaped
//! implementation it replaced.
//!
//! [`Oracle`] is that implementation, byte for byte: a `BTreeMap` per
//! line and a `BTreeSet` per run. The two see the same lines through
//! the same door ([`Folded::insert_line`]) and must answer every public
//! query the same way — including out-of-order and repeated seqs,
//! unparseable run names and lines that are not documents at all.

use std::collections::{BTreeMap, BTreeSet};

use kernel::{RunId, Seq};
use proptest::prelude::*;

use super::{Folded, locate};

/// The maps this module used to be, kept as the oracle for the layout
/// it was replaced by.
struct Oracle {
    entries: BTreeMap<Seq, (String, u64)>,
    runs: BTreeMap<RunId, BTreeSet<Seq>>,
}

impl Oracle {
    fn empty() -> Oracle {
        Oracle {
            entries: BTreeMap::new(),
            runs: BTreeMap::new(),
        }
    }

    fn insert_line(&mut self, name: &str, offset: u64, body: &[u8]) {
        let Some(located) = locate(body) else {
            return;
        };
        self.entries.insert(located.seq, (name.to_owned(), offset));
        if let Some(run) = located.run {
            self.runs.entry(run).or_default().insert(located.seq);
        }
    }

    fn run_seqs_before(&self, run: RunId, before: Option<Seq>) -> Vec<Seq> {
        use std::ops::Bound;
        let upper = match before {
            Some(seq) => Bound::Excluded(seq),
            None => Bound::Unbounded,
        };
        self.runs
            .get(&run)
            .into_iter()
            .flat_map(|seqs| seqs.range((Bound::Unbounded, upper)).rev().copied())
            .collect()
    }
}

/// One line to index: which segment and offset it claims, its seq, and
/// its run as raw text — raw, because a spelling the ledger would never
/// write is exactly what the damaged-ledger path must survive.
#[derive(Debug, Clone)]
struct Line {
    seg: u8,
    offset: u64,
    seq: u64,
    run: Option<String>,
}

fn run_names() -> impl Strategy<Value = Option<String>> {
    // Three mints plus nil, and a fourth spelling no `RunId` accepts.
    let valid = prop_oneof![Just(0u8), Just(1u8), Just(2u8), Just(u8::MAX),];
    prop_oneof![
        8 => valid.prop_map(|byte| Some(run_id(byte).to_string())),
        2 => Just(Some("not-a-uuid-this-city-writes".to_owned())),
        2 => Just(None),
    ]
}

fn run_id(byte: u8) -> RunId {
    let mut bytes = [0u8; 16];
    bytes[15] = byte;
    RunId::from_bytes(bytes)
}

fn lines() -> impl Strategy<Value = Vec<Line>> {
    // A small seq space forces repeats and out-of-order arrivals; the
    // occasional large value keeps the tail honest. An offset too wide
    // to pack sends a line to the outliers, and when its slot is already
    // in the column that slot becomes a hole.
    let seq = prop_oneof![6 => 0u64..12, 1 => 12u64..4000];
    let offset = prop_oneof![8 => 0u64..65536, 1 => (1u64 << 48)..(1u64 << 49)];
    let line = (0u8..4, offset, seq, run_names()).prop_map(|(seg, offset, seq, run)| Line {
        seg,
        offset,
        seq,
        run,
    });
    prop::collection::vec(line, 0..48)
}

fn body_of(line: &Line) -> Vec<u8> {
    match &line.run {
        Some(run) => format!("{{\"seq\":{},\"run\":\"{}\"}}", line.seq, run).into_bytes(),
        None => format!("{{\"seq\":{}}}", line.seq).into_bytes(),
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    /// Every public query must answer exactly what the maps answered.
    #[test]
    fn folded_answers_as_the_maps_did(held in lines(), probes in proptest::collection::vec(0u64..4000, 12)) {
        let mut folded = Folded::empty();
        let mut oracle = Oracle::empty();
        for line in &held {
            let name = format!("seg-{}", line.seg);
            let body = body_of(line);
            folded.insert_line(&name, line.offset, &body);
            oracle.insert_line(&name, line.offset, &body);
        }

        for probe in &probes {
            let seq = Seq::new(*probe);
            let found = folded.loc_of(seq);
            let expected = oracle.entries.get(&seq);
            match (found, expected) {
                (Some((name, offset)), Some((held_name, held_offset))) => {
                    prop_assert_eq!(name, held_name.as_str());
                    prop_assert_eq!(offset, *held_offset);
                }
                (None, None) => {}
                (found, expected) => {
                    panic!("loc_of({probe}): {found:?} against {expected:?}");
                }
            }
        }

        let ascending: Vec<Seq> = oracle.entries.keys().copied().collect();
        prop_assert_eq!(folded.seqs().collect::<Vec<_>>(), ascending.clone());
        prop_assert_eq!(
            folded.seqs().rev().collect::<Vec<_>>(),
            ascending.iter().rev().copied().collect::<Vec<_>>()
        );
        // Both ends taken in turn meet in the middle without losing or
        // repeating a seq.
        let mut from_both = Vec::new();
        let mut seqs = folded.seqs();
        while let Some(low) = seqs.next() {
            from_both.push(low);
            from_both.extend(seqs.next_back());
        }
        from_both.sort();
        prop_assert_eq!(from_both, ascending);
        for probe in &probes {
            let from = Seq::new(*probe);
            let at_or_after: Vec<Seq> = oracle.entries.range(from..).map(|(seq, _)| *seq).collect();
            prop_assert_eq!(folded.seqs_from(from).collect::<Vec<_>>(), at_or_after.clone());
            prop_assert_eq!(
                folded.seqs_from(from).rev().collect::<Vec<_>>(),
                at_or_after.into_iter().rev().collect::<Vec<_>>()
            );
        }
        prop_assert_eq!(folded.tail_seq(), oracle.entries.keys().next_back().copied());
        prop_assert_eq!(folded.len(), oracle.entries.len());
        prop_assert_eq!(folded.is_empty(), oracle.entries.is_empty());

        for byte in [0u8, 1, 2, u8::MAX] {
            let run = run_id(byte);
            for before in [None, Some(Seq::FIRST), Some(Seq::new(3)), Some(Seq::new(4000))] {
                prop_assert_eq!(
                    folded.run_seqs_before(run, before).collect::<Vec<_>>(),
                    oracle.run_seqs_before(run, before)
                );
            }
        }
    }
}

/// A healthy ledger numbers its lines one after another, so the seq of
/// each line is implied by its place and only the offset is held.
#[test]
fn a_contiguous_ledger_costs_eight_bytes_per_record() {
    let mut folded = Folded::empty();
    let count = 10_000u64;
    for seq in 1..=count {
        let body = format!("{{\"seq\":{seq}}}");
        folded.insert_line("seg-0", seq * 64, body.as_bytes());
    }
    assert_eq!(
        (folded.len(), folded.entries.resident_bytes()),
        (10_000, 80_000)
    );
}

/// A damaged ledger whose seqs double from line to line must not double
/// the column with them: holes stay bounded by the lines the column holds.
#[test]
fn doubling_seqs_leave_the_column_bounded_by_its_lines() {
    let mut folded = Folded::empty();
    let seqs = std::iter::once(1u64)
        .chain(std::iter::successors(Some(65u64), |seq| seq.checked_mul(2)).take(15));
    for seq in seqs {
        let body = format!("{{\"seq\":{seq}}}");
        folded.insert_line("seg-0", seq, body.as_bytes());
    }
    let bound = 64 * (folded.len() + 64);
    assert!(
        folded.entries.resident_bytes() <= bound,
        "{} resident bytes for {} lines, bound {bound}",
        folded.entries.resident_bytes(),
        folded.len()
    );
}
