// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{Address, AxCode, AxError, EventRecord, RunId, Seq, TimeMs};
use runtime::clock::{UtcSpan, parse_iso};

use super::document::{Chosen, Decimal};

const DAY_MS: u64 = 86_400_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    first: Option<Seq>,
    last: Option<Seq>,
    run: Option<RunId>,
    building: Option<Address>,
    span: UtcSpan,
}

impl Selection {

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

    #[must_use]
    pub fn during(self, span: UtcSpan) -> Selection {
        Selection { span, ..self }
    }

    pub(super) fn admits(&self, record: &EventRecord) -> bool {
        self.holds_seq(record.seq())
            && self.span.contains(record.t())
            && self.run.is_none_or(|run| record.run() == run)
            && self
                .building
                .as_ref()
                .is_none_or(|building| record.addr().is_some_and(|addr| addr.is_within(building)))
    }

    pub(super) fn holds_seq(&self, seq: Seq) -> bool {
        self.first.is_none_or(|first| seq >= first) && self.last.is_none_or(|last| seq <= last)
    }

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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Window<'a> {
    pub since: Option<&'a str>,
    pub until: Option<&'a str>,
    pub day: Option<&'a str>,
}

impl Window<'_> {

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
