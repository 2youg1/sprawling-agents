// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The state of the person's face of `sprawling view` and the frame it
//! draws (sprawling-SPEC.md 8-117). Both lenses share one selected
//! thing; nothing here touches the terminal or the disk.

use std::collections::BTreeSet;

use kernel::RunId;
use sprawling::lineage::RunLine;

use super::arrange::{Entry, NodeKey, arrange};
use super::detail::{json_lines, line_lines};
use super::follow::Row;
use super::keys::Action;
use super::list::{beside, cut, has_children, record_lines, scrolled, tree_lines, visible};
use super::rounds::{Rounds, fold};

/// From this many columns on, the detail pane stays open on the right.
pub(super) const SIDE_PANE_MIN_WIDTH: usize = 110;

/// The status line while the lines older than the first window are
/// still being folded.
pub(super) const FILLING: &str = "filling in the older ledger lines";

/// The terminal's size in character cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Size {
    pub(super) columns: usize,
    pub(super) rows: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Lens {
    Tree,
    Records,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Detail {
    Pane,
    Full,
}

/// How far a movement goes; `usize::MAX` stops at the end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    Back(usize),
    On(usize),
}

/// How much of the ledger `records` holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Reach {
    Window,
    Whole,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Life {
    Open,
    Closed,
}

pub(super) struct Face {
    runs: Vec<RunLine>,
    rounds: Rounds,
    entries: Vec<Entry>,
    records: Vec<Row>,
    lens: Lens,
    tree_at: usize,
    record_at: usize,
    expanded: BTreeSet<usize>,
    detail: Detail,
    life: Life,
    reach: Reach,
    size: Size,
}

impl Face {
    /// The tree lens, the cursor where the person is most likely needed:
    /// the latest run waiting on the person's answer, else the latest
    /// active run, else the latest run, else the city.
    pub(super) fn open(runs: &[RunLine], records: Vec<Row>, size: Size) -> Face {
        let entries = arrange(runs, &Rounds::new());
        let latest = |keep: fn(&RunLine) -> bool| {
            runs.iter()
                .filter(|line| keep(line))
                .max_by_key(|line| line.first_seq)
                .map(|line| line.run)
        };
        let chosen = latest(|line| line.unanswered > 0)
            .or_else(|| latest(|line| line.state == Some(storage::RunPhase::Active)))
            .or_else(|| latest(|_| true));
        let tree_at = chosen
            .and_then(|run| {
                entries
                    .iter()
                    .position(|entry| entry.key == NodeKey::Run(run))
            })
            .unwrap_or(0);
        let mut face = Face {
            runs: runs.to_vec(),
            rounds: Rounds::new(),
            entries,
            records,
            lens: Lens::Tree,
            tree_at: 0,
            record_at: 0,
            expanded: BTreeSet::new(),
            detail: Detail::Pane,
            life: Life::Open,
            reach: Reach::Whole,
            size,
        };
        face.select_entry(tree_at);
        face
    }

    /// The face over the newest lines only, while the whole fold runs.
    pub(super) fn open_window(runs: &[RunLine], window: Vec<Row>, size: Size) -> Face {
        Face {
            reach: Reach::Window,
            ..Face::open(runs, window, size)
        }
    }

    /// Takes the whole fold in place of the window: the lineage, and every
    /// line, the ones older than the window before it. The same node and
    /// the same line stay selected, and every opened run is folded again.
    pub(super) fn fill(&mut self, runs: &[RunLine], whole: Vec<Row>) {
        let line_seq = self.records.get(self.record_at).map(|row| row.seq);
        self.records = whole;
        self.record_at = self.first_record_from(line_seq);
        let opened: Vec<RunId> = self.rounds.keys().copied().collect();
        for run in opened {
            self.rounds.insert(run, fold(run, &self.records));
        }
        self.runs = runs.to_vec();
        self.reach = Reach::Whole;
        self.rearrange();
        self.open_ancestors(self.tree_at);
    }

    pub(super) fn apply(&mut self, action: Action) {
        let page = self.size.rows.max(1);
        match action {
            Action::Up => self.step(Step::Back(1)),
            Action::Down => self.step(Step::On(1)),
            Action::PageUp => self.step(Step::Back(page)),
            Action::PageDown => self.step(Step::On(page)),
            Action::First => self.step(Step::Back(usize::MAX)),
            Action::Last => self.step(Step::On(usize::MAX)),
            Action::Collapse => self.collapse(),
            Action::Expand => self.expand(),
            Action::SwitchLens => self.switch_lens(),
            Action::OpenDetail => self.detail = Detail::Full,
            Action::CloseDetail => self.detail = Detail::Pane,
            Action::Quit => self.life = Life::Closed,
        }
    }

    pub(super) fn resize(&mut self, size: Size) {
        self.size = size;
    }

    /// Takes the lineage and the lines appended since the last look, and
    /// folds again every opened run the new lines belong to.
    pub(super) fn follow(&mut self, runs: &[RunLine], appended: Vec<Row>) {
        let stale: BTreeSet<RunId> = appended
            .iter()
            .map(|row| row.run)
            .filter(|run| self.rounds.contains_key(run))
            .collect();
        self.records.extend(appended);
        for run in stale {
            self.rounds.insert(run, fold(run, &self.records));
        }
        self.runs = runs.to_vec();
        self.rearrange();
    }

    /// Arranges the tree again: the same node stays selected and the same
    /// nodes stay open, and every run the tree did not have opens its
    /// ancestors so it can be seen.
    fn rearrange(&mut self) {
        let selected = self
            .entries
            .get(self.tree_at)
            .map(|entry| entry.key.clone());
        let open: BTreeSet<NodeKey> = self
            .expanded
            .iter()
            .filter_map(|at| self.entries.get(*at).map(|entry| entry.key.clone()))
            .collect();
        let known: BTreeSet<NodeKey> = self
            .entries
            .iter()
            .filter(|entry| matches!(entry.key, NodeKey::Run(_)))
            .map(|entry| entry.key.clone())
            .collect();
        self.entries = arrange(&self.runs, &self.rounds);
        self.expanded = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| open.contains(&entry.key))
            .map(|(at, _)| at)
            .collect();
        let fresh: Vec<usize> = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| {
                matches!(entry.key, NodeKey::Run(_)) && !known.contains(&entry.key)
            })
            .map(|(at, _)| at)
            .collect();
        for at in fresh {
            self.open_ancestors(at);
        }
        self.tree_at = selected
            .and_then(|key| self.entries.iter().position(|entry| entry.key == key))
            .unwrap_or(0);
    }

    pub(super) fn is_closed(&self) -> bool {
        self.life == Life::Closed
    }

    /// The frame for the current size, one string per terminal row; the
    /// last one is the status line while the window is being filled.
    pub(super) fn frame(&self) -> Vec<String> {
        match self.reach {
            Reach::Whole => self.body(self.size.rows),
            Reach::Window => {
                let rows = self.size.rows.saturating_sub(1);
                let mut lines = self.body(rows);
                lines.resize(rows, String::new());
                lines.push(cut(FILLING, self.size.columns));
                lines
            }
        }
    }

    fn body(&self, rows: usize) -> Vec<String> {
        let columns = self.size.columns;
        let detail = self.detail_lines();
        if self.detail == Detail::Full {
            return detail
                .iter()
                .take(rows)
                .map(|line| cut(line, columns))
                .collect();
        }
        let list = self.list_lines(rows);
        if columns < SIDE_PANE_MIN_WIDTH {
            return list.iter().map(|line| cut(line, columns)).collect();
        }
        beside(&list, &detail, columns, rows)
    }

    fn step(&mut self, step: Step) {
        let move_by = |at: usize, len: usize| match step {
            Step::Back(by) => at.saturating_sub(by),
            Step::On(by) => at.saturating_add(by).min(len.saturating_sub(1)),
        };
        match self.lens {
            Lens::Tree => {
                let shown = visible(&self.entries, &self.expanded);
                let at = shown.iter().position(|at| *at == self.tree_at).unwrap_or(0);
                if let Some(entry) = shown.get(move_by(at, shown.len())) {
                    self.tree_at = *entry;
                }
            }
            Lens::Records => self.record_at = move_by(self.record_at, self.records.len()),
        }
    }

    /// Folds an open node; on a folded or childless one, climbs to its
    /// parent. The records lens has nothing to fold.
    fn collapse(&mut self) {
        if self.lens == Lens::Records {
            return;
        }
        if !self.expanded.remove(&self.tree_at)
            && let Some(parent) = self
                .entries
                .get(self.tree_at)
                .and_then(|entry| entry.parent)
        {
            self.tree_at = parent;
        }
    }

    /// Unfolds a folded node; on an open one, descends to its first child.
    fn expand(&mut self) {
        if self.lens == Lens::Records {
            return;
        }
        if let Some(&NodeKey::Run(run)) = self.entries.get(self.tree_at).map(|entry| &entry.key)
            && !self.rounds.contains_key(&run)
        {
            self.rounds.insert(run, fold(run, &self.records));
            self.rearrange();
        }
        if !has_children(&self.entries, self.tree_at) {
            return;
        }
        if !self.expanded.insert(self.tree_at) {
            self.tree_at = self.tree_at.saturating_add(1);
        }
    }

    /// Moves to the other lens with the same thing selected: a node's
    /// earliest line, or the run node a line belongs to.
    fn switch_lens(&mut self) {
        match self.lens {
            Lens::Tree => {
                self.record_at =
                    self.first_record_from(self.entries.get(self.tree_at).map(|entry| entry.seq));
                self.lens = Lens::Records;
            }
            Lens::Records => {
                let key = self.records.get(self.record_at).map(|row| {
                    if row.run == RunId::CITY {
                        NodeKey::City
                    } else {
                        NodeKey::Run(row.run)
                    }
                });
                if let Some(at) =
                    key.and_then(|key| self.entries.iter().position(|entry| entry.key == key))
                {
                    self.select_entry(at);
                }
                self.lens = Lens::Tree;
            }
        }
    }

    /// The first line at or after `seq`, or the last line.
    fn first_record_from(&self, seq: Option<kernel::Seq>) -> usize {
        seq.map_or(0, |seq| {
            self.records
                .partition_point(|row| row.seq < seq)
                .min(self.records.len().saturating_sub(1))
        })
    }

    /// Selects entry `at` in the tree and opens every ancestor of it.
    fn select_entry(&mut self, at: usize) {
        self.tree_at = at;
        self.open_ancestors(at);
    }

    fn open_ancestors(&mut self, at: usize) {
        let mut parent = self.entries.get(at).and_then(|entry| entry.parent);
        while let Some(up) = parent {
            self.expanded.insert(up);
            parent = self.entries.get(up).and_then(|entry| entry.parent);
        }
    }

    fn list_lines(&self, rows: usize) -> Vec<String> {
        match self.lens {
            Lens::Tree => {
                let shown = visible(&self.entries, &self.expanded);
                let cursor = shown.iter().position(|at| *at == self.tree_at).unwrap_or(0);
                let lines = tree_lines(&self.entries, &shown, &self.expanded, &self.rounds);
                scrolled(lines, cursor, rows)
            }
            Lens::Records => record_lines(&self.records, self.record_at, rows),
        }
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
                .map(|row| line_lines(&row.line))
                .unwrap_or_default(),
        }
    }
}
