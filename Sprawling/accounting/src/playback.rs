// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use kernel::layout::CityLayout;
use kernel::{AxCode, AxError, Seq};

use document::{CutoffLine, Decimal, Document, Source};
use project::{Projection, Scanning};
use reader::Readership;

mod check;
mod consistency;
mod diff;
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
mod traced;
mod walk;

pub use check::{Asked, City, Report, Verdict, check};
pub use encode::Bundle;
pub use landing::{Place, land};
pub use page::{BUNDLE_BLOCK, embed};
pub use reader::{Confidential, Reader};
pub use select::{Selection, Window};

pub const SCHEMA: &str = "sprawling.playback/3";

pub const PROJECTION_RULES: u32 = 4;

pub const BUNDLE_MAX_BYTES: usize = 32 * 1024 * 1024;

pub const DIFF_MAX_BYTES: usize = 64 * 1024;

pub const PAGE_MAX_BYTES: usize = BUNDLE_MAX_BYTES + 16 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cutoff {

    Latest,

    At(Seq),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub selection: Selection,
    pub reader: Reader,
    pub cutoff: Cutoff,
}

pub fn export(city_root: &Path, request: &Request) -> Result<Bundle, AxError> {
    match project(city_root, request, Scanning::Lazy)? {
        Projected::Whole { document, .. } => encode::encode(&document),
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

enum Projected {

    Whole {
        document: Box<Document>,
        #[cfg_attr(
            not(test),
            expect(
                dead_code,
                reason = "the lazy-scan tests read the count; an export does not report it"
            )
        )]
        scans: u64,
    },

    EndsAt(Option<Seq>),
}

fn project(city_root: &Path, request: &Request, scanning: Scanning) -> Result<Projected, AxError> {
    let ledger = CityLayout::new(city_root).ledger();
    let city = storage::Provenance::city_of(&ledger).map_err(storage::StorageError::into_ax)?;
    let readership = Readership::new(city_root, request.reader.clone());
    let mut projection = Projection::new(&request.selection, readership, city_root, scanning);
    let walked = walk::walk(&ledger, request.cutoff, |raw, walked| {
        projection.apply(raw, walked)
    })?;
    let reached = match (walked.tip, request.cutoff) {
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
        .finish(source, &walked.index)
        .map(|(document, scans)| Projected::Whole {
            document: Box::new(document),
            scans,
        })
}

#[cfg(test)]
pub(crate) mod tests;
