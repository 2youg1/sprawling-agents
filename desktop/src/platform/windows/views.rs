// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a snapshot minted, which generation a window is on, and the
//! action an older view cannot buy.
//!
//! A `ref` here means one thing and it is worth stating exactly: **the
//! place an element occupied on the screen at the moment the snapshot
//! was taken**. Nothing is held open across the call — the tree walk
//! keeps its COM objects to itself (desktop-SPEC.md §8.6, second pair) —
//! so a ref is a rectangle and a generation, and that is the whole of
//! it.
//!
//! From which the rule follows rather than being decreed: a window that
//! is snapshotted again is on a new generation, and every ref of the
//! generation before it is refused. An action decided while looking at
//! one arrangement of a window does not land on a different arrangement
//! of it; it is refused, and the caller looks again. `browser::act`
//! holds the same line for a page, and the reason is the same one.
//!
//! A `point` carries no generation check, because a point did not come
//! from a snapshot: the caller measured it against the window's own
//! edges, and `geometry::Bounds::at` is what judges it.

use crate::refusal::{Refusal, RefusalCode};
use std::collections::BTreeMap;

use super::geometry::Bounds;

/// How many refs one snapshot mints before it stops walking. A tree a
/// model cannot read to the end is a tree it has not read.
pub(crate) const MOST_REFS_PER_SNAPSHOT: usize = 500;

/// How deep a snapshot goes when the caller does not say.
pub(crate) const DEFAULT_DEPTH: u32 = 8;

/// One node of a window's tree, as a caller reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Node {
    pub(crate) reference: String,
    pub(crate) role: String,
    pub(crate) name: String,
    pub(crate) bounds: Bounds,
    pub(crate) depth: u32,
}

/// What one window's last snapshot left behind.
#[derive(Debug, Clone)]
struct Seen {
    generation: u64,
    nodes: BTreeMap<String, Node>,
}

/// Every window this connection has looked at.
///
/// Keyed by the window's title, which is what the caller names it by, so
/// a window renamed between two calls is a window this table has not
/// seen — which is the honest answer rather than a stale one.
#[derive(Debug, Default)]
pub(crate) struct Views {
    windows: BTreeMap<String, Seen>,
}

impl Views {
    pub(crate) fn new() -> Views {
        Views {
            windows: BTreeMap::new(),
        }
    }

    /// Records a fresh snapshot of one window and answers with the
    /// generation it is now on.
    ///
    /// The generation only ever rises, so a caller holding an old one
    /// cannot have it come back around: `saturating_add` stops rather
    /// than wraps, and a connection that took eighteen quintillion
    /// snapshots has a problem this counter is not it.
    pub(crate) fn mint(&mut self, window: &str, nodes: Vec<Node>) -> u64 {
        let generation = self
            .windows
            .get(window)
            .map_or(1, |seen| seen.generation.saturating_add(1));
        let nodes = nodes
            .into_iter()
            .map(|node| (node.reference.clone(), node))
            .collect();
        self.windows
            .insert(window.to_owned(), Seen { generation, nodes });
        generation
    }

    /// Where a ref decided against `generation` currently is.
    ///
    /// # Errors
    /// Refuses a window nobody has snapshotted, a generation that is not
    /// this window's current one, and a ref that generation did not
    /// mint.
    pub(crate) fn resolve(
        &self,
        window: &str,
        generation: u64,
        reference: &str,
    ) -> Result<&Node, Refusal> {
        let Some(seen) = self.windows.get(window) else {
            return Err(stale(format!(
                "nothing has been snapshotted for `{window}`, so `{reference}` names nothing"
            )));
        };
        if seen.generation != generation {
            return Err(stale(format!(
                "`{reference}` was decided against generation {generation}, and `{window}` is \
                 now on generation {}",
                seen.generation
            )));
        }
        seen.nodes.get(reference).ok_or_else(|| {
            stale(format!(
                "generation {generation} of `{window}` minted no `{reference}`"
            ))
        })
    }

    /// The generation a window is on, for a caller that asks what it is
    /// holding.
    pub(crate) fn generation(&self, window: &str) -> Option<u64> {
        self.windows.get(window).map(|seen| seen.generation)
    }
}

/// One recovery sentence, because there is one thing to do about every
/// one of these: look again.
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

    fn node(reference: &str, left: i32) -> Node {
        Node {
            reference: reference.to_owned(),
            role: "Button".to_owned(),
            name: "Save".to_owned(),
            bounds: Bounds::from_corners(left, 0, left.saturating_add(80), 30).unwrap(),
            depth: 1,
        }
    }

    #[test]
    fn a_ref_from_the_current_generation_resolves_to_where_it_was_seen() {
        let mut views = Views::new();
        let generation = views.mint("a.txt — Notepad", vec![node("e1", 100), node("e2", 200)]);
        assert_eq!(generation, 1);
        let found = views.resolve("a.txt — Notepad", 1, "e2").unwrap();
        assert_eq!(found.bounds.width(), 80);
        assert_eq!(found.role, "Button");
    }

    /// The rule this module exists for. A second snapshot retires the
    /// first, and an action decided against the first is refused rather
    /// than landing on whatever moved into that position.
    #[test]
    fn an_action_decided_against_an_older_view_is_refused_not_relocated() {
        let mut views = Views::new();
        views.mint("Calculator", vec![node("e1", 100)]);
        let second = views.mint("Calculator", vec![node("e1", 900)]);
        assert_eq!(second, 2);
        let refusal = views
            .resolve("Calculator", 1, "e1")
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
        // The current generation still resolves, and to the new place.
        assert_eq!(
            views.resolve("Calculator", 2, "e1").unwrap().bounds,
            Bounds::from_corners(900, 0, 980, 30).unwrap()
        );
    }

    #[test]
    fn a_window_nobody_looked_at_and_a_ref_nobody_minted_are_both_refused() {
        let mut views = Views::new();
        assert!(views.generation("Calculator").is_none());
        let unseen = views.resolve("Calculator", 1, "e1").unwrap_err();
        assert!(
            unseen.as_error()["data"]["subject"]
                .as_str()
                .unwrap()
                .contains("nothing has been snapshotted")
        );
        views.mint("Calculator", vec![node("e1", 100)]);
        assert_eq!(views.generation("Calculator"), Some(1));
        let unminted = views.resolve("Calculator", 1, "e99").unwrap_err();
        assert!(
            unminted.as_error()["data"]["subject"]
                .as_str()
                .unwrap()
                .contains("minted no `e99`")
        );
    }

    /// Two windows do not share a generation counter: looking at one
    /// does not retire what was decided about the other.
    #[test]
    fn each_window_carries_its_own_generation() {
        let mut views = Views::new();
        views.mint("Calculator", vec![node("e1", 0)]);
        views.mint("a.txt — Notepad", vec![node("e1", 0)]);
        views.mint("a.txt — Notepad", vec![node("e1", 0)]);
        assert_eq!(views.generation("Calculator"), Some(1));
        assert_eq!(views.generation("a.txt — Notepad"), Some(2));
        assert!(views.resolve("Calculator", 1, "e1").is_ok());
    }
}
