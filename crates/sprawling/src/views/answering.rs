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
//! **The fold is not here.** How one record moves the views lives with
//! the views themselves (`views::holding`), which is the module whose
//! whole subject is what they hold; this one is the reading side, and
//! keeping the two apart is what stops a question from quietly changing
//! the thing it reports on.

use kernel::EventRecord;

// Where a city keeps its ledger and how a building reads off disk are
// `bin::assembly`'s: it forms the city that laid them out. Borrowed
// rather than copied, so "where the ledger lives" keeps one answer.
use super::holding::Views;
use super::lines::{buildings_of, endpoints_answer, summarize};
use crate::assembly::{ledger_dir, read_building};

impl Views {
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
                    halted: self.halted.iter().cloned().collect(),
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
            channels::Query::Governance => {
                channels::Answer::Governance(channels::GovernanceAnswer {
                    autonomy: self.autonomy.clone(),
                    decided: self.decided.clone(),
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
            channels::Query::Hunks { oid_a, oid_b, path } => {
                match memory::of_file(&self.city_root, *oid_a, memory::Head::Commit(*oid_b), path) {
                    Ok(patch) => channels::Answer::Hunks(Box::new(channels::HunksAnswer {
                        oid_a: *oid_a,
                        oid_b: *oid_b,
                        path: path.clone(),
                        lines: patch
                            .lines
                            .into_iter()
                            .map(|line| channels::PatchLine {
                                number: line.number,
                                text: line.text,
                            })
                            .collect(),
                        withheld: patch
                            .withheld
                            .into_iter()
                            .map(|held| channels::Withheld {
                                number: held.number,
                                reason: held.reason,
                            })
                            .collect(),
                    })),
                    // An oid this city never wrote, for the reason `Changes`
                    // gives: "it changed nothing" and "I cannot read it" are
                    // different answers, and a reader's next move differs.
                    Err(_) => channels::Answer::Unavailable {
                        query: format!("Hunks({oid_a}..{oid_b} {path})"),
                    },
                }
            }
            channels::Query::Commit { oid } => self.commit_answer(*oid),
            channels::Query::Commits {
                building,
                before,
                limit,
            } => channels::Answer::Commits(self.commits_answer(building.as_ref(), *before, *limit)),
            // Three readings that used to run in the browser, answered
            // here since card-6.5 so a second client draws a session
            // without folding the ledger itself.
            channels::Query::Rounds { run } => {
                channels::Answer::Rounds(Box::new(self.rounds_answer(*run)))
            }
            channels::Query::Evidence { run } => {
                channels::Answer::Evidence(self.evidence_answer(*run))
            }
            channels::Query::CostOf { node } => channels::Answer::CostOf(self.cost_of_answer(node)),
            // The tree itself, one level and one file at a time (card-6.4).
            channels::Query::Listing { at } => {
                channels::Answer::Listing(self.listing_answer(at.as_ref()))
            }
            channels::Query::Document { at } => match self.document_answer(at) {
                Some(answer) => channels::Answer::Document(Box::new(answer)),
                // A file this city does not hold, for the reason a building
                // nobody raised is: "I could not look" is its own answer.
                None => channels::Answer::Unavailable {
                    query: format!("Document({})", at.as_str()),
                },
            },
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
