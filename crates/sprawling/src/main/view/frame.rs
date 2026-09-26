// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The state of the person's face of `sprawling view` and the frame it
//! draws (sprawling-SPEC.md 8-91). Both lenses share one selected
//! thing; nothing here touches the terminal or the disk.

use std::collections::BTreeSet;

use kernel::{RunId, Seq};
use sprawling::lineage::RunLine;

use super::arrange::{Entry, NodeKey, arrange};
use super::detail::json_lines;
use super::keys::Action;

/// From this many columns on, the detail pane stays open on the right.
pub(super) const SIDE_PANE_MIN_WIDTH: usize = 110;

/// One Ledger line of the `records` lens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Row {
    pub(super) seq: Seq,
    pub(super) run: RunId,
    pub(super) line: String,
}

/// The terminal's size in character cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Size {
    pub(super) columns: usize,
    pub(super) rows: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Lens {
    Tree,
    Records,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Detail {
    Pane,
    Full,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Life {
    Open,
    Closed,
}

pub(super) struct Face {
    entries: Vec<Entry>,
    records: Vec<Row>,
    lens: Lens,
    tree_at: usize,
    record_at: usize,
    expanded: BTreeSet<usize>,
    detail: Detail,
    life: Life,
    size: Size,
}

impl Face {
    /// The tree lens, the cursor where the person is most likely needed:
    /// the latest active run, else the latest run, else the city.
    pub(super) fn open(runs: &[RunLine], records: Vec<Row>, size: Size) -> Face {
        let entries = arrange(runs);
        let latest = |active_only: bool| {
            runs.iter()
                .filter(|line| {
                    !active_only || line.state == Some(memory::RunPhase::Active)
                })
                .max_by_key(|line| line.first_seq)
                .map(|line| line.run)
        };
        let chosen = latest(true).or_else(|| latest(false));
        let tree_at = chosen
            .and_then(|run| entries.iter().position(|entry| entry.key == NodeKey::Run(run)))
            .unwrap_or(0);
        let mut face = Face {
            entries,
            records,
            lens: Lens::Tree,
            tree_at: 0,
            record_at: 0,
            expanded: BTreeSet::new(),
            detail: Detail::Pane,
            life: Life::Open,
            size,
        };
        face.select_entry(tree_at);
        face
    }

    pub(super) fn apply(&mut self, action: Action) {
        let _ = action;
    }

    pub(super) fn resize(&mut self, size: Size) {
        self.size = size;
    }

    pub(super) fn is_closed(&self) -> bool {
        self.life == Life::Closed
    }

    pub(super) fn lens(&self) -> Lens {
        self.lens
    }

    /// The frame for the current size, one string per terminal row.
    pub(super) fn frame(&self) -> Vec<String> {
        let Size { columns, rows } = self.size;
        let detail = self.detail_lines();
        if self.detail == Detail::Full {
            return detail.iter().take(rows).map(|line| cut(line, columns)).collect();
        }
        let list = self.list_lines(rows);
        if columns < SIDE_PANE_MIN_WIDTH {
            return list.iter().map(|line| cut(line, columns)).collect();
        }
        let left = columns / 2;
        let right = columns.saturating_sub(left).saturating_sub(1);
        (0..rows)
            .map(|at| {
                let side = |lines: &[String]| lines.get(at).map_or("", String::as_str).to_owned();
                format!("{:<left$}|{}", cut(&side(&list), left), cut(&side(&detail), right))
            })
            .collect()
    }

    fn visible(&self) -> Vec<usize> {
        let mut shown = Vec::new();
        for (at, entry) in self.entries.iter().enumerate() {
            if entry.parent.is_none_or(|parent| self.is_open(parent)) {
                shown.push(at);
            }
        }
        shown
    }

    fn is_open(&self, at: usize) -> bool {
        let mut cursor = Some(at);
        while let Some(at) = cursor {
            if !self.expanded.contains(&at) {
                return false;
            }
            cursor = self.entries.get(at).and_then(|entry| entry.parent);
        }
        true
    }

    fn has_children(&self, at: usize) -> bool {
        self.entries
            .get(at.saturating_add(1))
            .is_some_and(|next| next.parent == Some(at))
    }

    /// Selects entry `at` in the tree and opens every ancestor of it.
    fn select_entry(&mut self, at: usize) {
        self.tree_at = at;
        let mut parent = self.entries.get(at).and_then(|entry| entry.parent);
        while let Some(up) = parent {
            self.expanded.insert(up);
            parent = self.entries.get(up).and_then(|entry| entry.parent);
        }
    }

    fn list_lines(&self, rows: usize) -> Vec<String> {
        let (lines, cursor): (Vec<String>, usize) = match self.lens {
            Lens::Tree => {
                let shown = self.visible();
                let cursor = shown.iter().position(|at| *at == self.tree_at).unwrap_or(0);
                let lines = shown
                    .iter()
                    .filter_map(|at| self.entries.get(*at).map(|entry| (*at, entry)))
                    .map(|(at, entry)| {
                        let mark = match (self.has_children(at), self.expanded.contains(&at)) {
                            (false, _) => ' ',
                            (true, true) => '-',
                            (true, false) => '+',
                        };
                        format!("{}{mark} {}", "  ".repeat(entry.depth), entry.label)
                    })
                    .collect();
                (lines, cursor)
            }
            Lens::Records => (
                self.records.iter().map(|row| row.line.clone()).collect(),
                self.record_at,
            ),
        };
        let skip = cursor.saturating_add(1).saturating_sub(rows);
        lines
            .into_iter()
            .enumerate()
            .skip(skip)
            .take(rows)
            .map(|(at, line)| format!("{}{line}", if at == cursor { '>' } else { ' ' }))
            .collect()
    }

    fn detail_lines(&self) -> Vec<String> {
        match self.lens {
            Lens::Tree => self
                .entries
                .get(self.tree_at)
                .map(|entry| json_lines(&entry.detail))
                .unwrap_or_default(),
            Lens::Records => self
                .records
                .get(self.record_at)
                .map(|row| {
                    json_lines(
                        &serde_json::from_str(&row.line)
                            .unwrap_or_else(|_| serde_json::Value::String(row.line.clone())),
                    )
                })
                .unwrap_or_default(),
        }
    }
}

fn cut(line: &str, width: usize) -> String {
    line.chars().take(width).collect()
}
