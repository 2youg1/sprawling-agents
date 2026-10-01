// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The fold every query is answered from, and the lines a page reads off
//! it.
//!
//! **Why it is a projection and not part of the assembly point.** Nothing
//! here decides anything or reaches a provider: it folds records into the
//! answers a client asks for, and `Views::rebuild` throws the whole thing
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

use kernel::UsdMicros;

use super::holding::Views;
use super::prepared::{LedgerAsk, LiveAsk, Prepared, unavailable};

pub(crate) mod automation;
pub(crate) mod github;
mod history;
pub(crate) mod identity;
pub(super) mod range;
use super::lines::{endpoints_answer, known_hosts_answer, summarize};

/// How many runs a cost view names besides every active one: a bound
/// on the size of an answer on the wire, not a machine reading, so it is
/// a constant (sprawling-SPEC section 8-106).
pub(super) const TOP_BILLED: usize = 32;

impl Views {
    /// What this machine had when the city started.
    ///
    /// A city that never looked says so, for the reason a building
    /// nobody raised does: a page given an empty machine instead would
    /// tell a person every tool they have is missing.
    fn doctor_or_unavailable(&self) -> wire::Answer {
        match &self.machine {
            Some(found) => wire::Answer::Doctor(Box::new(found.clone())),
            None => unavailable("Doctor".to_owned()),
        }
    }

    /// One item's newest release, asked once the snapshot is let go
    /// because it leaves this machine (sprawling-SPEC.md 8-120).
    fn upstream_of(&self, item: &str) -> Prepared {
        Prepared::Upstream {
            ask: self.reach.upstream,
            item: item.to_owned(),
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
    fn metrics(&self) -> wire::MetricsAnswer {
        wire::MetricsAnswer {
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

    /// The ledger as a history reader takes it out of the snapshot.
    pub(super) fn ledger_ask(&self) -> LedgerAsk {
        LedgerAsk {
            city_root: self.city_root.clone(),
            index: std::sync::Arc::clone(&self.index),
        }
    }

    /// What a read of now needs, copied out of the snapshot.
    fn live_ask(&self) -> LiveAsk {
        LiveAsk {
            city_root: self.city_root.clone(),
            city: self.city.clone(),
            vault: self.vault.clone(),
        }
    }

    /// Answers one query in one call, lock or no lock.
    #[cfg(test)]
    pub fn answer(&mut self, query: &wire::Query) -> wire::Answer {
        self.prepare(query).finish()
    }

    /// Answers one query, or copies out what its read of the disk, git
    /// or network needs. Every arm either answers or names itself
    /// unavailable; none of them returns an empty result that a reader
    /// would mistake for an empty city.
    pub fn prepare(&self, query: &wire::Query) -> Prepared {
        Prepared::Held(match query {
            wire::Query::CityView => return Prepared::City(self.city_ask()),
            // An evicted run always has records in the Ledger, so a
            // recall that cannot read them is "I could not look"; the
            // Ledger is read after the snapshot is let go.
            wire::Query::RunView { run } => match self.hot.get(run) {
                Some(hot) => wire::Answer::Run(Some(summarize(*run, hot))),
                None if self.hot.was_evicted(run) => {
                    return Prepared::Recalled {
                        ledger: self.ledger_ask(),
                        run: *run,
                    };
                }
                None => wire::Answer::Run(None),
            },
            wire::Query::ApprovalQueue => wire::Answer::Approvals(wire::ApprovalsAnswer {
                items: self.governance.pending.values().cloned().collect(),
            }),
            wire::Query::Governance => wire::Answer::Governance(wire::GovernanceAnswer {
                autonomy: self.governance.autonomy.clone(),
                decided: self.decided.clone(),
            }),
            wire::Query::CostView => wire::Answer::Cost(Box::new(self.cost_answer())),
            wire::Query::History { before, limit } => {
                return Prepared::History {
                    ledger: self.ledger_ask(),
                    before: *before,
                    limit: *limit,
                };
            }
            wire::Query::HistoryRange { from, to, limit } => {
                return Prepared::HistoryRange {
                    ledger: self.ledger_ask(),
                    from: *from,
                    to: *to,
                    limit: *limit,
                };
            }
            wire::Query::Sessions { room } => wire::Answer::Sessions(self.sessions.answer(room)),
            wire::Query::RunHistory { run, before, limit } => {
                return Prepared::RunHistory {
                    ledger: self.ledger_ask(),
                    run: *run,
                    before: *before,
                    limit: *limit,
                };
            }
            wire::Query::Changes { base, head } => {
                return Prepared::Changes {
                    city_root: self.city_root.clone(),
                    base: *base,
                    head: *head,
                };
            }
            wire::Query::Hunks { oid_a, oid_b, path } => {
                return Prepared::Hunks {
                    city_root: self.city_root.clone(),
                    oid_a: *oid_a,
                    oid_b: *oid_b,
                    path: path.clone(),
                };
            }
            wire::Query::Commit { oid } => return self.prepare_commit(*oid),
            wire::Query::Commits {
                building,
                before,
                limit,
            } => return self.prepare_commits(building.as_ref(), *before, *limit),
            // Three readings answered here, so a second client folds no ledger itself.
            wire::Query::Rounds { run } => {
                return Prepared::Rounds {
                    ledger: self.ledger_ask(),
                    run: *run,
                };
            }
            wire::Query::Evidence { run } => {
                return Prepared::Evidence {
                    ledger: self.ledger_ask(),
                    run: *run,
                };
            }
            wire::Query::RunCosts { runs } => wire::Answer::RunCosts(self.run_costs_answer(runs)),
            wire::Query::CostOf { node } => match self.cost_of_answer(node) {
                Some(answer) => wire::Answer::CostOf(answer),
                None => unavailable(format!("CostOf({node})")),
            },
            // The tree itself, one level and one file at a time.
            wire::Query::Listing { at } => {
                return Prepared::Listing {
                    city_root: self.city_root.clone(),
                    at: at.clone(),
                };
            }
            wire::Query::Document { at } => {
                return Prepared::Document {
                    city_root: self.city_root.clone(),
                    at: at.clone(),
                };
            }
            wire::Query::Proposals(doc) => return self.proposals_ask(doc),
            wire::Query::Range { version, range } => return self.range_ask(*version, *range),
            // What an agent was told, and the store read that recovers
            // it. A run with no prompt yet and an object this city no
            // longer holds are both "I could not look".
            wire::Query::Prefix { run } => return Prepared::Prefix(self.prefix_ask(*run)),
            // Read at every asking rather than held: each file is one a person also edits or
            // another page also writes, and a copy kept in this fold would answer with what it
            // said the last time somebody used a page.
            wire::Query::Preferences => return Prepared::Preferences,
            wire::Query::Guide => return Prepared::Guide(self.city_root.clone()),
            wire::Query::Identity => {
                return Prepared::Identity {
                    city_root: self.city_root.clone(),
                };
            }
            wire::Query::Automation => {
                return Prepared::Automation {
                    city_root: self.city_root.clone(),
                };
            }
            wire::Query::GithubLogin(host) => return self.github_of(host.as_deref()),
            wire::Query::Config { addr } => {
                return Prepared::Config {
                    city_root: self.city_root.clone(),
                    addr: addr.clone(),
                };
            }
            wire::Query::Content { locator } => {
                return Prepared::Content {
                    city_root: self.city_root.clone(),
                    locator: locator.clone(),
                };
            }
            wire::Query::Skills { building } => {
                return Prepared::Skills {
                    city_root: self.city_root.clone(),
                    building: building.clone(),
                    pins: self.skill_pins.clone(),
                };
            }
            wire::Query::GitStatus { building } => match self.git_status_ask(building) {
                Some(ask) => return Prepared::GitStatus(ask),
                None => unavailable(format!("GitStatus({})", building.as_str())),
            },
            wire::Query::EndpointView => wire::Answer::Endpoints(endpoints_answer(&self.book)),
            wire::Query::KnownHosts => known_hosts_answer(),
            wire::Query::Harnesses => return Prepared::Harnesses(self.reach.programs),
            wire::Query::Doctor => self.doctor_or_unavailable(),
            wire::Query::McpHealth { addr } => {
                return Prepared::McpHealth {
                    live: self.live_ask(),
                    addr: addr.clone(),
                };
            }
            wire::Query::Toolkits => return Prepared::Toolkits(self.live_ask()),
            wire::Query::NewestRelease => return Prepared::Release(self.reach.registry),
            wire::Query::UpstreamVersion { item } => return self.upstream_of(item),
            wire::Query::BuildingView { addr } => {
                return Prepared::Building {
                    city_root: self.city_root.clone(),
                    addr: addr.clone(),
                    plans: std::sync::Arc::clone(&self.plans),
                };
            }
            wire::Query::InboxView { addr } => wire::Answer::Inbox(wire::InboxAnswer {
                addr: addr.clone(),
                waiting: self.waiting.get(addr).cloned().unwrap_or_default(),
            }),
            wire::Query::DiscardView => wire::Answer::Discards(wire::DiscardAnswer {
                rows: self.discards.values().cloned().collect(),
            }),
            wire::Query::RegistryView => wire::Answer::Registry(wire::RegistryAnswer {
                assets: self.assets.clone(),
            }),
            wire::Query::ArchiveSearch { needle } => {
                return Prepared::Archives {
                    city_root: self.city_root.clone(),
                    needle: needle.clone(),
                };
            }
            wire::Query::Metrics => {
                return Prepared::Metrics {
                    city_root: self.city_root.clone(),
                    held: self.metrics(),
                };
            }
        })
    }

    /// The city's cost view: the attribution report, with `by_run`
    /// narrowed to the active runs and the most billed others.
    fn cost_answer(&self) -> wire::CostAnswer {
        let report = self.attribution.report();
        wire::CostAnswer {
            total: report.total,
            by_run: top_billed(report.by_run, &self.active_names()),
            by_actor: report.by_actor,
            by_segment: report.by_segment,
            by_tool: report.by_tool,
            by_skill: report.by_skill,
            unpriced: wire::UnpricedCalls {
                calls: report.unpriced.calls,
                tokens: report.unpriced.tokens,
            },
        }
    }
}

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
            .filter(|(_, hot)| hot.phase == storage::RunPhase::Active)
            .map(|(run, _)| run.to_string())
            .collect()
    }
}
