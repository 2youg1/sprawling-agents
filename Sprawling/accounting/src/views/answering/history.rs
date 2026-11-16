// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxCode, AxError, EventRecord, UsdMicros};

use super::super::holding::Views;
use crate::views::lines::summarize;
use crate::views::prepared::{LedgerAsk, Prepared};

impl Views {

    pub(in crate::views) fn run_view_ask(&self, run: kernel::RunId) -> Prepared {
        match self.hot.get(&run) {
            Some(hot) => Prepared::Held(wire::Answer::Run(Some(Box::new(summarize(run, hot))))),
            None if self.hot.was_evicted(&run) => Prepared::Recalled {
                ledger: self.ledger_ask(),
                run,
            },
            None => Prepared::Held(wire::Answer::Run(None)),
        }
    }

    pub(in crate::views) fn history_ask(
        &self,
        before: Option<kernel::Seq>,
        limit: u32,
    ) -> Prepared {
        Prepared::History {
            ledger: self.ledger_ask(),
            before,
            limit,
        }
    }

    pub(in crate::views) fn history_range_ask(
        &self,
        from: kernel::Seq,
        to: kernel::Seq,
        limit: u32,
    ) -> Prepared {
        Prepared::HistoryRange {
            ledger: self.ledger_ask(),
            from,
            to,
            limit,
        }
    }

    pub(in crate::views) fn run_history_ask(
        &self,
        run: kernel::RunId,
        before: Option<kernel::Seq>,
        limit: u32,
    ) -> Prepared {
        Prepared::RunHistory {
            ledger: self.ledger_ask(),
            run,
            before,
            limit,
        }
    }
}

impl LedgerAsk {

    pub(in crate::views) fn history(
        &self,
        before: Option<kernel::Seq>,
        limit: u32,
    ) -> wire::HistoryAnswer {
        let empty = wire::HistoryAnswer {
            records: Vec::new(),
            earlier: None,
        };
        let Ok((index, dir)) = self.indexed() else {
            return empty;
        };
        let Some(tail) = index.tail_seq() else {
            return empty;
        };
        let end = match before {
            None => tail,

            Some(seq) => match seq.value().checked_sub(1) {
                None => return empty,
                Some(value) => kernel::Seq::new(value),
            },
        };
        let want = u64::from(limit.clamp(1, wire::HISTORY_MAX));
        let start = end.value().saturating_sub(want.saturating_sub(1));
        let mut records = Vec::new();
        let mut reader = index.reader(&dir);
        for value in start..=end.value() {
            let Ok(line) = reader.line_at(kernel::Seq::new(value)) else {
                break;
            };
            let Ok(record) = EventRecord::parse_line(&line) else {
                break;
            };
            records.push(record);
        }
        wire::HistoryAnswer {
            records,
            earlier: (start > kernel::Seq::FIRST.value()).then(|| kernel::Seq::new(start)),
        }
    }

    pub(in crate::views) fn history_range(
        &self,
        from: kernel::Seq,
        to: kernel::Seq,
        limit: u32,
    ) -> wire::HistoryRangeAnswer {
        let empty = wire::HistoryRangeAnswer {
            from,
            to,
            records: Vec::new(),
            next: None,
        };
        if to < from {
            return empty;
        }
        let Ok((index, dir)) = self.indexed() else {
            return empty;
        };
        let want = u64::from(limit.clamp(1, wire::HISTORY_MAX));
        let last = from
            .value()
            .saturating_add(want.saturating_sub(1))
            .min(to.value());
        let mut records = Vec::new();
        let mut reader = index.reader(&dir);

        let mut walked = true;
        for value in from.value()..=last {
            let Ok(line) = reader.line_at(kernel::Seq::new(value)) else {
                walked = false;
                break;
            };
            let Ok(record) = EventRecord::parse_line(&line) else {
                walked = false;
                break;
            };
            records.push(record);
        }
        wire::HistoryRangeAnswer {
            from,
            to,
            records,
            next: (walked && last < to.value()).then(|| kernel::Seq::new(last.saturating_add(1))),
        }
    }

    pub(in crate::views) fn run_history(
        &self,
        run: kernel::RunId,
        before: Option<kernel::Seq>,
        limit: u32,
    ) -> wire::HistoryAnswer {
        let empty = wire::HistoryAnswer {
            records: Vec::new(),
            earlier: None,
        };
        let Ok((index, dir)) = self.indexed() else {
            return empty;
        };
        let want = usize::try_from(limit.clamp(1, wire::HISTORY_MAX)).unwrap_or(1);

        let mut newest: Vec<kernel::Seq> = index
            .run_seqs_before(run, before)
            .take(want.saturating_add(1))
            .collect();
        let has_older = newest.len() > want;
        newest.truncate(want);

        let earlier = has_older.then(|| newest.last().copied()).flatten();
        newest.reverse();
        let mut records = Vec::with_capacity(newest.len());
        let mut reader = index.reader(&dir);
        for seq in newest {
            let Ok(line) = reader.line_at(seq) else {
                break;
            };
            let Ok(record) = EventRecord::parse_line(&line) else {
                break;
            };
            records.push(record);
        }
        wire::HistoryAnswer { records, earlier }
    }

    pub(in crate::views) fn recalled(
        &self,
        run: kernel::RunId,
    ) -> Result<wire::RunSummary, AxError> {
        let mut alone = storage::HotView::new();
        self.fold_recalled(run, |record| alone.apply(record))?;
        alone
            .get(&run)
            .map(|hot| summarize(run, hot))
            .ok_or_else(|| {
                AxError::failure(
                    AxCode::PathNotFound,
                    "recall an evicted run",
                    run.to_string(),
                )
                .with_recovery(
                    "the ledger holds no record of this run; ask for the city view again",
                )
            })
    }

    pub(in crate::views) fn recalled_bill(&self, run: kernel::RunId) -> Result<UsdMicros, AxError> {
        let mut alone = storage::Attribution::new();
        self.fold_recalled(run, |record| alone.apply(record))?;
        Ok(alone.billed_to(&run).unwrap_or_default())
    }

    fn fold_recalled(
        &self,
        run: kernel::RunId,
        mut apply: impl FnMut(&EventRecord) -> Result<(), storage::StorageError>,
    ) -> Result<(), AxError> {
        let (index, dir) = self.indexed().map_err(storage::StorageError::into_ax)?;
        let mut oldest_first: Vec<kernel::Seq> = index.run_seqs_before(run, None).collect();
        oldest_first.reverse();
        let mut reader = index.reader(&dir);
        for seq in oldest_first {
            let line = reader
                .line_at(seq)
                .map_err(storage::StorageError::into_ax)?;
            apply(&EventRecord::parse_line(&line)?).map_err(storage::StorageError::into_ax)?;
        }
        Ok(())
    }
}
