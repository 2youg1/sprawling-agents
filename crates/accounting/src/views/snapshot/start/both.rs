// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Two folds started together on one pass over the ledger, from the
//! earlier of their two snapshots (sprawling-SPEC 8-122). That the pass
//! gives each fold what its own whole fold gives is `twoCutsOnePass` in
//! `crates/storage/spec/Snapshot.lean`.

use std::path::Path;

use kernel::{AxError, Seq};
use runtime::replay::fold_ledger_dir;
use storage::{ChainSnapshot, CheckedLine, LedgerIndex, SnapshotStart, StorageError, WholeFold};

use super::{
    Candidate, FoldStart, SnapshotFold, Started, candidate, city_root_of, last_line, whole,
};

/// Two folds started on one pass, how many lines that pass checked, and
/// where it began.
pub struct BothStarted<A, B> {
    pub first: Started<A>,
    pub second: Started<B>,
    /// Lines checked one by one through `LineCheck` on this pass.
    pub checked: u64,
    pub from: TailFrom,
}

/// Where the one pass began.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TailFrom {
    /// At the earlier of the two snapshots' lines.
    Snapshots,
    /// At genesis, because one of the folds could not resume.
    Genesis,
}

/// Starts `A` and `B` over the ledger in `ledger_dir` on one pass: from
/// the earlier of their snapshots when both fit, each line shown only to
/// the fold whose snapshot precedes it; from genesis for both when either
/// cannot resume. A later snapshot whose own line turns out not to sit at
/// its seq is folded from genesis afterwards, the one case that reads
/// twice.
///
/// When the pass is from genesis, `first` is handed the index it built;
/// `second` an empty one.
///
/// # Errors
/// A line that does not verify, a record a fold cannot read, and an I/O
/// failure reading the ledger or a snapshot.
pub fn start_both<A: SnapshotFold, B: SnapshotFold>(
    ledger_dir: &Path,
) -> Result<BothStarted<A, B>, AxError> {
    let city_root = city_root_of(ledger_dir);
    match (candidate::<A>(city_root)?, candidate::<B>(city_root)?) {
        (
            Candidate::Fits {
                folded: a,
                snapshot: at_a,
            },
            Candidate::Fits {
                folded: b,
                snapshot: at_b,
            },
        ) => {
            if at_a.seq() <= at_b.seq() {
                onward(ledger_dir, (a, at_a), (b, at_b))
            } else {
                onward(ledger_dir, (b, at_b), (a, at_a)).map(|swapped| BothStarted {
                    first: swapped.second,
                    second: swapped.first,
                    checked: swapped.checked,
                    from: swapped.from,
                })
            }
        }
        (a, b) => from_genesis(ledger_dir, reason_of(a), reason_of(b)),
    }
}

fn reason_of<F>(candidate: Candidate<F>) -> Option<WholeFold> {
    match candidate {
        Candidate::Fits { .. } => None,
        Candidate::Whole(because) => Some(because),
    }
}

/// One pass from `earlier`'s snapshot, folding every tail line into it
/// and the lines past `later`'s seq into `later`.
fn onward<E: SnapshotFold, L: SnapshotFold>(
    ledger_dir: &Path,
    (mut early, at_early): (E, ChainSnapshot),
    (mut late, at_late): (L, ChainSnapshot),
) -> Result<BothStarted<E, L>, AxError> {
    let tail =
        match storage::tail_after(ledger_dir, at_early.clone()).map_err(StorageError::into_ax)? {
            SnapshotStart::Resume { tail, .. } => tail,
            SnapshotStart::Whole(because) => return from_genesis(ledger_dir, Some(because), None),
        };
    let mut check = at_early.resume()?;
    let mut late_fits = at_late.same_line(&at_early).then_some(true);
    let mut late_tail: usize = 0;
    for raw in &tail {
        let seq = check.expected();
        let checked = check
            .advance(raw)
            .map_err(|fault| fault.into_ax(seq.value().saturating_add(1)))?;
        if seq == at_late.seq() {
            late_fits = Some(matches!(at_late.fit(raw), storage::SnapshotFit::Fits));
        }
        let to_late = seq > at_late.seq() && late_fits == Some(true);
        if let CheckedLine::Known(record) = &checked {
            early.absorb(record)?;
            if to_late {
                late.absorb(record)?;
            }
        }
        if to_late {
            late_tail = late_tail.saturating_add(1);
        }
    }
    let last = tail.last().map(|line| {
        let count = u64::try_from(tail.len()).unwrap_or(u64::MAX);
        (
            Seq::new(at_early.seq().value().saturating_add(count)),
            line.clone(),
        )
    });
    early.resumed(ledger_dir)?;
    let second = match late_fits {
        Some(true) => {
            late.resumed(ledger_dir)?;
            Started {
                folded: late,
                from: FoldStart::Resumed { tail: late_tail },
                last: last.clone().filter(|_| late_tail > 0),
            }
        }
        Some(false) => whole(ledger_dir, WholeFold::Stale)?,
        None => whole(ledger_dir, WholeFold::Missing)?,
    };
    Ok(BothStarted {
        first: Started {
            folded: early,
            from: FoldStart::Resumed { tail: tail.len() },
            last,
        },
        second,
        checked: u64::try_from(tail.len()).unwrap_or(u64::MAX),
        from: TailFrom::Snapshots,
    })
}

/// Both folds from genesis on one pass, each started with the reason its
/// own snapshot was not used, or as folded alongside the other.
fn from_genesis<A: SnapshotFold, B: SnapshotFold>(
    ledger_dir: &Path,
    first: Option<WholeFold>,
    second: Option<WholeFold>,
) -> Result<BothStarted<A, B>, AxError> {
    let city_root = city_root_of(ledger_dir);
    let (mut a, mut b) = (A::empty(city_root), B::empty(city_root));
    let index = fold_ledger_dir(ledger_dir, |record| {
        a.absorb(record)?;
        b.absorb(record)
    })?;
    let last = last_line(&index, ledger_dir)?;
    let checked = u64::try_from(index.len()).unwrap_or(u64::MAX);
    a.keep_index(index, ledger_dir)?;
    b.keep_index(LedgerIndex::empty(), ledger_dir)?;
    let from = |because: Option<WholeFold>| because.map_or(FoldStart::Alongside, FoldStart::Whole);
    Ok(BothStarted {
        first: Started {
            folded: a,
            from: from(first),
            last: last.clone(),
        },
        second: Started {
            folded: b,
            from: from(second),
            last,
        },
        checked,
        from: TailFrom::Genesis,
    })
}
