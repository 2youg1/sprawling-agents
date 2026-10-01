// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A stretch of one city's history, exported as a playback bundle, and a
//! bundle checked against another or recomputed from its city
//! (accounting-SPEC.md 8-12).
//!
//! The properties the export holds are proved in
//! `crates/accounting/spec/Playback/Select.lean` and
//! `crates/accounting/spec/Playback/Project.lean`; this module is how
//! they hold. The command line (`bin::main::playback`) and the
//! resident's tool (`worker::workbench::tools::playback`) are thin doors
//! onto [`export`], [`embed`], [`check`] and [`land`]
//! (accounting-SPEC.md 8-13).

use std::path::Path;

use kernel::layout::CityLayout;
use kernel::{AxCode, AxError, Seq};

use document::{CutoffLine, Decimal, Document, Source};
use project::Projection;
use reader::Readership;

mod check;
mod consistency;
mod document;
mod encode;
mod landing;
mod links;
mod observed;
mod offline;
mod page;
mod project;
mod reader;
mod select;
mod walk;

pub use check::{Asked, City, Report, Verdict, check};
pub use encode::Bundle;
pub use landing::{Place, land};
pub use page::{BUNDLE_BLOCK, embed};
pub use reader::{Confidential, Reader};
pub use select::Selection;

/// The schema a bundle names; a reader refuses any other.
pub const SCHEMA: &str = "sprawling.playback/1";

/// The version of the rules a bundle is projected under. It moves with
/// every change to what a bundle holds or how a table is derived, so a
/// bundle this build cannot recompute says so instead of differing.
pub const PROJECTION_RULES: u32 = 2;

/// The most bytes a bundle may have. An initial value until the export
/// peak on a multi-day fixture is measured; it bounds the bundle, not
/// the memory an export uses.
pub const BUNDLE_MAX_BYTES: usize = 32 * 1024 * 1024;

/// The most bytes a checked file may have: a bundle's ceiling and 16 MiB
/// for the page around it and the fonts and pictures it carries. An
/// initial value, measured on the same fixture as the bundle's.
pub const PAGE_MAX_BYTES: usize = BUNDLE_MAX_BYTES + 16 * 1024 * 1024;

/// Where an export stops reading.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cutoff {
    /// The last complete line when the export starts.
    Latest,
    /// This seq: what a recomputation of an earlier bundle reads to.
    At(Seq),
}

/// One export: which lines, for whom, up to where.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub selection: Selection,
    pub reader: Reader,
    pub cutoff: Cutoff,
}

/// The bundle `request` selects from the city at `city_root`. Reads
/// only; writes nothing.
///
/// # Errors
/// The first ledger line that does not verify; a ledger that ends before
/// a pinned cutoff; a payload the projection must read and cannot; a
/// bundle over [`BUNDLE_MAX_BYTES`]. No partial bundle is ever returned.
pub fn export(city_root: &Path, request: &Request) -> Result<Bundle, AxError> {
    match project(city_root, request)? {
        Projected::Whole(document) => encode::encode(&document),
        Projected::EndsAt(reached) => Err(AxError::failure(
            AxCode::InvalidArgs,
            "export a playback bundle",
            format!(
                "the ledger ends at seq {}",
                reached.map_or_else(|| "none".to_owned(), |seq| seq.value().to_string())
            ),
        )
        .with_recovery("export without a cutoff, or from the city the cutoff was read in")),
    }
}

/// What one projection reached.
enum Projected {
    Whole(Box<Document>),
    /// The ledger ended before the pinned cutoff, at this seq.
    EndsAt(Option<Seq>),
}

fn project(city_root: &Path, request: &Request) -> Result<Projected, AxError> {
    let ledger = CityLayout::new(city_root).ledger();
    let city = storage::Provenance::city_of(&ledger).map_err(storage::StorageError::into_ax)?;
    let readership = Readership::new(city_root, request.reader.clone());
    let mut projection = Projection::new(&request.selection, readership);
    let tip = walk::walk(&ledger, request.cutoff, |raw, walked| {
        projection.apply(raw, walked)
    })?;
    let reached = match (tip, request.cutoff) {
        (Some(tip), Cutoff::Latest) => tip,
        (Some(tip), Cutoff::At(end)) if tip.seq == end => tip,
        (tip, Cutoff::Latest | Cutoff::At(_)) => {
            return Ok(Projected::EndsAt(tip.map(|tip| tip.seq)));
        }
    };
    let source = Source {
        city,
        selection: request.selection.chosen(),
        cutoff: CutoffLine {
            seq: Decimal(reached.seq.value()),
            chain_hash: reached.chain_hash,
        },
        rules: PROJECTION_RULES,
        reader: request.reader.name(),
    };
    projection
        .finish(source)
        .map(|document| Projected::Whole(Box::new(document)))
}

#[cfg(test)]
pub(crate) mod tests;
