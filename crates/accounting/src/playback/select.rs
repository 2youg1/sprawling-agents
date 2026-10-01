// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which lines a playback selection holds (accounting-SPEC.md 8-12 and
//! 8-17). The properties are `crates/accounting/spec/Playback/Select.lean`:
//! the selection is the intersection of every condition given, bounded
//! by the cutoff the walk stops at, and each line is judged on its own,
//! because a line's `t` does not rise with its seq.

use kernel::{Address, AxCode, AxError, EventRecord, RunId, Seq, TimeMs};
use runtime::clock::{UtcSpan, parse_iso};

use super::document::{Chosen, Decimal};

/// The milliseconds of one UTC day: Unix time counts no leap second, so
/// every UTC day is 86 400 seconds long.
const DAY_MS: u64 = 86_400_000;

/// A closed seq range, a run, a building and a UTC span, each optional;
/// a line is selected when every condition given holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    first: Option<Seq>,
    last: Option<Seq>,
    run: Option<RunId>,
    building: Option<Address>,
    span: UtcSpan,
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
            span: UtcSpan::default(),
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
            span: UtcSpan::default(),
        })
    }

    /// The selection with a time condition: a line's envelope `t` must
    /// lie in `span`, the end left out.
    #[must_use]
    pub fn during(self, span: UtcSpan) -> Selection {
        Selection { span, ..self }
    }

    /// Whether `record` is in the selection. The cutoff is the walk's to
    /// enforce: no line after it is ever offered here.
    pub(super) fn admits(&self, record: &EventRecord) -> bool {
        self.holds_seq(record.seq())
            && self.span.contains(record.t())
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
            since: self.span.since().map(|at| Decimal(at.value())),
            until: self.span.until().map(|at| Decimal(at.value())),
        }
    }

    /// The selection a bundle's `source` recorded, for recomputing it.
    ///
    /// # Errors
    /// The same contradictions [`Selection::new`] and `UtcSpan::new`
    /// refuse.
    pub(super) fn from_chosen(chosen: &Chosen) -> Result<Selection, AxError> {
        let moment = |end: Option<Decimal>| end.map(|at| TimeMs::new(at.0));
        Ok(Selection::new(
            chosen.from.map(|seq| Seq::new(seq.0)),
            chosen.through.map(|seq| Seq::new(seq.0)),
            chosen.run,
            chosen.building.clone(),
        )?
        .during(UtcSpan::new(moment(chosen.since), moment(chosen.until))?))
    }
}

/// The time conditions of one export, as the person or the resident
/// wrote them: `since` and `until` in the one shape `runtime::clock::iso`
/// writes, `day` as `YYYY-MM-DD`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Window<'a> {
    pub since: Option<&'a str>,
    pub until: Option<&'a str>,
    pub day: Option<&'a str>,
}

impl Window<'_> {
    /// The span every condition given holds in: the latest start, the
    /// earliest end. No condition is the unbounded span.
    ///
    /// # Errors
    /// `E_INVALID_ARGS` for a moment `parse_iso` refuses, a day that is
    /// not a calendar day in `YYYY-MM-DD`, and conditions whose crossing
    /// holds no moment (`UtcSpan::new`'s refusal).
    pub fn span(&self) -> Result<UtcSpan, AxError> {
        let day = self.day.map(day_of).transpose()?;
        let since = self.since.map(parse_iso).transpose()?;
        let until = self.until.map(parse_iso).transpose()?;
        UtcSpan::new(
            since.into_iter().chain(day.map(|(start, _)| start)).max(),
            until.into_iter().chain(day.map(|(_, end)| end)).min(),
        )
    }
}

/// The first moment of the UTC day `raw` names and the first of the next.
/// The day is read by `parse_iso` as that day's midnight, so the calendar
/// that judges it is the one `runtime::clock` holds.
fn day_of(raw: &str) -> Result<(TimeMs, TimeMs), AxError> {
    let refused = || {
        AxError::failure(AxCode::InvalidArgs, "select a playback day", raw).with_recovery(
            "write the day in UTC as 2026-05-14: a year, a month and a day the calendar has",
        )
    };
    let start = parse_iso(&format!("{raw}T00:00:00Z")).map_err(|_not_a_day| refused())?;
    let end = start
        .value()
        .checked_add(DAY_MS)
        .map(TimeMs::new)
        .ok_or_else(refused)?;
    Ok((start, end))
}
