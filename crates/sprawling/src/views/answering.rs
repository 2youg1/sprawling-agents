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
use std::path::PathBuf;
use std::sync::Mutex;

use kernel::{Address, AxCode, AxError};

use super::document::document_answer;
use super::git_status::GitStatusAsk;
use super::holding::Views;
use super::listing::listing_answer;

mod history;
use super::lines::{buildings_of, config_answer, endpoints_answer, summarize};
use crate::assembly::read_building;

/// The answer to a question this city could not look up.
///
/// One shape, named once: a reader that met an empty city and a reader
/// that met a city which could not look have to be able to tell the
/// difference, and every caller spelling the refusal itself is how the
/// two start looking alike.
fn unavailable(query: String) -> channels::Answer {
    channels::Answer::Unavailable { query }
}

/// Answers one query from the views the fold shares with every reader,
/// holding them only while [`Views::prepare`] copies out what the query
/// needs: the disk, git or network read in [`Prepared::finish`] runs
/// after the lock is released, so the fold never waits on it.
///
/// # Errors
/// `StorageFatal` when a panic poisoned the view lock: the views no
/// longer follow the ledger, and only a restart rebuilds them.
pub(crate) fn answer_outside_the_lock(
    views: &Mutex<Views>,
    query: &channels::Query,
) -> Result<channels::Answer, AxError> {
    let prepared = views
        .lock()
        .map_err(|_| {
            AxError::failure(
                AxCode::StorageFatal,
                "read the city views",
                "the view lock is poisoned",
            )
            .with_recovery("restart the server; its views rebuild from the ledger")
        })?
        .prepare(query);
    Ok(prepared.finish())
}

/// A query's answer split at the view lock: what the views settled while
/// held, or the small data a read of the disk, git or network needs,
/// copied out so that read can run with the lock released.
pub(crate) enum Prepared {
    /// Answered from the views alone.
    Held(channels::Answer),
    /// The working tree of one building against its last fence.
    GitStatus(GitStatusAsk),
    /// The person's own settings file.
    Preferences,
    /// The configuration ladder of one address.
    Config { city_root: PathBuf, addr: Address },
    /// The release page, which leaves this machine.
    Release,
    /// One level of the tree.
    Listing {
        city_root: PathBuf,
        at: Option<Address>,
    },
    /// The head of one file.
    Document { city_root: PathBuf, at: Address },
}

impl Prepared {
    /// Does the read the views left for after the lock, and answers.
    pub(crate) fn finish(self) -> channels::Answer {
        match self {
            Self::Held(answer) => answer,
            Self::GitStatus(ask) => ask.read(),
            // A settings file that cannot be read is "I could not
            // look", not an empty set of preferences.
            Self::Preferences => match crate::person::read() {
                Ok(settled) => channels::Answer::Preferences(Box::new(settled)),
                Err(_) => unavailable("Preferences".to_owned()),
            },
            // A ladder that cannot be read is "I could not look": the
            // files are the person's own and the page says so rather
            // than drawing figures nothing on disk states.
            Self::Config { city_root, addr } => match config_answer(&city_root, &addr) {
                Ok(answer) => channels::Answer::Config(Box::new(answer)),
                Err(_) => unavailable(format!("Config({})", addr.as_str())),
            },
            // Leaves this machine, and only on a press (channels-SPEC 8-36).
            Self::Release => channels::Answer::Release(Box::new(crate::release::answer())),
            Self::Listing { city_root, at } => {
                channels::Answer::Listing(listing_answer(&city_root, at))
            }
            // A file this city does not hold, for the reason a building
            // nobody raised is: "I could not look" is its own answer.
            Self::Document { city_root, at } => {
                let query = format!("Document({})", at.as_str());
                match document_answer(&city_root, at) {
                    Some(answer) => channels::Answer::Document(Box::new(answer)),
                    None => unavailable(query),
                }
            }
        }
    }
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

    /// How much of everything this city is holding right now.
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
            buildings: u64::try_from(buildings_of(&self.city_root).len()).unwrap_or(u64::MAX),
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
                channels::Answer::History(Box::new(self.history(*before, *limit)))
            }
            channels::Query::HistoryRange { from, to, limit } => {
                channels::Answer::HistoryRange(Box::new(self.history_range(*from, *to, *limit)))
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
                self.hunks_answer(*oid_a, *oid_b, path)
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
                channels::Answer::Rounds(Box::new(self.rounds_answer(*run)))
            }
            channels::Query::Evidence { run } => {
                channels::Answer::Evidence(self.evidence_answer(*run))
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
            channels::Query::Content { locator } => match self.content_answer(locator) {
                Some(answer) => channels::Answer::Content(Box::new(answer)),
                None => unavailable(format!("Content({locator})")),
            },
            channels::Query::Skills { building } => match self.skills_answer(building) {
                Some(answer) => channels::Answer::Skills(Box::new(answer)),
                None => unavailable(format!("Skills({})", building.as_str())),
            },
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
            channels::Query::Metrics => channels::Answer::Metrics(Box::new(self.metrics())),
        })
    }
}

/// One shut scope in the shape a `halt` frame names it.
///
/// The ledger keeps `Scope` and its own spelling; a page is answered in
/// the vocabulary it would use to ask, so nothing on the other side has
/// to take a string apart to know which building it is looking at.
fn named(scope: &kernel::event::Scope) -> channels::HaltScope {
    match scope {
        kernel::event::Scope::City => channels::HaltScope::City,
        kernel::event::Scope::Building(addr) => channels::HaltScope::Building(addr.clone()),
        kernel::event::Scope::Workshop(addr) => channels::HaltScope::Workshop(addr.clone()),
    }
}
