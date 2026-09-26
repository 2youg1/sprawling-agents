// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Every run of a city folded into one line that carries its parent
//! pointers (sprawling-SPEC.md 8-93).
//!
//! This is the projection the `tree` lens of `sprawling view` draws:
//! the agent reads it as JSON lines, the interactive viewer arranges it
//! as a tree. Whether a run is still working is not decided here; it is
//! read off `memory::HotView`, the same fold the city's run list answers
//! from, so the tree and the run list cannot disagree about it.

use std::collections::BTreeMap;
use std::path::Path;

use kernel::event::record::{RunForked, RunStarted};
use kernel::{Address, AxError, EventKind, EventRecord, RunId, Seq};

/// One run, and where it hangs in the lineage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunLine {
    pub run: RunId,
    /// The room the run worked in, from its `run_started` record.
    pub addr: Option<Address>,
    /// The seq of the `session_opened` line that began the stretch this
    /// run started in; `None` for a room's first stretch.
    pub session: Option<Seq>,
    /// `run_forked.from` when the run is a fork, otherwise
    /// `run_started.parent`.
    pub parent: Option<RunId>,
    /// `run_forked.at_seq`: the last line of the parent the fork inherits.
    pub forked_at: Option<Seq>,
    /// `run_started.predecessor`: the run this one took over from.
    pub predecessor: Option<RunId>,
    pub first_seq: Seq,
    pub last_seq: Seq,
    pub state: Option<memory::RunPhase>,
}

impl RunLine {
    /// The line `sprawling view --runs` writes for this run.
    pub fn to_json(&self) -> serde_json::Value {
        let state = self.state.as_ref().map(|phase| match phase {
            memory::RunPhase::Active => "active",
            memory::RunPhase::Frozen => "frozen",
        });
        serde_json::json!({
            "run": self.run.to_string(),
            "addr": self.addr.as_ref().map(Address::as_str),
            "session": self.session.map(|seq| seq.value()),
            "parent": self.parent.map(|run| run.to_string()),
            "forked_at": self.forked_at.map(|seq| seq.value()),
            "predecessor": self.predecessor.map(|run| run.to_string()),
            "first_seq": self.first_seq.value(),
            "last_seq": self.last_seq.value(),
            "state": state,
        })
    }
}

/// The fold: every run seen so far, and the latest stretch of each room.
#[derive(Default)]
pub struct Lineage {
    runs: BTreeMap<RunId, RunLine>,
    stretches: BTreeMap<Address, Seq>,
    hot: memory::HotView,
}

impl Lineage {
    /// Folds one record in, in seq order.
    ///
    /// # Errors
    /// Refuses a `run_started` or `run_forked` whose payload cannot be
    /// read, and whatever `memory::HotView` refuses.
    #[expect(
        clippy::wildcard_enum_match_arm,
        reason = "two kinds carry lineage; every other kind only moves last_seq"
    )]
    pub fn apply(&mut self, record: &EventRecord) -> Result<(), AxError> {
        self.hot
            .apply(record)
            .map_err(memory::MemoryError::into_ax)?;
        let seq = record.seq();
        if record.kind() == EventKind::SessionOpened
            && let Some(addr) = record.addr()
        {
            self.stretches.insert(addr.clone(), seq);
        }
        let run = record.run();
        if run == RunId::CITY {
            return Ok(());
        }
        let line = self.runs.entry(run).or_insert_with(|| RunLine {
            run,
            addr: None,
            session: None,
            parent: None,
            forked_at: None,
            predecessor: None,
            first_seq: seq,
            last_seq: seq,
            state: None,
        });
        line.last_seq = seq;
        match record.kind() {
            EventKind::RunStarted => {
                let started = record.data().read::<RunStarted>()?;
                line.addr = record.addr().cloned();
                line.session = line
                    .addr
                    .as_ref()
                    .and_then(|addr| self.stretches.get(addr).copied());
                line.parent = line.parent.or(started.parent);
                line.predecessor = started.predecessor;
            }
            EventKind::RunForked => {
                let forked = record.data().read::<RunForked>()?;
                line.parent = Some(forked.from);
                line.forked_at = Some(forked.at_seq);
            }
            _ => {}
        }
        Ok(())
    }

    /// Every run, oldest first, with its state read off the hot view.
    pub fn lines(&self) -> impl Iterator<Item = RunLine> + '_ {
        let mut lines: Vec<&RunLine> = self.runs.values().collect();
        lines.sort_by_key(|line| line.first_seq);
        lines.into_iter().map(|line| RunLine {
            state: self.hot.get(&line.run).map(|hot| hot.phase),
            ..line.clone()
        })
    }
}

/// The lineage of the ledger kept in `ledger_dir`, read through the
/// ledger's index.
///
/// # Errors
/// The index cannot be built, a line cannot be read or parsed, or a
/// record the fold needs is malformed.
pub fn lineage_of(ledger_dir: &Path) -> Result<Lineage, AxError> {
    let index = memory::LedgerIndex::rebuild(ledger_dir).map_err(memory::MemoryError::into_ax)?;
    let mut reader = index.reader(ledger_dir);
    let mut lineage = Lineage::default();
    for seq in index.seqs() {
        let line = reader.line_at(seq).map_err(memory::MemoryError::into_ax)?;
        lineage.apply(&EventRecord::parse_line(&line)?)?;
    }
    Ok(lineage)
}
