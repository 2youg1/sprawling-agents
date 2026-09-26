// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Where the views start folding: after the snapshot, or from genesis
//! and why; and the snapshot a served city cuts (sprawling-SPEC 8-91).

use std::path::Path;

use kernel::layout::CityLayout;
use kernel::{AxError, RunId, Seq};
use memory::{
    ChainSnapshot, CheckedLine, JsonlLedger, MemoryError, OpenReport, SnapshotStart, WholeFold,
};
use runtime::diagnostics::{Diagnostics, Level, Site};
use runtime::replay::fold_ledger_dir;

use crate::views::{Views, views_fold_version};

use super::{Standing, city_root_of, epoch_of, fold_city};

/// How a start began.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ViewsStart {
    /// From the snapshot, folding the `tail` lines after it.
    Resumed { tail: usize },
    /// From genesis, and why the snapshot was not used.
    Whole(WholeFold),
}

/// The views of the ledger in `ledger_dir` and how the start began:
/// from its snapshot when one fits and from genesis otherwise; a
/// snapshot that fails verification or does not decode is never
/// trusted.
///
/// # Errors
/// A tail or a whole history that does not verify, a record a view
/// cannot read, and an I/O failure reading the ledger or the snapshot.
pub(crate) fn start_views(ledger_dir: &Path) -> Result<(Views, ViewsStart), AxError> {
    let city_root = city_root_of(ledger_dir);
    let snapshots = CityLayout::new(city_root).snapshot();
    match memory::start_from_snapshot(ledger_dir, &snapshots, views_fold_version())
        .map_err(MemoryError::into_ax)?
    {
        SnapshotStart::Resume { snapshot, tail } => {
            match Views::decode(city_root, snapshot.views()) {
                Ok(views) => resume(views, &snapshot, tail),
                Err(undecodable) => whole(ledger_dir, WholeFold::Damaged(undecodable.to_string())),
            }
        }
        // The lines are folded again as a stream rather than held: a
        // whole history in memory at once is what the streaming fold
        // exists to avoid.
        SnapshotStart::Whole { lines, because } => {
            drop(lines);
            whole(ledger_dir, because)
        }
    }
}

/// What `serve` starts from: [`fold_city`], then a views snapshot cut at
/// the last line it folded, so a one-shot read afterwards folds only what
/// arrives after it (sprawling-SPEC 8-91).
///
/// `serve` itself folds the whole history rather than resuming: the
/// worker's standing is folded on the same pass and needs every record,
/// so starting the views from a snapshot would add a second read rather
/// than remove one.
///
/// A cut that fails is a `Refuse` line in `log`, not an error: the
/// snapshot only shortens a later read, which folds from the older
/// snapshot or from genesis to the same views.
///
/// # Errors
/// Those of [`fold_city`].
pub(crate) fn start_served_views(
    ledger_dir: &Path,
    log: &mut Diagnostics,
) -> Result<(Views, (JsonlLedger, OpenReport, Standing)), AxError> {
    let (views, held) = fold_city(ledger_dir)?;
    let last = views.last_folded_line(ledger_dir);
    let site = Site {
        run: RunId::CITY,
        seq: last
            .as_ref()
            .ok()
            .and_then(|last| last.as_ref().map(|(seq, _)| *seq))
            .unwrap_or(Seq::FIRST),
        module: "bin::assembly",
    };
    let cut = last.and_then(|last| cut_views_snapshot(ledger_dir, &views, last.as_ref()));
    if let Err(fault) = cut {
        log.write(
            Level::Refuse,
            site,
            &format!(
                "the views snapshot was not cut: {fault}; serving goes on, and the next read folds from the older snapshot or from genesis"
            ),
        );
    }
    Ok((views, held))
}

/// Cut a snapshot of `views` at `last`, the last line they folded;
/// nothing when they have folded nothing.
///
/// # Errors
/// An encoding failure and an I/O failure writing the snapshot.
fn cut_views_snapshot(
    ledger_dir: &Path,
    views: &Views,
    last: Option<&(Seq, Vec<u8>)>,
) -> Result<(), AxError> {
    let Some((seq, line)) = last else {
        return Ok(());
    };
    let snapshot = ChainSnapshot::cut(views_fold_version(), *seq, line, views.encode()?);
    memory::write_snapshot(
        &CityLayout::new(city_root_of(ledger_dir)).snapshot(),
        &snapshot,
    )
    .map_err(MemoryError::into_ax)
}

/// The snapshot's views with the tail folded on, each tail line through
/// the same per-line check a whole history passes. The index starts
/// empty and is refreshed by the first question that reads the ledger.
fn resume(
    mut views: Views,
    snapshot: &ChainSnapshot,
    tail: Vec<Vec<u8>>,
) -> Result<(Views, ViewsStart), AxError> {
    let mut check = snapshot.resume()?;
    for raw in &tail {
        let seq = check.expected();
        let checked = check
            .advance(raw)
            .map_err(|fault| fault.into_ax(seq.value().saturating_add(1)))?;
        match checked {
            CheckedLine::Known(record) => views.apply(&record)?,
            CheckedLine::IgnoredUnknown(_) => {}
        }
    }
    Ok((views, ViewsStart::Resumed { tail: tail.len() }))
}

/// The views folded from genesis, streamed and verified line by line.
fn whole(ledger_dir: &Path, because: WholeFold) -> Result<(Views, ViewsStart), AxError> {
    let mut views = Views::new(city_root_of(ledger_dir));
    let index = fold_ledger_dir(ledger_dir, |record| views.apply(record))?;
    views.adopt_epoch(epoch_of(&index, ledger_dir)?);
    views.hold_index(index);
    Ok((views, ViewsStart::Whole(because)))
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
