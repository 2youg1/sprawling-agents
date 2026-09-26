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
use std::collections::BTreeSet;
use std::sync::Mutex;

use kernel::{AxCode, AxError, UsdMicros};

use super::holding::Views;
use super::prepared::{Prepared, unavailable};

mod history;
use super::lines::{endpoints_answer, summarize};

/// How many runs a cost view names besides every active one: a bound
/// on the size of an answer on the wire, not a machine reading, so it is
/// a constant (sprawling-SPEC section 8-90).
pub(super) const TOP_BILLED: usize = 32;

/// Answers one query from the views the fold shares with every reader,
/// holding them only while [`Views::prepare`] copies out what the query
/// needs: the disk, git or network read in [`Prepared::finish`] runs
/// after the lock is released, so the fold never waits on it.
///
/// The answer is dated under the same lock its copy is taken under, so
/// the date is exactly the first record the answer does not reflect.
///
/// # Errors
/// `StorageFatal` when a panic poisoned the view lock: the views no
/// longer follow the ledger, and only a restart rebuilds them. The date
/// is then the first seq, which every record follows.
pub(crate) fn answer_outside_the_lock(
    views: &Mutex<Views>,
    query: &channels::Query,
) -> (kernel::Seq, Result<channels::Answer, AxError>) {
    let Ok(mut held) = views.lock() else {
        return (
            kernel::Seq::FIRST,
            Err(AxError::failure(
                AxCode::StorageFatal,
                "read the city views",
                "the view lock is poisoned",
            )
            .with_recovery("restart the server; its views rebuild from the ledger")),
        );
    };
    let as_of = held.next_unfolded();
    let prepared = held.prepare(query);
    drop(held);
    (as_of, Ok(prepared.finish()))
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

    /// Answers one query in one call, lock or no lock.
    #[cfg(test)]
    pub(crate) fn answer(&mut self, query: &channels::Query) -> channels::Answer {
        self.prepare(query).finish()
    }

    /// Answers one query, or copies out what its read of the disk, git
    /// or network needs. Every arm either answers or names itself
    /// unavailable; none of them returns an empty result that a reader
    /// would mistake for an empty city.
    pub(crate) fn prepare(&mut self, query: &channels::Query) -> Prepared {
        Prepared::Held(match query {
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
                    halted: self.governance.halted.iter().map(named).collect(),
                })
            }
            // An evicted run always has records in the Ledger, so a
            // recall that cannot read them is "I could not look".
            channels::Query::RunView { run } => match self.hot.get(run) {
                Some(hot) => channels::Answer::Run(Some(summarize(*run, hot))),
                None if self.hot.was_evicted(run) => match self.recalled(*run) {
                    Some(summary) => channels::Answer::Run(Some(summary)),
                    None => unavailable(format!("RunView({run})")),
                },
                None => channels::Answer::Run(None),
            },
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
                    by_run: top_billed(report.by_run, &self.active_names()),
                    by_actor: report.by_actor,
                    by_segment: report.by_segment,
                    by_tool: report.by_tool,
                    by_skill: report.by_skill,
                    unpriced: channels::UnpricedCalls {
                        calls: report.unpriced.calls,
                        tokens: report.unpriced.tokens,
                    },
                }))
            }
            channels::Query::History { before, limit } => {
                channels::Answer::History(Box::new(self.history(*before, *limit)))
            }
            channels::Query::HistoryRange { from, to, limit } => {
                channels::Answer::HistoryRange(Box::new(self.history_range(*from, *to, *limit)))
            }
            channels::Query::RunHistory { run, before, limit } => {
                channels::Answer::History(Box::new(self.run_history(*run, *before, *limit)))
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
            } => match self.commits_answer(building.as_ref(), *before, *limit) {
                Some(page) => channels::Answer::Commits(page),
                None => unavailable(format!("Commits({before:?})")),
            },
            // Three readings answered here so a second client draws a
            // session without folding the ledger itself.
            channels::Query::Rounds { run } => {
                channels::Answer::Rounds(Box::new(self.rounds_answer(*run)))
            }
            channels::Query::Evidence { run } => {
                channels::Answer::Evidence(self.evidence_answer(*run))
            }
            channels::Query::RunCosts { runs } => {
                channels::Answer::RunCosts(self.run_costs_answer(runs))
            }
            channels::Query::CostOf { node } => match self.cost_of_answer(node) {
                Some(answer) => channels::Answer::CostOf(answer),
                None => unavailable(format!("CostOf({node})")),
            },
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
            channels::Query::Prefix { run } => match self.prefix_answer(*run) {
                Some(answer) => channels::Answer::Prefix(Box::new(answer)),
                None => unavailable(format!("Prefix({run})")),
            },
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
                match self.git_status_ask(building) {
                    Some(ask) => return Prepared::GitStatus(ask),
                    None => unavailable(format!("GitStatus({})", building.as_str())),
                }
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
                let city_root = self.city_root.clone();
                let plan = self.plans.of(&city_root, addr);
                return Prepared::Building {
                    city_root,
                    addr: addr.clone(),
                    plan,
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

/// One shut scope in the shape a `halt` frame names it.
///
/// The ledger keeps `Scope` and its own spelling; a page is answered in
/// the vocabulary it would use to ask, so nothing on the other side has
/// to take a string apart to know which building it is looking at.
/// The rows of a cost view's `by_run`: every run `active` names, and
/// the [`TOP_BILLED`] other runs billed most (a tie goes to the lower
/// name), in name order like the report they come from.
///
/// The cut is one `select_nth_unstable_by` over the other runs, so
/// asking costs O(runs) and never sorts them all.
fn top_billed(
    by_run: Vec<(String, UsdMicros)>,
    active: &BTreeSet<String>,
) -> Vec<(String, UsdMicros)> {
    let (mut shown, mut others): (Vec<_>, Vec<_>) = by_run
        .into_iter()
        .partition(|(run, _)| active.contains(run));
    if let Some(last) = TOP_BILLED.checked_sub(1)
        && others.len() > TOP_BILLED
    {
        others.select_nth_unstable_by(last, |(left, paid_left), (right, paid_right)| {
            paid_right.cmp(paid_left).then_with(|| left.cmp(right))
        });
        others.truncate(TOP_BILLED);
    }
    shown.append(&mut others);
    shown.sort_unstable_by(|(left, _), (right, _)| left.cmp(right));
    shown
}

impl Views {
    /// The active runs under the name the attribution keys them by,
    /// their own display form.
    fn active_names(&self) -> BTreeSet<String> {
        self.hot
            .runs()
            .filter(|(_, hot)| hot.phase == memory::RunPhase::Active)
            .map(|(run, _)| run.to_string())
            .collect()
    }
}

fn named(scope: &kernel::event::Scope) -> channels::HaltScope {
    match scope {
        kernel::event::Scope::City => channels::HaltScope::City,
        kernel::event::Scope::Building(addr) => channels::HaltScope::Building(addr.clone()),
        kernel::event::Scope::Workshop(addr) => channels::HaltScope::Workshop(addr.clone()),
    }
}
