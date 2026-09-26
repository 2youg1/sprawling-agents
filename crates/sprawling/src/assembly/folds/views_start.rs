// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Where the views start folding: after the snapshot, or from genesis
//! and why (sprawling-SPEC 8-91).

use std::path::Path;

use kernel::layout::CityLayout;
use kernel::{AxError, RunId, Seq};
use memory::{ChainSnapshot, CheckedLine, MemoryError, SnapshotStart, WholeFold};
use runtime::diagnostics::{Diagnostics, Level, Site};

use crate::views::{Views, views_fold_version};

use super::known_records;

/// The views a start folded, how it began, and where the next snapshot
/// is cut.
pub(crate) struct StartedViews {
    pub(crate) views: Views,
    pub(crate) from: ViewsStart,
    /// The last line this start folded, with its seq. `None` when it
    /// folded nothing past the snapshot, so there is nothing to cut.
    last: Option<(Seq, Vec<u8>)>,
}

/// How a start began.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ViewsStart {
    /// From the snapshot, folding the `tail` lines after it.
    Resumed { tail: usize },
    /// From genesis, and why the snapshot was not used.
    Whole(WholeFold),
}

/// The views of the ledger in `ledger_dir`, from its snapshot when one
/// fits and from genesis otherwise; a snapshot that fails verification
/// or does not decode is never trusted.
///
/// # Errors
/// A tail or a whole history that does not verify, a record a view
/// cannot read, and an I/O failure reading the ledger or the snapshot.
pub(crate) fn start_views(ledger_dir: &Path) -> Result<StartedViews, AxError> {
    let city_root = city_root_of(ledger_dir);
    let snapshots = CityLayout::new(city_root).snapshot();
    match memory::start_from_snapshot(ledger_dir, &snapshots, views_fold_version())
        .map_err(MemoryError::into_ax)?
    {
        SnapshotStart::Resume { snapshot, tail } => {
            match Views::decode(city_root, snapshot.views()) {
                Ok(views) => resume(views, &snapshot, tail),
                Err(undecodable) => whole(
                    city_root,
                    memory::read_raw_lines_at(ledger_dir).map_err(MemoryError::into_ax)?,
                    WholeFold::Damaged(undecodable.to_string()),
                ),
            }
        }
        SnapshotStart::Whole { lines, because } => whole(city_root, lines, because),
    }
}

/// The views `serve` starts with: [`start_views`], then a snapshot cut
/// at the last line it folded, then where the start began written to
/// `log` (sprawling-SPEC 8-91).
///
/// # Errors
/// Those of [`start_views`].
pub(crate) fn start_served_views(
    ledger_dir: &Path,
    log: &mut Diagnostics,
) -> Result<Views, AxError> {
    let started = start_views(ledger_dir)?;
    cut_views_snapshot(ledger_dir, &started)?;
    started.report_start(log);
    Ok(started.views)
}

/// Cut a snapshot of `started` at the last line it folded; nothing when
/// it folded nothing past the snapshot it resumed from.
///
/// # Errors
/// An encoding failure and an I/O failure writing the snapshot.
fn cut_views_snapshot(ledger_dir: &Path, started: &StartedViews) -> Result<(), AxError> {
    let Some((seq, line)) = &started.last else {
        return Ok(());
    };
    let snapshot = ChainSnapshot::cut(views_fold_version(), *seq, line, started.views.encode()?);
    memory::write_snapshot(
        &CityLayout::new(city_root_of(ledger_dir)).snapshot(),
        &snapshot,
    )
    .map_err(MemoryError::into_ax)
}

impl StartedViews {
    /// Say where this start began and, when not from the snapshot, why.
    fn report_start(&self, log: &mut Diagnostics) {
        let site = Site {
            run: RunId::CITY,
            seq: self.last.as_ref().map_or(Seq::FIRST, |(seq, _)| *seq),
            module: "bin::assembly",
        };
        log.write(Level::Effect, site, &self.from.to_string());
    }
}

impl std::fmt::Display for ViewsStart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let why = match self {
            ViewsStart::Resumed { tail } => {
                return write!(
                    f,
                    "the views resumed from their snapshot and folded {tail} newer lines"
                );
            }
            ViewsStart::Whole(WholeFold::NoSnapshot) => "there is no snapshot yet".to_owned(),
            ViewsStart::Whole(WholeFold::Damaged(reason)) => {
                format!("the snapshot was refused: {reason}")
            }
            ViewsStart::Whole(WholeFold::OtherFoldVersion { found }) => {
                format!("the snapshot was cut under fold version {found}, not this build's")
            }
            ViewsStart::Whole(WholeFold::Stale) => {
                "the snapshot was cut from another ledger".to_owned()
            }
            ViewsStart::Whole(WholeFold::Missing) => {
                "the ledger ends before the snapshot's line".to_owned()
            }
        };
        write!(f, "the views folded the whole ledger from genesis: {why}")
    }
}

/// The snapshot's views with the tail folded on, each tail line through
/// the same per-line check a whole history passes.
fn resume(
    mut views: Views,
    snapshot: &ChainSnapshot,
    tail: Vec<Vec<u8>>,
) -> Result<StartedViews, AxError> {
    let mut check = snapshot.resume()?;
    let mut last_seq = None;
    for raw in &tail {
        let seq = check.expected();
        let checked = check
            .advance(raw)
            .map_err(|fault| fault.into_ax(seq.value().saturating_add(1)))?;
        match checked {
            CheckedLine::Known(record) => views.apply(&record)?,
            CheckedLine::IgnoredUnknown(_) => {}
        }
        last_seq = Some(seq);
    }
    let from = ViewsStart::Resumed { tail: tail.len() };
    Ok(StartedViews {
        views,
        from,
        last: last_seq.zip(tail.into_iter().last()),
    })
}

/// The views folded from genesis over `lines`, verified first.
fn whole(
    city_root: &Path,
    lines: Vec<Vec<u8>>,
    because: WholeFold,
) -> Result<StartedViews, AxError> {
    let verified = runtime::replay::verify_lines(lines)?;
    let mut views = Views::new(city_root);
    for record in known_records(&verified) {
        views.apply(record)?;
    }
    let last = verified
        .tail_seq()
        .zip(verified.raw_lines().last().cloned());
    Ok(StartedViews {
        views,
        from: ViewsStart::Whole(because),
        last,
    })
}

fn city_root_of(ledger_dir: &Path) -> &Path {
    ledger_dir
        .parent()
        .and_then(Path::parent)
        .unwrap_or(ledger_dir)
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
