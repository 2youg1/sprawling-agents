// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Blocking queries: what a node waits for and the circles it waits in.

use std::collections::{BTreeMap, BTreeSet};

use crate::error::{AxCode, AxError};
use crate::node_id::NodeId;

use super::PlanNode;

pub(crate) fn refusal(
    action: &'static str,
    subject: String,
    recovery: impl Into<String>,
) -> AxError {
    AxError::failure(AxCode::InvalidArgs, action, subject).with_recovery(recovery)
}
pub(crate) fn first_cycle(nodes: &BTreeMap<NodeId, PlanNode>) -> Option<Vec<NodeId>> {
    let mut waiting: BTreeMap<&NodeId, usize> = nodes
        .iter()
        .map(|(id, node)| (id, node.row.needs.len()))
        .collect();
    let mut queue: Vec<&NodeId> = waiting
        .iter()
        .filter(|(_, count)| **count == 0)
        .map(|(id, _)| *id)
        .collect();
    while let Some(free) = queue.pop() {
        waiting.remove(free);
        for (id, node) in nodes {
            if !node.row.needs.contains(free) {
                continue;
            }
            if let Some(count) = waiting.get_mut(id) {
                *count = count.saturating_sub(1);
                if *count == 0 {
                    queue.push(id);
                }
            }
        }
    }
    let stuck: BTreeSet<&NodeId> = waiting.keys().copied().collect();
    let start = *stuck.iter().next()?;
    let mut walk = vec![start.clone()];
    let mut here = start;
    // Bounded by the number of stuck nodes: a walk that long has
    // already repeated a node, and the repeat is the circle.
    for _ in 0..stuck.len() {
        let next = nodes
            .get(here)?
            .row
            .needs
            .iter()
            .find(|need| stuck.contains(need))?;
        walk.push(next.clone());
        if next == start {
            break;
        }
        here = next;
    }
    Some(walk)
}
