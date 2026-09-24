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
    // occasional large value keeps the tail honest.
    let seq = prop_oneof![6 => 0u64..12, 1 => 12u64..4000];
    let offset = 0u64..65536;
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

        prop_assert_eq!(folded.tail_seq(), oracle.entries.keys().next_back().copied());
        prop_assert_eq!(folded.len(), oracle.entries.len());
        prop_assert_eq!(folded.is_empty(), oracle.entries.is_empty());

        for byte in [0u8, 1, 2, u8::MAX] {
            let run = run_id(byte);
            for before in [None, Some(Seq::FIRST), Some(Seq::new(3)), Some(Seq::new(4000))] {
                prop_assert_eq!(
                    folded.run_seqs_before(run, before),
                    oracle.run_seqs_before(run, before)
                );
            }
        }
    }
}
