// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

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
use super::providers::{ProviderAsk, TableRows, endpoints_answer, figures_answer};

pub(super) const TOP_BILLED: usize = 32;

impl Views {
    fn doctor_or_unavailable(&self) -> wire::Answer {
        match &self.machine {
            Some(found) => wire::Answer::Doctor(Box::new(found.clone())),
            None => unavailable("Doctor".to_owned()),
        }
    }

    fn upstream_of(&self, item: &str) -> Prepared {
        Prepared::Upstream {
            ask: self.reach.upstream,
            item: item.to_owned(),
        }
    }

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

    pub(super) fn ledger_ask(&self) -> LedgerAsk {
        LedgerAsk {
            city_root: self.city_root.clone(),
            index: std::sync::Arc::clone(&self.index),
        }
    }

    fn usage(&self, question: UsageQuestion) -> Prepared {
        Prepared::Usage(UsageAsk {
            ledger: self.ledger_ask(),
            question,
        })
    }

    fn live_ask(&self) -> LiveAsk {
        LiveAsk {
            city_root: self.city_root.clone(),
            city: self.city.clone(),
            vault: self.vault.clone(),
        }
    }

    #[cfg(test)]
    pub fn answer(&mut self, query: &wire::Query) -> wire::Answer {
        self.prepare(query).finish()
    }

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
            wire::Query::Layers { addr } => return self.layers_ask(addr),
            wire::Query::Catalog { building } => return self.catalog_ask(building),
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

            wire::Query::Prefix { run } => return Prepared::Prefix(self.prefix_ask(*run)),
            wire::Query::PromptShape { run } => {
                return Prepared::PromptShape(self.prompt_shape_ask(*run));
            }

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
            wire::Query::Template { template, addr } => template_answer(template, addr),
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
            wire::Query::ModelFigures {
                endpoint,
                base_url,
                face,
                models,
            } => figures_answer(
                &self.book,
                &TableRows {
                    endpoint: endpoint.as_str(),
                    base_url,
                    face: *face,
                    models,
                },
            ),
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

            wire::Query::Privacy => privacy_unavailable(),
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

    fn changes_ask(&self, base: kernel::GitOid, head: Option<kernel::GitOid>) -> Prepared {
        Prepared::Changes {
            city_root: self.city_root.clone(),
            base,
            head,
        }
    }

    fn hunks_ask(&self, oid_a: kernel::GitOid, oid_b: kernel::GitOid, path: String) -> Prepared {
        Prepared::Hunks {
            city_root: self.city_root.clone(),
            oid_a,
            oid_b,
            path,
        }
    }

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
            renewals: wire::Renewals {
                billed: report.renewals.billed,
                read: report.renewals.read,
                late: report.renewals.late,
                unreported: report.renewals.unreported,
                refused: report.renewals.refused,
            },
        }
    }
}

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
    fn active_names(&self) -> BTreeSet<String> {
        self.hot
            .runs()
            .filter(|(_, hot)| hot.phase == storage::RunPhase::Active)
            .map(|(run, _)| run.to_string())
            .collect()
    }
}

fn privacy_unavailable() -> wire::Answer {
    unavailable_because(
        "Privacy".to_owned(),
        &kernel::AxError::failure(
            kernel::AxCode::ToolUnavailable,
            "answer the privacy page",
            "the city's views do not read host privacy controls",
        )
        .with_recovery("ask the city's own listener, which serves the privacy page"),
    )
}

fn template_answer(template: &wire::TemplateName, addr: &kernel::Address) -> wire::Answer {
    match city::BuildingTemplate::parse(template.as_str())
        .and_then(|known| city::template_rules(addr, known))
    {
        Ok(rules) => wire::Answer::Template(Box::new(wire::TemplateAnswer {
            template: template.clone(),
            addr: addr.clone(),
            rules,
        })),
        Err(stopped) => unavailable_because("Template".to_owned(), &stopped),
    }
}
