// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which runs claimed one plan node, and what they were billed.
//!
//! **Nothing here prices anything.** `gateway::cost` prices a call and
//! `storage::attribution` decides which run a billed call belongs to;
//! this module joins those two facts to the node a run claimed and adds
//! the result up. A second place that priced a call would be a second
//! answer to what a city spent, and the one that drifted would be the
//! one nobody was looking at.

use kernel::{NodeId, RunId, UsdMicros};

use super::holding::Views;

impl Views {
    /// What one plan node has cost so far.
    ///
    /// A node nobody has claimed answers zero with an empty list rather
    /// than `Unavailable`: "no run has held this node" is a true answer,
    /// while `None` says the view could not read a run's records.
    pub(super) fn cost_of_answer(&self, node: &NodeId) -> Option<wire::CostOfAnswer> {
        let held = self.claims.get(node).cloned().unwrap_or_default();
        let mut runs: Vec<(RunId, UsdMicros)> = Vec::with_capacity(held.len());
        let mut spent: u64 = 0;
        for run in held {
            let billed = self.billed_to(run)?;
            spent = spent.saturating_add(billed.get());
            runs.push((run, billed));
        }
        Some(wire::CostOfAnswer {
            node: node.clone(),
            spent: UsdMicros::new(spent),
            runs,
        })
    }

    /// What each named run was billed, in the order asked and cut at
    /// `wire::RUN_COSTS_MAX`; a run whose records cannot be read has
    /// no row.
    pub(super) fn run_costs_answer(&self, runs: &[RunId]) -> wire::RunCostsAnswer {
        wire::RunCostsAnswer {
            asked: runs.to_vec(),
            runs: runs
                .iter()
                .take(wire::RUN_COSTS_MAX)
                .filter_map(|run| self.billed_to(*run).map(|billed| (*run, billed)))
                .collect(),
        }
    }

    /// What one run was billed, the one answer every view that shows a
    /// run's money reads (sprawling-SPEC section 8-106): the
    /// attribution's row while it holds one, the run's own records in
    /// the Ledger once the hot view evicted it, and zero for a run no
    /// priced call was attributed to. `None` when the Ledger could not
    /// be read.
    pub(super) fn billed_to(&self, run: RunId) -> Option<UsdMicros> {
        match self.attribution.billed_to(&run) {
            Some(billed) => Some(billed),
            None if self.hot.was_evicted(&run) => self.ledger_ask().recalled_bill(run),
            None => Some(UsdMicros::default()),
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use crate::views::Views;
    use kernel::{
        Address, B3Hash, EventDraft, EventKind, EventRecord, NodeId, Payload, RunId, Seq, TimeMs,
    };

    fn record(seq: u64, run: RunId, kind: EventKind, data: serde_json::Value) -> EventRecord {
        EventRecord::from_draft(
            EventDraft {
                run,
                t: TimeMs::new(seq),
                who: "lab/parser".to_owned(),
                addr: Some(Address::parse("lab/parser").unwrap()),
                kind,
                data: Payload::new(data.as_object().unwrap().clone()).unwrap(),
                ig: false,
            },
            Seq::new(seq),
            B3Hash::digest(b"prev"),
        )
    }

    /// The join the card asks for: the node a run claimed, and what
    /// `storage::attribution` says that run was billed.
    #[test]
    fn a_claimed_node_costs_what_the_runs_that_held_it_were_billed() {
        let dir = tempfile::tempdir().unwrap();
        let mut views = Views::new(dir.path());
        let run = RunId::from_bytes([7u8; 16]);
        views
            .apply(&record(
                1,
                run,
                EventKind::RoadmapClaimed,
                serde_json::json!({ "by": "lab/parser", "node": "2.3", "verb": "claimed",
                                    "item": "the lexer" }),
            ))
            .unwrap();
        views
            .apply(&record(
                2,
                run,
                EventKind::ModelReturned,
                serde_json::json!({ "billed_usd_micros": 12_500 }),
            ))
            .unwrap();

        let node = NodeId::parse("2.3").unwrap();
        let wire::Answer::CostOf(answer) =
            views.answer(&wire::Query::CostOf { node: node.clone() })
        else {
            panic!("CostOf answers with a cost");
        };
        assert_eq!(answer.node, node);
        assert_eq!(answer.spent.get(), 12_500);
        assert_eq!(answer.runs, vec![(run, kernel::UsdMicros::new(12_500))]);
    }

    /// A node nobody claimed is answered, not refused: "no run has held
    /// this" is a fact, while `Unavailable` says the view could not look.
    #[test]
    fn a_node_nobody_claimed_costs_nothing_rather_than_being_unavailable() {
        let dir = tempfile::tempdir().unwrap();
        let mut views = Views::new(dir.path());
        let wire::Answer::CostOf(answer) = views.answer(&wire::Query::CostOf {
            node: NodeId::parse("9.1").unwrap(),
        }) else {
            panic!("an unclaimed node still has an answer");
        };
        assert_eq!(answer.spent.get(), 0);
        assert!(answer.runs.is_empty());
    }
}
