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

// Where a city keeps its ledger and how a building reads off disk are
// `bin::assembly`'s: it forms the city that laid them out. Borrowed
// rather than copied, so "where the ledger lives" keeps one answer.
use std::sync::{Arc, Mutex, PoisonError};

use super::holding::Views;
use super::prepared::{LedgerAsk, Prepared, unavailable};

mod history;
use super::lines::{endpoints_answer, summarize};

/// The views the fold last finished, handed to every reader
/// (sprawling-SPEC.md 8-93).
///
/// The lock covers one `Arc` copy or swap and nothing that can panic,
/// so even a poisoned lock holds a whole `Arc`, and it is read as one.
pub(crate) struct Published {
    current: Mutex<Arc<Views>>,
}

impl Published {
    pub(crate) fn new(views: Views) -> Published {
        Published {
            current: Mutex::new(Arc::new(views)),
        }
    }

    /// The views as the fold last published them. Held only while a
    /// query copies out what it needs, because the fold takes a retired
    /// copy back only once no reader holds it.
    pub(crate) fn snapshot(&self) -> Arc<Views> {
        Arc::clone(&self.current.lock().unwrap_or_else(PoisonError::into_inner))
    }

    /// Publishes `latest` and hands back the copy it replaces, which is
    /// dropped or reclaimed outside the lock.
    pub(crate) fn replace(&self, latest: Arc<Views>) -> Arc<Views> {
        std::mem::replace(
            &mut *self.current.lock().unwrap_or_else(PoisonError::into_inner),
            latest,
        )
    }
}

/// Answers one query from a snapshot of the views, holding it only while
/// [`Views::prepare`] copies out what the query needs: the disk, git or
/// network read in [`Prepared::finish`] runs after the snapshot is let
/// go, so neither the fold nor another reader waits on it.
pub(crate) fn answer_outside_the_lock(
    views: &Published,
    query: &channels::Query,
) -> channels::Answer {
    let prepared = views.snapshot().prepare(query);
    prepared.finish()
}

impl Views {
    /// What this machine had when the city started.
    ///
    /// A city that never looked says so, for the reason a building
    /// nobody raised does: a page given an empty machine instead would
    /// tell a person every tool they have is missing.
    fn doctor_or_unavailable(&self) -> channels::Answer {
        match &self.machine {
            Some(found) => channels::Answer::Doctor(Box::new(found.clone())),
            None => unavailable("Doctor".to_owned()),
        }
    }

    /// How much of everything this city is holding right now, but for
    /// the building count, which `Prepared::finish` reads off the
    /// directory once the lock is released.
    ///
    /// A count that cannot be expressed is reported as the largest
    /// count this wire can carry rather than dropped: a saturated
    /// figure is visibly wrong on a page, and an absent one reads as
    /// zero.
    fn metrics(&self) -> channels::MetricsAnswer {
        channels::MetricsAnswer {
            events: self.events,
            runs_active: self.hot.active_count(),
            runs_frozen: self.hot.frozen_count(),
            buildings: 0,
            approvals_waiting: u64::try_from(self.governance.pending.len()).unwrap_or(u64::MAX),
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
        }
    }

    /// The ledger as a history reader takes it out of the view lock.
    pub(super) fn ledger_ask(&self) -> LedgerAsk {
        LedgerAsk {
            city_root: self.city_root.clone(),
            index: std::sync::Arc::clone(&self.index),
        }
    }

    /// Answers one query in one call, lock or no lock.
    #[cfg(test)]
    pub(crate) fn answer(&mut self, query: &channels::Query) -> channels::Answer {
        self.prepare(query).finish()
    }

    /// Answers one query, or copies out what its read of the disk, git
    /// or network needs. Every arm either answers or names itself
    /// unavailable; none of them returns an empty result that a reader
    /// would mistake for an empty city.
    pub(crate) fn prepare(&self, query: &channels::Query) -> Prepared {
        Prepared::Held(match query {
            channels::Query::CityView => return Prepared::City(self.city_ask()),
            channels::Query::RunView { run } => {
                channels::Answer::Run(self.hot.get(run).map(|hot| summarize(*run, hot)))
            }
            channels::Query::ApprovalQueue => {
                channels::Answer::Approvals(channels::ApprovalsAnswer {
                    items: self.governance.pending.values().cloned().collect(),
                })
            }
            channels::Query::Governance => {
                channels::Answer::Governance(channels::GovernanceAnswer {
                    autonomy: self.governance.autonomy.clone(),
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
                return Prepared::History {
                    ledger: self.ledger_ask(),
                    before: *before,
                    limit: *limit,
                };
            }
            channels::Query::HistoryRange { from, to, limit } => {
                return Prepared::HistoryRange {
                    ledger: self.ledger_ask(),
                    from: *from,
                    to: *to,
                    limit: *limit,
                };
            }
            channels::Query::RunHistory { run, before, limit } => {
                return Prepared::RunHistory {
                    ledger: self.ledger_ask(),
                    run: *run,
                    before: *before,
                    limit: *limit,
                };
            }
            channels::Query::Changes { base, head } => {
                return Prepared::Changes {
                    city_root: self.city_root.clone(),
                    base: *base,
                    head: *head,
                };
            }
            channels::Query::Hunks { oid_a, oid_b, path } => {
                return Prepared::Hunks {
                    city_root: self.city_root.clone(),
                    oid_a: *oid_a,
                    oid_b: *oid_b,
                    path: path.clone(),
                };
            }
            channels::Query::Commit { oid } => self.commit_answer(*oid),
            channels::Query::Commits {
                building,
                before,
                limit,
            } => channels::Answer::Commits(self.commits_answer(building.as_ref(), *before, *limit)),
            // Three readings answered here so a second client draws a
            // session without folding the ledger itself.
            channels::Query::Rounds { run } => {
                return Prepared::Rounds {
                    ledger: self.ledger_ask(),
                    run: *run,
                };
            }
            channels::Query::Evidence { run } => {
                return Prepared::Evidence {
                    ledger: self.ledger_ask(),
                    run: *run,
                };
            }
            channels::Query::CostOf { node } => channels::Answer::CostOf(self.cost_of_answer(node)),
            // The tree itself, one level and one file at a time.
            channels::Query::Listing { at } => {
                return Prepared::Listing {
                    city_root: self.city_root.clone(),
                    at: at.clone(),
                };
            }
            channels::Query::Document { at } => {
                return Prepared::Document {
                    city_root: self.city_root.clone(),
                    at: at.clone(),
                };
            }
            // What an agent was told, and the store read that recovers
            // it. A run with no prompt yet and an object this city no
            // longer holds are both "I could not look".
            channels::Query::Prefix { run } => return Prepared::Prefix(self.prefix_ask(*run)),
            // Read at every asking rather than held: the file is one a
            // person also edits, and a copy kept in this fold would
            // answer with what it said the last time somebody used a
            // page.
            channels::Query::Preferences => return Prepared::Preferences,
            channels::Query::Config { addr } => {
                return Prepared::Config {
                    city_root: self.city_root.clone(),
                    addr: addr.clone(),
                };
            }
            channels::Query::Content { locator } => {
                return Prepared::Content {
                    city_root: self.city_root.clone(),
                    locator: locator.clone(),
                };
            }
            channels::Query::Skills { building } => {
                return Prepared::Skills {
                    city_root: self.city_root.clone(),
                    building: building.clone(),
                    pins: self.skill_pins.clone(),
                };
            }
            channels::Query::GitStatus { building } => {
                return Prepared::GitStatus(self.git_status_ask(building));
            }
            channels::Query::EndpointView => {
                channels::Answer::Endpoints(endpoints_answer(&self.book))
            }
            channels::Query::Doctor => self.doctor_or_unavailable(),
            channels::Query::McpHealth { addr } => {
                channels::Answer::McpHealth(Box::new(self.mcp_health_answer(addr)))
            }
            channels::Query::Toolkits => {
                channels::Answer::Toolkits(Box::new(self.toolkits_answer()))
            }
            channels::Query::Release => return Prepared::Release,
            channels::Query::BuildingView { addr } => {
                return Prepared::Building {
                    city_root: self.city_root.clone(),
                    addr: addr.clone(),
                    plans: std::sync::Arc::clone(&self.plans),
                };
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
                return Prepared::Archives {
                    city_root: self.city_root.clone(),
                    needle: needle.clone(),
                };
            }
            channels::Query::Metrics => {
                return Prepared::Metrics {
                    city_root: self.city_root.clone(),
                    held: self.metrics(),
                };
            }
        })
    }
}
