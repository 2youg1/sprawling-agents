// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use crate::outline::Role;
use crate::refusal::{Refusal, RefusalCode};
use std::collections::BTreeMap;

use super::focus::Aim;
use super::geometry::Bounds;

pub(crate) const MOST_REFS_PER_SNAPSHOT: usize = 500;

pub(crate) const DEFAULT_DEPTH: u32 = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Node {
    pub(crate) reference: String,
    pub(crate) role: Role,
    pub(crate) name: String,
    pub(crate) bounds: Bounds,
    pub(crate) depth: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Sight {
    pub(crate) aim: Aim,
    pub(crate) bounds: Bounds,
}

#[derive(Debug, Clone)]
struct Seen {
    generation: u64,
    bounds: Bounds,
    nodes: BTreeMap<String, Node>,
}

#[derive(Debug, Default)]
pub(crate) struct Views {
    windows: BTreeMap<Aim, Seen>,
}

impl Views {
    pub(crate) fn new() -> Views {
        Views {
            windows: BTreeMap::new(),
        }
    }

    pub(crate) fn mint(&mut self, sight: Sight, nodes: Vec<Node>) -> u64 {
        let generation = self
            .windows
            .get(&sight.aim)
            .map_or(1, |seen| seen.generation.saturating_add(1));
        let nodes = nodes
            .into_iter()
            .map(|node| (node.reference.clone(), node))
            .collect();
        self.windows.insert(
            sight.aim,
            Seen {
                generation,
                bounds: sight.bounds,
                nodes,
            },
        );
        generation
    }

    pub(crate) fn resolve(
        &self,
        sight: Sight,
        generation: u64,
        reference: &str,
    ) -> Result<&Node, Refusal> {
        let Some(seen) = self.windows.get(&sight.aim) else {
            return Err(stale(format!(
                "nothing has been snapshotted for this window, so `{reference}` names nothing"
            )));
        };
        if seen.generation != generation {
            return Err(stale(format!(
                "`{reference}` was decided against generation {generation}, and this window is \
                 now on generation {}",
                seen.generation
            )));
        }
        if seen.bounds != sight.bounds {
            return Err(stale(format!(
                "this window has moved or changed size since generation {generation} was taken"
            )));
        }
        seen.nodes.get(reference).ok_or_else(|| {
            stale(format!(
                "generation {generation} of this window minted no `{reference}`"
            ))
        })
    }

    pub(crate) fn generation(&self, aim: Aim) -> Option<u64> {
        self.windows.get(&aim).map(|seen| seen.generation)
    }
}

fn stale(because: String) -> Refusal {
    Refusal::new(
        RefusalCode::InvalidArgs,
        "act on a window",
        because,
        "take a fresh `desktop.snapshot` of this window and act on a ref from it, with the \
         generation it reports",
    )
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
    use super::*;

    const CALCULATOR: Aim = Aim(0x1000);
    const NOTEPAD: Aim = Aim(0x2000);

    fn at(aim: Aim, left: i32) -> Sight {
        Sight {
            aim,
            bounds: Bounds::from_corners(left, 0, left.saturating_add(400), 300).unwrap(),
        }
    }

    fn node(reference: &str, left: i32) -> Node {
        Node {
            reference: reference.to_owned(),
            role: Role::Button,
            name: "Save".to_owned(),
            bounds: Bounds::from_corners(left, 0, left.saturating_add(80), 30).unwrap(),
            depth: 1,
        }
    }

    #[test]
    fn a_ref_from_the_current_generation_resolves_to_where_it_was_seen() {
        let mut views = Views::new();
        let generation = views.mint(at(NOTEPAD, 0), vec![node("e1", 100), node("e2", 200)]);
        assert_eq!(generation, 1);
        let found = views.resolve(at(NOTEPAD, 0), 1, "e2").unwrap();
        assert_eq!(found.bounds.width(), 80);
        assert_eq!(found.role, Role::Button);
    }

    #[test]
    fn an_action_decided_against_an_older_view_is_refused_not_relocated() {
        let mut views = Views::new();
        views.mint(at(CALCULATOR, 0), vec![node("e1", 100)]);
        let second = views.mint(at(CALCULATOR, 0), vec![node("e1", 900)]);
        assert_eq!(second, 2);
        let refusal = views
            .resolve(at(CALCULATOR, 0), 1, "e1")
            .expect_err("generation 1 is spent");
        let error = refusal.as_error();
        assert_eq!(error["data"]["code"], "E_INVALID_ARGS");
        assert!(
            error["data"]["subject"]
                .as_str()
                .unwrap()
                .contains("now on generation 2")
        );
        assert!(
            error["data"]["recovery"]
                .as_str()
                .unwrap()
                .contains("fresh `desktop.snapshot`")
        );

        assert_eq!(
            views.resolve(at(CALCULATOR, 0), 2, "e1").unwrap().bounds,
            Bounds::from_corners(900, 0, 980, 30).unwrap()
        );
    }

    #[test]
    fn a_window_nobody_looked_at_and_a_ref_nobody_minted_are_both_refused() {
        let mut views = Views::new();
        assert!(views.generation(CALCULATOR).is_none());
        let unseen = views.resolve(at(CALCULATOR, 0), 1, "e1").unwrap_err();
        assert!(
            unseen.as_error()["data"]["subject"]
                .as_str()
                .unwrap()
                .contains("nothing has been snapshotted")
        );
        views.mint(at(CALCULATOR, 0), vec![node("e1", 100)]);
        assert_eq!(views.generation(CALCULATOR), Some(1));
        let unminted = views.resolve(at(CALCULATOR, 0), 1, "e99").unwrap_err();
        assert!(
            unminted.as_error()["data"]["subject"]
                .as_str()
                .unwrap()
                .contains("minted no `e99`")
        );
    }

    #[test]
    fn each_window_carries_its_own_generation() {
        let mut views = Views::new();
        views.mint(at(CALCULATOR, 0), vec![node("e1", 0)]);
        views.mint(at(NOTEPAD, 0), vec![node("e1", 0)]);
        views.mint(at(NOTEPAD, 0), vec![node("e1", 0)]);
        assert_eq!(views.generation(CALCULATOR), Some(1));
        assert_eq!(views.generation(NOTEPAD), Some(2));
        assert!(views.resolve(at(CALCULATOR, 0), 1, "e1").is_ok());
    }

    #[test]
    fn a_window_that_moved_since_its_snapshot_refuses_the_old_generation() {
        let mut views = Views::new();
        views.mint(at(CALCULATOR, 0), vec![node("e1", 100)]);
        assert_eq!(
            views.resolve(at(CALCULATOR, 50), 1, "e1").cloned(),
            Err(stale(
                "this window has moved or changed size since generation 1 was taken".to_owned()
            ))
        );
    }
}
