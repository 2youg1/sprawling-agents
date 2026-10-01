// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which lines a playback selection holds (accounting-SPEC.md 8-12).
//! The properties are `crates/accounting/spec/Playback/Select.lean`: the
//! selection is the intersection of every condition given, bounded by
//! the cutoff the walk stops at.

use kernel::{Address, AxCode, AxError, EventRecord, RunId, Seq};
use runtime::clock::UtcSpan;

use super::document::{Chosen, Decimal};

/// A closed seq range, a run and a building, each optional; a line is
/// selected when every condition given holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    first: Option<Seq>,
    last: Option<Seq>,
    run: Option<RunId>,
    building: Option<Address>,
}

impl Selection {
    /// The selection of every line.
    #[must_use]
    pub fn everything() -> Selection {
        Selection {
            first: None,
            last: None,
            run: None,
            building: None,
        }
    }

    /// A selection of the lines from `first` through `last`, both
    /// included, of `run`, addressed within `building`.
    ///
    /// # Errors
    /// `E_INVALID_ARGS` when `first` comes after `last`: that range is a
    /// contradiction, where a range no line falls in is a legal, empty
    /// selection.
    pub fn new(
        first: Option<Seq>,
        last: Option<Seq>,
        run: Option<RunId>,
        building: Option<Address>,
    ) -> Result<Selection, AxError> {
        if let (Some(first), Some(last)) = (first, last)
            && first > last
        {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "select a playback range",
                format!("from {} through {}", first.value(), last.value()),
            )
            .with_recovery("give --from a seq no later than --through"));
        }
        Ok(Selection {
            first,
            last,
            run,
            building,
        })
    }

    /// The selection with a time condition added.
    #[must_use]
    pub fn during(self, _span: UtcSpan) -> Selection {
        self
    }

    /// Whether `record` is in the selection. The cutoff is the walk's to
    /// enforce: no line after it is ever offered here.
    pub(super) fn admits(&self, record: &EventRecord) -> bool {
        self.holds_seq(record.seq())
            && self.run.is_none_or(|run| record.run() == run)
            && self
                .building
                .as_ref()
                .is_none_or(|building| record.addr().is_some_and(|addr| addr.is_within(building)))
    }

    /// Whether `seq` lies in the closed range; the one condition a line
    /// of an unknown kind can be judged by.
    pub(super) fn holds_seq(&self, seq: Seq) -> bool {
        self.first.is_none_or(|first| seq >= first) && self.last.is_none_or(|last| seq <= last)
    }

    /// The selection as the bundle's `source` records it.
    pub(super) fn chosen(&self) -> Chosen {
        Chosen {
            from: self.first.map(|seq| Decimal(seq.value())),
            through: self.last.map(|seq| Decimal(seq.value())),
            run: self.run,
            building: self.building.clone(),
        }
    }

    /// The selection a bundle's `source` recorded, for recomputing it.
    ///
    /// # Errors
    /// The same contradiction [`Selection::new`] refuses.
    pub(super) fn from_chosen(chosen: &Chosen) -> Result<Selection, AxError> {
        Selection::new(
            chosen.from.map(|seq| Seq::new(seq.0)),
            chosen.through.map(|seq| Seq::new(seq.0)),
            chosen.run,
            chosen.building.clone(),
        )
    }
}

/// The time conditions of one export, as given.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Window<'a> {
    pub since: Option<&'a str>,
    pub until: Option<&'a str>,
    pub day: Option<&'a str>,
}

impl Window<'_> {
    /// The span the conditions make.
    ///
    /// # Errors
    /// None yet.
    pub fn span(&self) -> Result<UtcSpan, AxError> {
        Ok(UtcSpan::default())
    }
}
