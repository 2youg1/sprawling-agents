// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The three properties `crates/accounting/spec/Views/Usage.lean` proves
//! of its model, checked of the fold on traces drawn from the model's
//! input space: pins and reads by three runs of three skills.

use std::collections::BTreeSet;

use kernel::{EventKind, EventRecord};
use proptest::prelude::*;
use serde_json::json;

use super::Usage;
use super::tests::{called, now, record, started};

/// One line of the model in `Usage.lean`: a pin, a read, or another line.
#[derive(Debug, Clone)]
enum Line {
    Pinned { run: u8, skill: u8 },
    Read { run: u8, skill: u8 },
    Other,
}

fn lines() -> impl Strategy<Value = Vec<Line>> {
    let line = prop_oneof![
        (0u8..3, 0u8..3).prop_map(|(run, skill)| Line::Pinned { run, skill }),
        (0u8..3, 0u8..3).prop_map(|(run, skill)| Line::Read { run, skill }),
        Just(Line::Other),
    ];
    proptest::collection::vec(line, 0..24)
}

fn records(lines: &[Line]) -> Vec<EventRecord> {
    lines
        .iter()
        .zip(1u64..)
        .map(|(line, seq)| match line {
            Line::Pinned { run, skill } => started(seq, *run, &[(&format!("s{skill}"), "body")]),
            Line::Read { run, skill } => called(
                now(seq, *run),
                &format!("c{seq}"),
                "read",
                json!({ "path": format!("s{skill}") }),
            ),
            Line::Other => record(now(seq, 0), EventKind::RunFrozen, json!({})),
        })
        .collect()
}

proptest! {
    /// `a_read_the_run_did_not_pin_is_not_a_use` and `every_use_was_pinned`:
    /// a read counts exactly when its run pinned that skill before it.
    #[test]
    fn a_read_counts_exactly_when_its_run_pinned_the_skill(lines in lines()) {
        let usage = Usage::fold(records(&lines));
        let mut pinned = BTreeSet::new();
        let mut expected = Vec::new();
        for (line, seq) in lines.iter().zip(1u64..) {
            match line {
                Line::Pinned { run, skill } => { pinned.insert((*run, *skill)); }
                Line::Read { run, skill } => if pinned.contains(&(*run, *skill)) { expected.push(seq); },
                Line::Other => {}
            }
        }
        let counted: Vec<u64> = usage.reads.iter().map(|read| read.used.seq.value()).collect();
        prop_assert_eq!(counted, expected);
    }

    /// `the_fold_only_appends`: folding more of the ledger never moves a
    /// use already counted.
    #[test]
    fn folding_more_only_appends_uses(lines in lines(), cut in 0usize..24) {
        let all = records(&lines);
        let cut = cut.min(all.len());
        let seqs = |usage: Usage| usage.reads.iter().map(|read| read.used.seq).collect::<Vec<_>>();
        let before = seqs(Usage::fold(&all[..cut]));
        let after = seqs(Usage::fold(&all));
        prop_assert_eq!(&after[..before.len()], &before[..]);
    }
}
