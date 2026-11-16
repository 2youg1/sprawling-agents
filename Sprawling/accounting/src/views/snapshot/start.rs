// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::{Path, PathBuf};

use kernel::layout::CityLayout;
use kernel::{AxError, EventRecord, Seq};
use runtime::replay::fold_ledger_dir;
use storage::{
    ChainSnapshot, CheckedLine, LedgerIndex, SnapshotStart, StorageError, StoredSnapshot, WholeFold,
};

mod both;

pub use both::{BothStarted, TailFrom, start_both};

pub fn city_root_of(ledger_dir: &Path) -> &Path {
    ledger_dir.ancestors().nth(2).unwrap_or(ledger_dir)
}

pub trait SnapshotFold: Sized {

    const DIR: &'static str;

    fn fold_version() -> u32;

    fn empty(city_root: &Path) -> Self;

    fn decode(city_root: &Path, bytes: &[u8]) -> Result<Self, AxError>;
    fn encode(&self) -> Result<Vec<u8>, AxError>;

    fn absorb(&mut self, record: &EventRecord) -> Result<(), AxError>;

    fn keep_index(&mut self, index: LedgerIndex, ledger_dir: &Path) -> Result<(), AxError>;

    fn resumed(&mut self, ledger_dir: &Path) -> Result<(), AxError>;
}

pub fn snapshot_dir<F: SnapshotFold>(city_root: &Path) -> PathBuf {
    CityLayout::new(city_root).snapshot().join(F::DIR)
}

pub fn proof_dir(city_root: &Path) -> PathBuf {
    CityLayout::new(city_root).snapshot().join("verified")
}

pub struct Started<F> {
    pub folded: F,
    pub from: FoldStart,

    last: Option<(Seq, Vec<u8>)>,
}

impl<F> Started<F> {

    pub fn last_seq(&self) -> Option<Seq> {
        self.last.as_ref().map(|(seq, _)| *seq)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FoldStart {

    Resumed { tail: usize },

    Whole(WholeFold),

    Alongside,
}

pub fn start<F: SnapshotFold>(ledger_dir: &Path) -> Result<Started<F>, AxError> {
    begin(ledger_dir, Resuming::AsItStands).map(|audited| audited.started)
}

#[derive(Clone, Copy)]
enum Resuming {

    AsItStands,

    AfterProof,
}

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

pub struct Audited<F> {
    pub started: Started<F>,
    pub lines_checked: u64,
}

pub fn start_audited<F: SnapshotFold>(ledger_dir: &Path) -> Result<Audited<F>, AxError> {
    begin(ledger_dir, Resuming::AfterProof)
}

fn prove(ledger_dir: &Path) -> Result<u64, AxError> {
    let records = storage::ProofRecords::read_only(&proof_dir(city_root_of(ledger_dir)));
    let proven = storage::prove_chain(ledger_dir, &records).map_err(StorageError::into_ax)?;
    match proven.audit {
        storage::ChainAudit::Whole { .. } => Ok(proven.counted.lines_checked),
        storage::ChainAudit::Broken(reason) => Err(reason),
    }
}

fn folded_lines<F>(started: &Started<F>) -> u64 {
    match started.from {
        FoldStart::Resumed { tail } => u64::try_from(tail).unwrap_or(u64::MAX),
        FoldStart::Whole(_) | FoldStart::Alongside => started
            .last_seq()
            .map_or(0, |seq| seq.value().saturating_add(1)),
    }
}

pub fn cut<F: SnapshotFold>(ledger_dir: &Path, started: &Started<F>) -> Result<(), AxError> {
    cut_at(ledger_dir, &started.folded, started.last.as_ref())
}

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

enum Candidate<F> {
    Fits { folded: F, snapshot: ChainSnapshot },
    Whole(WholeFold),
}

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
