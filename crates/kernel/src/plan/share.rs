// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Share division: weight is conserved because [`crate::share`] is the only way to hold any.

use crate::error::AxError;
use crate::node_id::NodeId;
use crate::share::Share;

use super::PlanTree;

/// Gives `pot` to `among` by their weights, then recurses.
pub(crate) fn hand_out(tree: &mut PlanTree, pot: Share, among: &[NodeId]) -> Result<(), AxError> {
    if among.is_empty() {
        return Ok(());
    }
    let weights: Vec<u32> = among
        .iter()
        .map(|id| tree.nodes.get(id).map_or(1, |node| node.row.weight))
        .collect();
    // Every weight zero is a table that says nothing about how this
    // level divides, and an even cut is the reading that keeps the
    // share where the rows are.
    let parts = if weights.iter().all(|weight| *weight == 0) {
        pot.split(&vec![1; among.len()])?
    } else {
        pot.split(&weights)?
    };
    for (id, part) in among.iter().zip(parts) {
        let children = {
            let Some(node) = tree.nodes.get_mut(id) else {
                continue;
            };
            node.share = part;
            node.children.clone()
        };
        hand_out(tree, part, &children)?;
    }
    Ok(())
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests {
    use super::super::PlanTree;
    use super::*;
    use crate::share;
    use crate::spine::{RoadmapShape, check_roadmap_shape};
    fn tree(text: &str) -> PlanTree {
        let RoadmapShape::WellFormed { rows } = check_roadmap_shape(text) else {
            panic!("the fixture parses");
        };
        PlanTree::build(rows).expect("the fixture is a tree")
    }
    const HEAD: &str = "\
| # | Item | Weight | Needs | Status | Evidence |
|---|------|--------|-------|--------|----------|
";
    #[test]
    fn a_branch_hands_its_whole_share_to_its_children() {
        let plan = tree(&format!(
            "{HEAD}\
| 1 | build | 1 |  | Not started | |
| 1.1 | design | 1 |  | Not started | |
| 1.2 | code | 3 |  | Not started | |
| 2 | ship | 1 |  | Not started | |
"
        ));
        let share_of = |raw: &str| plan.get(&NodeId::parse(raw).unwrap()).unwrap().share;
        assert_eq!(share_of("1").ppb(), 500_000_000);
        assert_eq!(share_of("2").ppb(), 500_000_000);
        assert_eq!(share_of("1.1").ppb(), 125_000_000);
        assert_eq!(share_of("1.2").ppb(), 375_000_000);
        assert_eq!(
            share::gather(&[share_of("1.1"), share_of("1.2")]),
            share_of("1"),
            "the children add up to the branch"
        );
    }

    /// The card's own claim, in numbers: splitting a branch cannot take
    /// anything from its neighbours.
    /// The card's own claim, in numbers: splitting a branch cannot take
    /// anything from its neighbours.
    #[test]
    fn dividing_a_branch_generously_takes_nothing_from_the_others() {
        let before = tree(&format!(
            "{HEAD}\
| 1 | build | 1 |  | Not started | |
| 2 | ship | 1 |  | Not started | |
"
        ));
        let after = tree(&format!(
            "{HEAD}\
| 1 | build | 1 |  | Not started | |
| 1.1 | a | 900 |  | Not started | |
| 1.2 | b | 100 |  | Not started | |
| 2 | ship | 1 |  | Not started | |
"
        ));
        let two = NodeId::parse("2").unwrap();
        assert_eq!(
            before.get(&two).unwrap().share,
            after.get(&two).unwrap().share
        );
    }
}
