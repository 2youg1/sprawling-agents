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
use super::prepared::{LedgerAsk, LiveAsk, Prepared, unavailable, unavailable_because};
use super::usage::UsageAsk;
use super::usage::UsageQuestion::{self, Export, Mcp, Shells, Skills};

pub(crate) mod automation;
pub(crate) mod github;
mod history;
pub(crate) mod identity;
pub(super) mod preview;
pub(super) mod range;
pub(super) mod reply;
pub(super) mod stored;
use super::lines::known_hosts_answer;
use super::providers::{ProviderAsk, endpoints_answer};

/// How many runs a cost view names besides every active one: a bound
/// on the size of an answer on the wire, not a machine reading, so it is
/// a constant (`crates/sprawling/Spec.lean` §8-106).
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
    /// because it leaves this machine (`crates/sprawling/Spec.lean` §8-120).
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
                .fold(0, u64::saturating_add),
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

    /// The usage questions, answered from the whole ledger once the
    /// snapshot is let go (accounting D49).
    fn usage(&self, question: UsageQuestion) -> Prepared {
        Prepared::Usage(UsageAsk {
            ledger: self.ledger_ask(),
            question,
        })
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
            wire::Query::RunView { run } => return self.run_view_ask(*run),
            wire::Query::ApprovalQueue => wire::Answer::Approvals(wire::ApprovalsAnswer {
                items: self.governance.pending.values().cloned().collect(),
            }),
            wire::Query::Governance => wire::Answer::Governance(wire::GovernanceAnswer {
                autonomy: self.governance.autonomy.clone(),
                decided: self.decided.clone(),
            }),
            wire::Query::CostView => wire::Answer::Cost(Box::new(self.cost_answer())),
            wire::Query::History { before, limit } => return self.history_ask(*before, *limit),
            wire::Query::HistoryRange { from, to, limit } => {
                return self.history_range_ask(*from, *to, *limit);
            }
            wire::Query::Sessions { room } => wire::Answer::Sessions(self.sessions.answer(room)),
            wire::Query::RunHistory { run, before, limit } => {
                return self.run_history_ask(*run, *before, *limit);
            }
            wire::Query::Changes { base, head } => return self.changes_ask(*base, *head),
            wire::Query::Hunks { oid_a, oid_b, path } => {
                return self.hunks_ask(*oid_a, *oid_b, path.clone());
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
                Ok(answer) => wire::Answer::CostOf(answer),
                Err(stopped) => unavailable_because(format!("CostOf({node})"), &stopped),
            },
            // The tree itself, one level and one file at a time.
            wire::Query::Listing { at } => {
                return Prepared::Listing {
                    city_root: self.city_root.clone(),
                    at: at.clone(),
                };
            }
            wire::Query::Document { at } => return self.document_ask(at),
            wire::Query::Find { under, text } => return self.find_ask(under, text),
            wire::Query::Proposals(doc) => return self.proposals_ask(doc),
            wire::Query::OpenProposals => wire::Answer::OpenProposals(self.open_proposals_answer()),
            wire::Query::Range { version, range } => return self.range_ask(*version, *range),
            wire::Query::Versions { at } => return self.versions_ask(at),
            wire::Query::Bytes { version, range } => return self.bytes_ask(*version, *range),
            wire::Query::Export { at, version } => return self.export_ask(at, *version),
            wire::Query::Preview { version, viewport } => {
                return self.preview_ask(*version, *viewport);
            }
            wire::Query::Reply { text, state } => {
                return Prepared::Reply {
                    text: text.clone(),
                    state: *state,
                };
            }
            // What an agent was told, and the store read that recovers it. A run with no prompt
            // yet and an object this city no longer holds are both "I could not look".
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
                return Prepared::Provider(ProviderAsk::Config {
                    city_root: self.city_root.clone(),
                    addr: addr.clone(),
                    vault: self.vault.clone(),
                });
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
                Ok(ask) => return Prepared::GitStatus(ask),
                Err(stopped) => {
                    unavailable_because(format!("GitStatus({})", building.as_str()), &stopped)
                }
            },
            wire::Query::EndpointView => {
                return Prepared::Provider(ProviderAsk::Endpoints {
                    held: endpoints_answer(&self.book),
                    vault: self.vault.clone(),
                });
            }
            wire::Query::KnownHosts => known_hosts_answer(),
            wire::Query::Harnesses => return Prepared::Harnesses(self.reach.programs),
            wire::Query::AgentCatalog => return super::agents::catalog_ask(self),
            wire::Query::ParseAgentSpec { text } => return super::agents::pasted_ask(self, text),
            wire::Query::Devices => unavailable(query.name().to_owned()),
            wire::Query::Doctor => self.doctor_or_unavailable(),
            wire::Query::McpHealth { addr } => {
                return Prepared::McpHealth {
                    live: self.live_ask(),
                    addr: addr.clone(),
                };
            }
            wire::Query::Toolkits => return Prepared::Toolkits(self.live_ask()),
            wire::Query::SkillUsage { skill } => return self.usage(Skills(skill.clone())),
            wire::Query::McpUsage { server } => return self.usage(Mcp(server.clone())),
            wire::Query::UsageExport { what, format } => return self.usage(Export(*what, *format)),
            wire::Query::Shells => return self.usage(Shells),
            wire::Query::NewestRelease => return Prepared::Release(self.reach.registry),
            // The host's privacy page is read by the serving binary's own listener
            // (`crates/sprawling/spec/Privacy/Service.lean`); the views hold nothing of the host.
            wire::Query::Privacy => unavailable_because(
                "Privacy".to_owned(),
                &kernel::AxError::failure(
                    kernel::AxCode::ToolUnavailable,
                    "answer the privacy page",
                    "the city's views do not read host privacy controls",
                )
                .with_recovery("ask the city's own listener, which serves the privacy page"),
            ),
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

    /// What changed between a checkpoint and a later one, or the
    /// working tree, read once the snapshot is let go.
    fn changes_ask(&self, base: kernel::GitOid, head: Option<kernel::GitOid>) -> Prepared {
        Prepared::Changes {
            city_root: self.city_root.clone(),
            base,
            head,
        }
    }

    /// The patch text of one file between two checkpoints, read once
    /// the snapshot is let go.
    fn hunks_ask(&self, oid_a: kernel::GitOid, oid_b: kernel::GitOid, path: String) -> Prepared {
        Prepared::Hunks {
            city_root: self.city_root.clone(),
            oid_a,
            oid_b,
            path,
        }
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
