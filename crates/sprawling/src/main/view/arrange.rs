// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The lineage of a city arranged as the tree the person reads:
//! city › building › room › session › run › round › call, a fork under
//! the run it forked from (sprawling-SPEC.md 8-91). Every node has one
//! parent.

use std::collections::BTreeMap;

use kernel::{Address, RunId, Seq};
use serde_json::json;
use sprawling::lineage::RunLine;

use super::rounds::{Rounds, append_below};

/// Which node an entry is; two entries never share a key.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum NodeKey {
    City,
    Building(String),
    Room(Address),
    Session(Address, Option<Seq>),
    Run(RunId),
    Round(RunId, u32),
    Call(RunId, Seq),
}

/// One node, in display order.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Entry {
    pub(super) key: NodeKey,
    pub(super) depth: usize,
    pub(super) parent: Option<usize>,
    /// The earliest `first_seq` in this node's subtree.
    pub(super) seq: Seq,
    pub(super) label: String,
    pub(super) detail: serde_json::Value,
}

/// The tree of `runs` flattened depth first, the city at index 0;
/// children in the order their earliest run started, a folded run's
/// rounds before its forks.
pub(super) fn arrange(runs: &[RunLine], rounds: &Rounds) -> Vec<Entry> {
    let mut ordered: Vec<&RunLine> = runs.iter().collect();
    ordered.sort_by_key(|line| line.first_seq);
    let known: BTreeMap<RunId, &RunLine> = ordered.iter().map(|line| (line.run, *line)).collect();
    let mut children: BTreeMap<NodeKey, Vec<NodeKey>> = BTreeMap::new();
    let mut first: BTreeMap<NodeKey, Seq> = BTreeMap::new();
    for line in &ordered {
        let mut chain = vec![NodeKey::Run(line.run)];
        match (
            line.parent.filter(|parent| known.contains_key(parent)),
            &line.addr,
        ) {
            (Some(parent), _) => chain.push(NodeKey::Run(parent)),
            (None, Some(addr)) => chain.extend([
                NodeKey::Session(addr.clone(), line.session),
                NodeKey::Room(addr.clone()),
                NodeKey::Building(building_of(addr)),
                NodeKey::City,
            ]),
            (None, None) => chain.push(NodeKey::City),
        }
        for pair in chain.windows(2) {
            if let [child, parent] = pair {
                let siblings = children.entry(parent.clone()).or_default();
                if !siblings.contains(child) {
                    siblings.push(child.clone());
                }
            }
        }
        for key in chain {
            first.entry(key).or_insert(line.first_seq);
        }
    }
    let mut entries = Vec::new();
    let mut stack = vec![(NodeKey::City, 0_usize, None::<usize>)];
    while let Some((key, depth, parent)) = stack.pop() {
        let at = entries.len();
        let seq = first.get(&key).copied().unwrap_or(Seq::new(0));
        let (label, detail) = describe(&key, &known, runs.len());
        let below = depth.saturating_add(1);
        stack.extend(
            children
                .get(&key)
                .into_iter()
                .flatten()
                .rev()
                .map(|child| (child.clone(), below, Some(at))),
        );
        let folded = if let NodeKey::Run(run) = &key {
            rounds.get(run)
        } else {
            None
        };
        entries.push(Entry {
            key,
            depth,
            parent,
            seq,
            label,
            detail,
        });
        if let Some(folded) = folded {
            append_below(&mut entries, at, folded);
        }
    }
    entries
}

fn building_of(addr: &Address) -> String {
    addr.as_str()
        .split_once('/')
        .map_or(addr.as_str(), |(building, _)| building)
        .to_owned()
}

fn describe(
    key: &NodeKey,
    known: &BTreeMap<RunId, &RunLine>,
    run_count: usize,
) -> (String, serde_json::Value) {
    match key {
        NodeKey::City => ("city".to_owned(), json!({ "runs": run_count })),
        NodeKey::Building(name) => (name.clone(), json!({ "building": name })),
        NodeKey::Room(addr) => (addr.to_string(), json!({ "addr": addr.as_str() })),
        NodeKey::Session(addr, session) => (
            session.map_or_else(
                || "session (first stretch)".to_owned(),
                |seq| format!("session @{}", seq.value()),
            ),
            json!({ "addr": addr.as_str(), "session": session.map(|seq| seq.value()) }),
        ),
        NodeKey::Run(run) => known.get(run).map_or_else(
            || (run.to_string(), json!({ "run": run.to_string() })),
            |line| (run_label(line), line.to_json()),
        ),
        NodeKey::Round(..) | NodeKey::Call(..) => (String::new(), serde_json::Value::Null),
    }
}

fn run_label(line: &RunLine) -> String {
    let state = match line.state {
        Some(memory::RunPhase::Active) => "active",
        Some(memory::RunPhase::Frozen) => "frozen",
        None => "ended",
    };
    let mut label = format!(
        "run {} {state} #{}..{}",
        line.run,
        line.first_seq.value(),
        line.last_seq.value()
    );
    if let Some(at) = line.forked_at {
        label.push_str(&format!(" fork @{}", at.value()));
    }
    if let Some(predecessor) = line.predecessor {
        label.push_str(&format!(" after {predecessor}"));
    }
    label
}
