// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Where a fold a snapshot holds starts: after the snapshot, or from
//! genesis and why, and the snapshot a start cuts (sprawling-SPEC 8-91).
//!
//! Held beside the views rather than in the assembly point, which also
//! starts the worker's standing from it: a one-shot read starts the views
//! here through `Views::rebuild`, and the read side never names the
//! assembly point (sprawling-SPEC.md 8-92).

use std::path::{Path, PathBuf};

use kernel::layout::CityLayout;
use kernel::{AxError, EventRecord, Seq};
use memory::{ChainSnapshot, CheckedLine, LedgerIndex, MemoryError, SnapshotStart, WholeFold};
use runtime::replay::fold_ledger_dir;

/// The city a ledger directory belongs to, two levels up.
pub(crate) fn city_root_of(ledger_dir: &Path) -> &Path {
    ledger_dir.ancestors().nth(2).unwrap_or(ledger_dir)
}

/// A fold a snapshot can hold: the views a page reads, and the standing
/// a worker judges from. Each keeps its own snapshot directory and fold
/// version, because each changes its encoding on its own.
pub(crate) trait SnapshotFold: Sized {
    /// The directory under `<city>/.sprawling/snapshot/` this fold's
    /// snapshot lives in.
    const DIR: &'static str;
    /// The `fold_version` its snapshot is cut and accepted under.
    fn fold_version() -> u32;
    /// The fold of an empty history, served from `city_root`.
    fn empty(city_root: &Path) -> Self;
    /// The fold `encode` wrote, served from `city_root`.
    fn decode(city_root: &Path, bytes: &[u8]) -> Result<Self, AxError>;
    fn encode(&self) -> Result<Vec<u8>, AxError>;
    /// One verified record, folded in.
    fn absorb(&mut self, record: &EventRecord) -> Result<(), AxError>;
    /// The index a fold from genesis built while it streamed the ledger:
    /// a fold that reads the ledger again keeps it, and one that does not
    /// drops it.
    fn keep_index(&mut self, index: LedgerIndex, ledger_dir: &Path) -> Result<(), AxError>;
}

/// Where the snapshot of `F` for the city at `city_root` lives.
pub(crate) fn snapshot_dir<F: SnapshotFold>(city_root: &Path) -> PathBuf {
    CityLayout::new(city_root).snapshot().join(F::DIR)
}

/// A fold a start built, how it began, and where the next snapshot is
/// cut.
pub(crate) struct Started<F> {
    pub(crate) folded: F,
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "serve folds the whole history on the standing's pass, so no served page has a start reason to read (8-91); the other starts discard how they began, and the tests read it"
        )
    )]
    pub(crate) from: FoldStart,
    /// The last line this start folded, with its seq. `None` when it
    /// folded nothing past the snapshot, so there is nothing to cut.
    last: Option<(Seq, Vec<u8>)>,
}

/// How a start began.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FoldStart {
    /// From the snapshot, folding the `tail` lines after it.
    Resumed { tail: usize },
    /// From genesis, and why the snapshot was not used.
    Whole(WholeFold),
}

/// The fold of the ledger in `ledger_dir`, from its snapshot when one
/// fits and from genesis otherwise; a snapshot that fails verification
/// or does not decode is never trusted.
///
/// # Errors
/// A tail or a whole history that does not verify, a record the fold
/// cannot read, and an I/O failure reading the ledger or the snapshot.
pub(crate) fn start<F: SnapshotFold>(ledger_dir: &Path) -> Result<Started<F>, AxError> {
    let city_root = city_root_of(ledger_dir);
    match memory::start_from_snapshot(ledger_dir, &snapshot_dir::<F>(city_root), F::fold_version())
        .map_err(MemoryError::into_ax)?
    {
        SnapshotStart::Resume { snapshot, tail } => match F::decode(city_root, snapshot.views()) {
            Ok(folded) => resume(folded, &snapshot, tail),
            Err(undecodable) => whole(ledger_dir, WholeFold::Damaged(undecodable.to_string())),
        },
        // The lines are folded again as a stream rather than held: a
        // whole history in memory at once is what the streaming fold
        // exists to avoid.
        SnapshotStart::Whole { lines, because } => {
            drop(lines);
            whole(ledger_dir, because)
        }
    }
}

/// [`start`] once an audit of the whole chain returns `Whole`, so a start
/// from the snapshot never accepts a chain a whole fold would refuse: the
/// snapshot's fit checks only the line at its seq, and the ledger open
/// scans only the last segment (sprawling-SPEC 8-92).
///
/// # Errors
/// The audit's reason when the chain is broken or cannot be read, and
/// those of [`start`].
pub(crate) fn start_audited<F: SnapshotFold>(ledger_dir: &Path) -> Result<Started<F>, AxError> {
    match memory::audit_chain(ledger_dir).map_err(MemoryError::into_ax)? {
        memory::ChainAudit::Whole { .. } => start(ledger_dir),
        memory::ChainAudit::Broken(reason) => Err(reason),
    }
}

/// Cut a snapshot of `started` at the last line it folded; nothing when
/// it folded nothing past the snapshot it resumed from.
///
/// # Errors
/// An encoding failure and an I/O failure writing the snapshot.
pub(crate) fn cut<F: SnapshotFold>(ledger_dir: &Path, started: &Started<F>) -> Result<(), AxError> {
    cut_at(ledger_dir, &started.folded, started.last.as_ref())
}

/// Cut a snapshot of `folded` at `last`, the last line it folded;
/// nothing when it has folded nothing.
///
/// # Errors
/// An encoding failure and an I/O failure writing the snapshot.
pub(crate) fn cut_at<F: SnapshotFold>(
    ledger_dir: &Path,
    folded: &F,
    last: Option<&(Seq, Vec<u8>)>,
) -> Result<(), AxError> {
    let Some((seq, line)) = last else {
        return Ok(());
    };
    let snapshot = ChainSnapshot::cut(F::fold_version(), *seq, line, folded.encode()?);
    memory::write_snapshot(&snapshot_dir::<F>(city_root_of(ledger_dir)), &snapshot)
        .map_err(MemoryError::into_ax)
}

/// The last line `index` holds, with its seq, where a snapshot of what
/// was folded while it was built is cut. `None` for a ledger with no
/// line yet.
///
/// # Errors
/// A segment the index names that cannot be read.
pub(crate) fn last_line(
    index: &LedgerIndex,
    ledger_dir: &Path,
) -> Result<Option<(Seq, Vec<u8>)>, AxError> {
    index
        .tail_seq()
        .map(|seq| {
            index
                .reader(ledger_dir)
                .line_at(seq)
                .map(|line| (seq, line))
        })
        .transpose()
        .map_err(MemoryError::into_ax)
}

impl std::fmt::Display for FoldStart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let why = match self {
            FoldStart::Resumed { tail } => {
                return write!(f, "resumed from the snapshot and folded {tail} newer lines");
            }
            FoldStart::Whole(WholeFold::NoSnapshot) => "there is no snapshot yet".to_owned(),
            FoldStart::Whole(WholeFold::Damaged(reason)) => {
                format!("the snapshot was refused: {reason}")
            }
            FoldStart::Whole(WholeFold::OtherFoldVersion { found }) => {
                format!("the snapshot was cut under fold version {found}, not this build's")
            }
            FoldStart::Whole(WholeFold::Stale) => {
                "the snapshot was cut from another ledger".to_owned()
            }
            FoldStart::Whole(WholeFold::Missing) => {
                "the ledger ends before the snapshot's line".to_owned()
            }
        };
        write!(f, "folded the whole ledger from genesis: {why}")
    }
}

/// The snapshot's fold with the tail folded on, each tail line through
/// the same per-line check a whole history passes.
fn resume<F: SnapshotFold>(
    mut folded: F,
    snapshot: &ChainSnapshot,
    tail: Vec<Vec<u8>>,
) -> Result<Started<F>, AxError> {
    let mut check = snapshot.resume()?;
    let mut last_seq = None;
    for raw in &tail {
        let seq = check.expected();
        let checked = check
            .advance(raw)
            .map_err(|fault| fault.into_ax(seq.value().saturating_add(1)))?;
        match checked {
            CheckedLine::Known(record) => folded.absorb(&record)?,
            CheckedLine::IgnoredUnknown(_) => {}
        }
        last_seq = Some(seq);
    }
    let from = FoldStart::Resumed { tail: tail.len() };
    Ok(Started {
        folded,
        from,
        last: last_seq.zip(tail.into_iter().last()),
    })
}

/// An empty fold with the history in `ledger_dir` folded on from
/// genesis, streamed and verified line by line.
fn whole<F: SnapshotFold>(ledger_dir: &Path, because: WholeFold) -> Result<Started<F>, AxError> {
    let mut folded = F::empty(city_root_of(ledger_dir));
    let index = fold_ledger_dir(ledger_dir, |record| folded.absorb(record))?;
    let last = last_line(&index, ledger_dir)?;
    folded.keep_index(index, ledger_dir)?;
    Ok(Started {
        folded,
        from: FoldStart::Whole(because),
        last,
    })
}
