// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The fold every query is answered from, and the lines a page reads off
//! it.
//!
//! **Why it is a projection and not part of the assembly point.** Nothing
//! here decides anything or reaches a provider: it folds records into the
//! answers a client asks for, and `rebuild_views` throws the whole thing
//! away and folds the ledger again to get the same bytes. That is
//! ARCHITECTURE.md section 9 shape 7, while `bin::assembly` is an
//! adapter - and a file holding two shapes is what section 9 says a split
//! looks like.
//!
//! **What it deliberately does not hold.** The plans are
//! `crate::plan_view`'s and are read through it; a second parse here
//! would be a second answer to "what is stuck and why", and only one of
//! them would be folding the records that say why. What waits in a room
//! is folded from signal records rather than read off a queue, because a
//! queue answers by being consumed and a view that consumed what it
//! showed would change the thing it reports on.

use kernel::{AxCode, AxError, EventKind, EventRecord};

// Where a city keeps its ledger and how a building reads off disk are
// `bin::assembly`'s: it forms the city that laid them out. Borrowed
// rather than copied, so "where the ledger lives" keeps one answer.
use super::holding::Views;
use super::lines::{
    buildings_of, discard_lines, endpoints_answer, pursuit_from, registry_line, signal_line,
    summarize,
};
use crate::assembly::{ledger_dir, read_building};

impl Views {
    /// Folds one record into every view that cares about it.
    ///
    /// # Errors
    /// Propagates a view's own refusal to fold a malformed record.
    pub(crate) fn apply(&mut self, record: &EventRecord) -> Result<(), AxError> {
        self.hot
            .apply(record)
            .map_err(memory::MemoryError::into_ax)?;
        self.attribution
            .apply(record)
            .map_err(memory::MemoryError::into_ax)?;
        self.book.apply(record)?;
        self.plans.apply(record);
        self.events = self.events.saturating_add(1);
        match record.kind() {
            EventKind::CityInitialized => {
                self.city = record.addr().cloned();
            }
            EventKind::SignalEnqueued => {
                if let Some((room, line)) = signal_line(record) {
                    self.waiting.entry(room).or_default().push(line);
                }
            }
            EventKind::SignalConsumed => {
                if let Some((room, line)) = signal_line(record)
                    && let Some(queue) = self.waiting.get_mut(&room)
                {
                    queue.retain(|held| held.id != line.id);
                }
            }
            EventKind::PursuitChanged => {
                if let Some((addr, held)) = pursuit_from(record) {
                    match held {
                        Some(entry) => {
                            self.pursuits.insert(addr, entry);
                        }
                        None => {
                            self.pursuits.remove(&addr);
                        }
                    }
                }
            }
            EventKind::FileDiscarded => {
                for line in discard_lines(record) {
                    self.discards.insert(line.path.clone(), line);
                }
            }
            EventKind::DiscardRestored => {
                for line in discard_lines(record) {
                    if let Some(held) = self.discards.get_mut(&line.path) {
                        held.restored = true;
                    }
                }
            }
            EventKind::AssetArchived => {
                if let Some(line) = registry_line(record) {
                    self.assets.push(line);
                }
            }
            EventKind::ApprovalRequested => {
                // The payload *is* the item: it was written by serialising
                // one, so it reads back as one. Rebuilding a lesser shape
                // out of hand-picked fields is how this view came to show
                // every waiting item as "(no summary recorded)" - the field
                // it read had never been written by anybody.
                let value = serde_json::Value::Object(record.data().as_map().clone());
                let item: kernel::ApprovalItem = serde_json::from_value(value).map_err(|err| {
                    AxError::failure(
                        AxCode::WireMismatch,
                        "fold an approval into the queue",
                        format!("seq {}: {err}", record.seq().value()),
                    )
                    .with_recovery(
                        "the record stands; this view skips it and the observer reports it",
                    )
                })?;
                self.approvals.insert(item.id.as_str().to_owned(), item);
            }
            EventKind::ApprovalResolved => {
                if let Some(id) = record
                    .data()
                    .as_map()
                    .get("id")
                    .and_then(serde_json::Value::as_str)
                {
                    self.approvals.remove(id);
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// A bounded slice of the one history, ending just before `before`
    /// or at the tail.
    ///
    /// Read from the ledger rather than held: a view that kept the
    /// records would be a second copy of the only history, and the index
    /// already maps a sequence to a byte offset. An unreadable line ends
    /// the slice rather than emptying it - what was read is still true.
    fn history(&mut self, before: Option<kernel::Seq>, limit: u32) -> channels::HistoryAnswer {
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
    fn run_history(
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

    /// Answers one query. Every arm either answers or names itself
    /// unavailable; none of them returns an empty result that a reader
    /// would mistake for an empty city.
    pub(crate) fn answer(&mut self, query: &channels::Query) -> channels::Answer {
        match query {
            channels::Query::CityView => {
                let runs: Vec<channels::RunSummary> = self
                    .hot
                    .runs()
                    .map(|(run, hot)| summarize(*run, hot))
                    .collect();
                let active = self.hot.active_count();
                let frozen = self.hot.frozen_count();
                channels::Answer::City(channels::CityAnswer {
                    runs,
                    active,
                    frozen,
                    buildings: self.spine(),
                    pursuits: self.pursuit_lines(),
                })
            }
            channels::Query::RunView { run } => {
                channels::Answer::Run(self.hot.get(run).map(|hot| summarize(*run, hot)))
            }
            channels::Query::ApprovalQueue => {
                channels::Answer::Approvals(channels::ApprovalsAnswer {
                    items: self.approvals.values().cloned().collect(),
                })
            }
            channels::Query::CostView => {
                let report = self.attribution.report();
                channels::Answer::Cost(Box::new(channels::CostAnswer {
                    total: report.total,
                    by_run: report.by_run,
                    by_actor: report.by_actor,
                    by_segment: report.by_segment,
                    by_tool: report.by_tool,
                    by_skill: report.by_skill,
                }))
            }
            channels::Query::History { before, limit } => {
                channels::Answer::History(Box::new(self.history(*before, *limit)))
            }
            channels::Query::RunHistory { run, before, limit } => {
                channels::Answer::History(Box::new(self.run_history(*run, *before, *limit)))
            }
            // A checkpoint this city does not hold is `Unavailable`, not
            // an empty change list: "nothing moved" and "I could not
            // look" are different answers and a reader acts differently
            // on each.
            channels::Query::Changes { base, head } => {
                let far = match head {
                    Some(oid) => memory::Head::Commit(*oid),
                    None => memory::Head::WorkingTree,
                };
                match memory::between(&self.city_root, *base, far) {
                    Ok(files) => channels::Answer::Changes(channels::ChangesAnswer {
                        base: *base,
                        head: *head,
                        files,
                    }),
                    Err(_) => channels::Answer::Unavailable {
                        query: format!("Changes({base})"),
                    },
                }
            }
            channels::Query::EndpointView => {
                channels::Answer::Endpoints(endpoints_answer(&self.book))
            }
            channels::Query::BuildingView { addr } => {
                let root = self.city_root.clone();
                let plan = self.plans.of(&root, addr);
                match read_building(&root, addr, plan) {
                    Some(answer) => channels::Answer::Building(Box::new(answer)),
                    // A building nobody raised is not an empty building. The
                    // page needs to be able to tell those apart.
                    None => channels::Answer::Unavailable {
                        query: format!("BuildingView({})", addr.as_str()),
                    },
                }
            }
            channels::Query::InboxView { addr } => channels::Answer::Inbox(channels::InboxAnswer {
                addr: addr.clone(),
                waiting: self.waiting.get(addr).cloned().unwrap_or_default(),
            }),
            channels::Query::DiscardView => channels::Answer::Discards(channels::DiscardAnswer {
                rows: self.discards.values().cloned().collect(),
            }),
            channels::Query::RegistryView => channels::Answer::Registry(channels::RegistryAnswer {
                assets: self.assets.clone(),
            }),
            channels::Query::ArchiveSearch { needle } => {
                channels::Answer::Archive(self.search_archives(needle))
            }
            channels::Query::Metrics => {
                channels::Answer::Metrics(Box::new(channels::MetricsAnswer {
                    events: self.events,
                    runs_active: self.hot.active_count(),
                    runs_frozen: self.hot.frozen_count(),
                    buildings: u64::try_from(buildings_of(&self.city_root).len())
                        .unwrap_or(u64::MAX),
                    approvals_waiting: u64::try_from(self.approvals.len()).unwrap_or(u64::MAX),
                    signals_waiting: self
                        .waiting
                        .values()
                        .map(|queue| u64::try_from(queue.len()).unwrap_or(u64::MAX))
                        .sum(),
                    discards_outstanding: self
                        .discards
                        .values()
                        .filter(|row| !row.restored)
                        .count()
                        .try_into()
                        .unwrap_or(u64::MAX),
                }))
            }
        }
    }

    /// Every archive entry whose subject contains `needle`, across every
    /// building, read from the shelves at the moment of asking.
    fn search_archives(&self, needle: &str) -> channels::ArchiveAnswer {
        let mut hits = Vec::new();
        let wanted = needle.to_lowercase();
        for building in buildings_of(&self.city_root) {
            let Ok(entries) = city::archive_index(&self.city_root, &building) else {
                continue;
            };
            for entry in entries {
                if !entry.subject.to_lowercase().contains(&wanted) {
                    continue;
                }
                hits.push(channels::ArchiveHit {
                    building: building.clone(),
                    kind: entry.kind.as_str().to_owned(),
                    day: entry.day,
                    subject: entry.subject,
                });
            }
        }
        channels::ArchiveAnswer {
            needle: needle.to_owned(),
            hits,
        }
    }
}
