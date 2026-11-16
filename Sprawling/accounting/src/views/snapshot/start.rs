// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Where a fold a snapshot holds starts: after the snapshot, or from
//! genesis and why, and the snapshot a start cuts (`crates/sprawling/Spec.lean` §8-91);
//! and where two such folds start together, on one pass (8-122).
//!
//! Held beside the views rather than in the assembly point, which also
//! starts the worker's standing from it: a one-shot read starts the views
//! here through `Views::rebuild`, and the read side never names the
//! assembly point (`crates/sprawling/Spec.lean` §8-92).

use std::path::{Path, PathBuf};

use kernel::layout::CityLayout;
use kernel::{AxError, EventRecord, Seq};
use runtime::replay::fold_ledger_dir;
use storage::{
    ChainSnapshot, CheckedLine, LedgerIndex, SnapshotStart, StorageError, StoredSnapshot, WholeFold,
};

mod both;

pub use both::{BothStarted, TailFrom, start_both};

/// The city a ledger directory belongs to, two levels up.
pub fn city_root_of(ledger_dir: &Path) -> &Path {
    ledger_dir.ancestors().nth(2).unwrap_or(ledger_dir)
}

/// A fold a snapshot can hold: the views a page reads, and the standing
/// a worker judges from. Each keeps its own snapshot directory and fold
/// version, because each changes its encoding on its own.
pub trait SnapshotFold: Sized {
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
    /// A fold resumed from its snapshot and caught up with the tail: one
    /// that reads the ledger again brings the index its snapshot carried
    /// up to the ledger as it stands.
    fn resumed(&mut self, ledger_dir: &Path) -> Result<(), AxError>;
}

/// Where the snapshot of `F` for the city at `city_root` lives.
pub fn snapshot_dir<F: SnapshotFold>(city_root: &Path) -> PathBuf {
    CityLayout::new(city_root).snapshot().join(F::DIR)
}

/// Where the city at `city_root` keeps its verified prefix records
/// (`crates/storage/Spec.lean` §8-30): the one spelling of that path.
pub fn proof_dir(city_root: &Path) -> PathBuf {
    CityLayout::new(city_root).snapshot().join("verified")
}

/// A fold a start built, how it began, and where the next snapshot is
/// cut.
pub struct Started<F> {
    pub folded: F,
    pub from: FoldStart,
    /// The last line this start folded, with its seq. `None` when it
    /// folded nothing past the snapshot, so there is nothing to cut.
    last: Option<(Seq, Vec<u8>)>,
}

impl<F> Started<F> {
    /// The seq of the last line this start folded, when it folded one.
    pub fn last_seq(&self) -> Option<Seq> {
        self.last.as_ref().map(|(seq, _)| *seq)
    }
}

/// How a start began.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FoldStart {
    /// From the snapshot, folding the `tail` lines after it.
    Resumed { tail: usize },
    /// From genesis, and why the snapshot was not used.
    Whole(WholeFold),
    /// From genesis on the pass another fold needed from genesis; this
    /// fold's own snapshot was not used (`crates/sprawling/Spec.lean` §8-122).
    Alongside,
}

/// The fold of the ledger in `ledger_dir`, from its snapshot when one
/// fits and from genesis otherwise; a snapshot that fails verification
/// or does not decode is never trusted.
///
/// # Errors
/// A tail or a whole history that does not verify, a record the fold
/// cannot read, and an I/O failure reading the ledger or the snapshot.
pub fn start<F: SnapshotFold>(ledger_dir: &Path) -> Result<Started<F>, AxError> {
    begin(ledger_dir, Resuming::AsItStands).map(|audited| audited.started)
}

/// What a start does before it resumes from a snapshot.
#[derive(Clone, Copy)]
enum Resuming {
    /// Nothing: a proof runs behind the start, or none is wanted.
    AsItStands,
    /// Prove the whole chain first, because the lines before the snapshot
    /// are not read by the start and nothing else will prove them.
    AfterProof,
}

/// The one start behind [`start`] and [`start_audited`]: from the
/// snapshot when one fits, proving the chain first when `resuming` says
/// so; from genesis otherwise, where the whole fold checks every line and
/// a proof would check each a second time (`crates/accounting/spec/Views/Snapshot.lean` §8-19).
fn begin<F: SnapshotFold>(ledger_dir: &Path, resuming: Resuming) -> Result<Audited<F>, AxError> {
    let city_root = city_root_of(ledger_dir);
    let (started, proved) = match storage::start_from_snapshot(
        ledger_dir,
        &snapshot_dir::<F>(city_root),
        F::fold_version(),
    )
    .map_err(StorageError::into_ax)?
    {
        SnapshotStart::Resume { snapshot, tail } => {
            let proved = match resuming {
                Resuming::AsItStands => 0,
                Resuming::AfterProof => prove(ledger_dir)?,
            };
            let started = match F::decode(city_root, snapshot.views()) {
                Ok(folded) => resume(folded, &snapshot, tail, ledger_dir)?,
                Err(undecodable) => whole(ledger_dir, WholeFold::Damaged(undecodable.to_string()))?,
            };
            (started, proved)
        }
        SnapshotStart::Whole(because) => (whole(ledger_dir, because)?, 0),
    };
    let folded = folded_lines(&started);
    Ok(Audited {
        started,
        lines_checked: proved.saturating_add(folded),
    })
}

/// A start from a proved history, and how many lines were checked one by
/// one on the way there: the proof's and the fold's together
/// (`crates/accounting/spec/Views/Snapshot.lean` §8-19).
pub struct Audited<F> {
    pub started: Started<F>,
    pub lines_checked: u64,
}

/// [`start`] from a proved history, so a start from the snapshot never
/// accepts a chain a whole fold would refuse: the snapshot's fit checks
/// only the line at its seq, and the ledger open scans only the last
/// segment (`crates/sprawling/Spec.lean` §8-101). Before resuming, the whole chain is
/// proved with the city's verified prefix records, read and never
/// written: a one-shot read does not write to the disk. From genesis the
/// whole fold checks every line through the same `LineCheck`, which is
/// the proof's verdict, so no separate proof runs (`crates/accounting/spec/Views/Snapshot.lean`
/// §8-19).
///
/// # Errors
/// The proof's reason when the chain is broken or cannot be read, and
/// those of [`start`].
pub fn start_audited<F: SnapshotFold>(ledger_dir: &Path) -> Result<Audited<F>, AxError> {
    begin(ledger_dir, Resuming::AfterProof)
}

/// Proves the chain in `ledger_dir` with the city's records read-only,
/// answering how many lines the proof checked one by one.
///
/// # Errors
/// The proof's reason when the chain is broken, and an I/O failure
/// reading it.
fn prove(ledger_dir: &Path) -> Result<u64, AxError> {
    let records = storage::ProofRecords::read_only(&proof_dir(city_root_of(ledger_dir)));
    let proven = storage::prove_chain(ledger_dir, &records).map_err(StorageError::into_ax)?;
    match proven.audit {
        storage::ChainAudit::Whole { .. } => Ok(proven.counted.lines_checked),
        storage::ChainAudit::Broken(reason) => Err(reason),
    }
}

/// The lines a start checked one by one while folding: the tail after its
/// snapshot, or every line from genesis to the last one it folded.
fn folded_lines<F>(started: &Started<F>) -> u64 {
    match started.from {
        FoldStart::Resumed { tail } => u64::try_from(tail).unwrap_or(u64::MAX),
        FoldStart::Whole(_) | FoldStart::Alongside => started
            .last_seq()
            .map_or(0, |seq| seq.value().saturating_add(1)),
    }
}

/// Cut a snapshot of `started` at the last line it folded; nothing when
/// it folded nothing past the snapshot it resumed from.
///
/// # Errors
/// An encoding failure and an I/O failure writing the snapshot.
pub fn cut<F: SnapshotFold>(ledger_dir: &Path, started: &Started<F>) -> Result<(), AxError> {
    cut_at(ledger_dir, &started.folded, started.last.as_ref())
}

/// Cut a snapshot of `folded` at `last`, the last line it folded;
/// nothing when it has folded nothing.
///
/// # Errors
/// An encoding failure and an I/O failure writing the snapshot.
pub fn cut_at<F: SnapshotFold>(
    ledger_dir: &Path,
    folded: &F,
    last: Option<&(Seq, Vec<u8>)>,
) -> Result<(), AxError> {
    let Some((seq, line)) = last else {
        return Ok(());
    };
    let snapshot = ChainSnapshot::cut(F::fold_version(), *seq, line, folded.encode()?);
    storage::write_snapshot(&snapshot_dir::<F>(city_root_of(ledger_dir)), &snapshot)
        .map_err(StorageError::into_ax)
}

/// The last line `index` holds, with its seq, where a snapshot of what
/// was folded while it was built is cut. `None` for a ledger with no
/// line yet.
///
/// # Errors
/// A segment the index names that cannot be read.
pub fn last_line(
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
        .map_err(StorageError::into_ax)
}

impl std::fmt::Display for FoldStart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let why = match self {
            FoldStart::Resumed { tail } => {
                return write!(f, "resumed from the snapshot and folded {tail} newer lines");
            }
            FoldStart::Alongside => {
                return f.write_str(
                    "folded from genesis on the pass the other fold needed from genesis",
                );
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

/// A snapshot of `F` that was read, cut under this build's fold version,
/// and decoded; or why there is none to resume from.
enum Candidate<F> {
    Fits { folded: F, snapshot: ChainSnapshot },
    Whole(WholeFold),
}

/// Reads the snapshot of `F` for the city at `city_root` and decodes it,
/// reading nothing of the ledger.
///
/// # Errors
/// An I/O failure reading the snapshot.
fn candidate<F: SnapshotFold>(city_root: &Path) -> Result<Candidate<F>, AxError> {
    Ok(
        match storage::read_snapshot(&snapshot_dir::<F>(city_root))
            .map_err(StorageError::into_ax)?
        {
            StoredSnapshot::Absent => Candidate::Whole(WholeFold::NoSnapshot),
            StoredSnapshot::Damaged(reason) => Candidate::Whole(WholeFold::Damaged(reason)),
            StoredSnapshot::Present(snapshot) if snapshot.fold_version() != F::fold_version() => {
                Candidate::Whole(WholeFold::OtherFoldVersion {
                    found: snapshot.fold_version(),
                })
            }
            StoredSnapshot::Present(snapshot) => match F::decode(city_root, snapshot.views()) {
                Ok(folded) => Candidate::Fits { folded, snapshot },
                Err(undecodable) => Candidate::Whole(WholeFold::Damaged(undecodable.to_string())),
            },
        },
    )
}

/// The snapshot's fold with the tail folded on, each tail line through
/// the same per-line check a whole history passes.
fn resume<F: SnapshotFold>(
    mut folded: F,
    snapshot: &ChainSnapshot,
    tail: Vec<Vec<u8>>,
    ledger_dir: &Path,
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
    folded.resumed(ledger_dir)?;
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
