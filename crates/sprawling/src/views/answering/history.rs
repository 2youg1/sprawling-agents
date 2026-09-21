// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The history readers: a bounded slice ending before a cursor, one
//! named range with its endpoints echoed, and a run's own transcript.

use kernel::EventRecord;

use crate::assembly::ledger_dir;
use crate::views::holding::Views;

impl Views {
    /// A bounded slice of the one history, ending just before `before`
    /// or at the tail.
    ///
    /// Read from the ledger rather than held: a view that kept the
    /// records would be a second copy of the only history, and the index
    /// already maps a sequence to a byte offset. An unreadable line ends
    /// the slice rather than emptying it - what was read is still true.
    pub(super) fn history(
        &mut self,
        before: Option<kernel::Seq>,
        limit: u32,
    ) -> channels::HistoryAnswer {
        let empty = channels::HistoryAnswer {
            records: Vec::new(),
            earlier: None,
        };
        let dir = ledger_dir(&self.city_root);
        if self.index.refresh(&dir).is_err() {
            return empty;
        }
        let Some(tail) = self.index.tail_seq() else {
            return empty;
        };
        let end = match before {
            None => tail,
            // The record just before the oldest one the caller holds.
            // `Seq::FIRST` is zero and it is the genesis line, so a
            // `before` of it has nothing behind it - and the arithmetic
            // that says so must not also make the genesis unreachable,
            // which is what a floor of one did.
            Some(seq) => match seq.value().checked_sub(1) {
                None => return empty,
                Some(value) => kernel::Seq::new(value),
            },
        };
        let want = u64::from(limit.clamp(1, channels::HISTORY_MAX));
        let start = end.value().saturating_sub(want.saturating_sub(1));
        let mut records = Vec::new();
        let mut reader = self.index.reader(&dir);
        for value in start..=end.value() {
            let Ok(line) = reader.line_at(kernel::Seq::new(value)) else {
                break;
            };
            let Ok(record) = EventRecord::parse_line(&line) else {
                break;
            };
            records.push(record);
        }
        channels::HistoryAnswer {
            records,
            earlier: (start > kernel::Seq::FIRST.value()).then(|| kernel::Seq::new(start)),
        }
    }

    /// The records between two sequence numbers, both ends included.
    ///
    /// The question a page asks after the event stream told it which
    /// range it missed, so both ends are the wire's and neither is a
    /// cursor: the caller holds no position to walk from, only the two
    /// numbers a frame named. The slice is capped at `limit` records and
    /// `next` says where to ask again when the range holds more, because
    /// the alternative - one unbounded answer - is the whole ledger on a
    /// socket.
    ///
    /// A line that will not read ends the slice rather than emptying it,
    /// for the reason `run_history` gives: what was read is still true.
    /// It also ends the walk, so `next` says nothing more can be asked
    /// for - the gaps the Ledger really has are not ranges this can fill,
    /// and a cursor pointing past one would have the page ask for ever.
    pub(super) fn history_range(
        &mut self,
        from: kernel::Seq,
        to: kernel::Seq,
        limit: u32,
    ) -> channels::HistoryRangeAnswer {
        let empty = channels::HistoryRangeAnswer {
            from,
            to,
            records: Vec::new(),
            next: None,
        };
        if to < from {
            return empty;
        }
        let dir = ledger_dir(&self.city_root);
        if self.index.refresh(&dir).is_err() {
            return empty;
        }
        let want = u64::from(limit.clamp(1, channels::HISTORY_MAX));
        let last = from
            .value()
            .saturating_add(want.saturating_sub(1))
            .min(to.value());
        let mut records = Vec::new();
        let mut reader = self.index.reader(&dir);
        // A walk that stopped early found no line at that sequence, which
        // means the Ledger ends there: nothing beyond it can be asked for
        // either, and a cursor pointing past it would have the page ask for
        // ever.
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
        channels::HistoryRangeAnswer {
            from,
            to,
            records,
            next: (walked && last < to.value()).then(|| kernel::Seq::new(last.saturating_add(1))),
        }
    }

    /// One session's slice of the history, ending just before `before`.
    ///
    /// One bound, not two. The index knows which sequences this run
    /// wrote, so the newest `limit` of them are named before a single
    /// line is read and the work is proportional to the answer rather
    /// than to the ledger. What used to need a second bound - a client
    /// naming a session that ended a month ago and making the server
    /// walk the whole history to find out - is no longer reachable, so
    /// the second bound is gone rather than merely unused.
    ///
    /// The lines are then read oldest first, which is both the order the
    /// answer is delivered in and the order the cursor walks without a
    /// seek. A line that will not read ends the slice rather than
    /// emptying it - what was read is still true.
    pub(super) fn run_history(
        &mut self,
        run: kernel::RunId,
        before: Option<kernel::Seq>,
        limit: u32,
    ) -> channels::HistoryAnswer {
        let empty = channels::HistoryAnswer {
            records: Vec::new(),
            earlier: None,
        };
        let dir = ledger_dir(&self.city_root);
        if self.index.refresh(&dir).is_err() {
            return empty;
        }
        let want = usize::try_from(limit.clamp(1, channels::HISTORY_MAX)).unwrap_or(1);
        // One more than was asked for: whether this session wrote
        // anything older is exactly what `earlier` reports, and taking
        // one extra sequence answers it without a second question.
        let mut newest: Vec<kernel::Seq> = self
            .index
            .run_seqs_before(run, before)
            .take(want.saturating_add(1))
            .collect();
        let has_older = newest.len() > want;
        newest.truncate(want);
        // The oldest record handed back, so the next question asks for
        // what is strictly before it and the two pages meet exactly.
        let earlier = has_older.then(|| newest.last().copied()).flatten();
        newest.reverse();
        let mut records = Vec::with_capacity(newest.len());
        let mut reader = self.index.reader(&dir);
        for seq in newest {
            let Ok(line) = reader.line_at(seq) else {
                break;
            };
            let Ok(record) = EventRecord::parse_line(&line) else {
                break;
            };
            records.push(record);
        }
        channels::HistoryAnswer { records, earlier }
    }
}
