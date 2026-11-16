// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

mod leaving;

pub use leaving::LiveAsk;

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};

use kernel::{Address, B3Hash, GitOid, Locator, RunId, Seq};

use super::archives::search_archives;
use super::building_page::read_building;
use super::city::CityAsk;
use super::commits::CommitsAsk;
use super::document::document_answer;
use super::finding::find_answer;
use super::git_status::GitStatusAsk;
use super::hunks::hunks_answer;
use super::lines::buildings_of;
use super::listing::listing_answer;
use super::prefix::{PrefixAsk, PromptShapeAsk, content_answer};
use super::skills::{SkillPins, skills_answer};
use crate::plan_view::{PlanView, plans_of};

pub(super) fn unavailable(query: String) -> wire::Answer {
    wire::Answer::Unavailable {
        query,
        reason: None,
    }
}

pub(super) fn unavailable_because(query: String, stopped: &kernel::AxError) -> wire::Answer {
    wire::Answer::Unavailable {
        query,
        reason: Some(stopped.to_string()),
    }
}

pub enum Prepared {
    Held(wire::Answer),

    GitStatus(GitStatusAsk),

    Commits(CommitsAsk),

    Preferences,

    Identity {
        city_root: PathBuf,
    },

    Automation {
        city_root: PathBuf,
    },

    GithubLogin {
        ask: Option<fn(&str) -> wire::GithubReading>,
        host: Option<String>,
    },

    Guide(PathBuf),

    Provider(super::providers::ProviderAsk),

    Release(Option<fn() -> wire::ReleaseAnswer>),

    Agents(super::agents::CatalogAsk),

    Refused(kernel::AxError),

    Upstream {
        ask: Option<fn(&str) -> wire::DoctorUpstream>,
        item: String,
    },

    Changes {
        city_root: PathBuf,
        base: GitOid,
        head: Option<GitOid>,
    },

    Hunks {
        city_root: PathBuf,
        oid_a: GitOid,
        oid_b: GitOid,
        path: String,
    },

    Content {
        city_root: PathBuf,
        locator: Locator,
    },

    Archives {
        city_root: PathBuf,
        needle: String,
    },

    Skills {
        city_root: PathBuf,
        building: Address,
        pins: SkillPins,
    },

    McpHealth {
        live: LiveAsk,
        addr: Address,
    },

    Toolkits(LiveAsk),

    Usage(super::usage::UsageAsk),

    Metrics {
        city_root: PathBuf,
        held: wire::MetricsAnswer,
    },

    Listing {
        city_root: PathBuf,
        at: Option<Address>,
    },

    Find(PathBuf, Address, String),

    Document {
        city_root: PathBuf,
        at: Address,
    },

    Proposals {
        ledger: LedgerAsk,
        doc: Address,
        open: Vec<documents::Offer>,
    },

    Range {
        city_root: PathBuf,
        version: B3Hash,
        range: documents::Span,
    },

    Preview {
        city_root: PathBuf,
        version: B3Hash,
        viewport: documents::Span,
    },

    Stored(super::answering::stored::StoredAsk),

    Versions(super::versions::VersionsAsk),

    Layers {
        city_root: PathBuf,
        addr: Address,
    },

    Catalog(super::catalog::CatalogAsk),

    Reply {
        text: String,
        state: documents::ReplyState,
    },

    City(CityAsk),

    Building {
        city_root: PathBuf,
        addr: Address,
        plans: Arc<Mutex<PlanView>>,
    },

    Prefix(PrefixAsk),

    PromptShape(PromptShapeAsk),

    History {
        ledger: LedgerAsk,
        before: Option<Seq>,
        limit: u32,
    },

    HistoryRange {
        ledger: LedgerAsk,
        from: Seq,
        to: Seq,
        limit: u32,
    },

    RunHistory {
        ledger: LedgerAsk,
        run: RunId,
        before: Option<Seq>,
        limit: u32,
    },

    Rounds {
        ledger: LedgerAsk,
        run: RunId,
    },

    Evidence {
        ledger: LedgerAsk,
        run: RunId,
    },

    Recalled {
        ledger: LedgerAsk,
        run: RunId,
    },
}

pub struct LedgerAsk {
    pub(super) city_root: PathBuf,
    pub(super) index: Arc<Mutex<storage::LedgerIndex>>,
}

impl LedgerAsk {
    pub(super) fn indexed(
        &self,
    ) -> Result<(MutexGuard<'_, storage::LedgerIndex>, PathBuf), storage::StorageError> {
        let dir = kernel::layout::CityLayout::new(&self.city_root).ledger();
        let mut index = self.index.lock().unwrap_or_else(|poisoned| {
            let mut index = poisoned.into_inner();
            *index = storage::LedgerIndex::empty();
            self.index.clear_poison();
            index
        });
        index.refresh(&dir)?;
        Ok((index, dir))
    }
}

impl Prepared {
    pub fn finish(self) -> wire::Answer {
        match self {
            Self::Held(answer) => answer,
            Self::GitStatus(ask) => ask.read(),
            Self::Commits(ask) => ask.read(),
            Self::Prefix(ask) => ask.read(),
            Self::PromptShape(ask) => ask.read(),
            Self::City(ask) => ask.read(),
            Self::History {
                ledger,
                before,
                limit,
            } => wire::Answer::History(Box::new(ledger.history(before, limit))),
            Self::HistoryRange {
                ledger,
                from,
                to,
                limit,
            } => wire::Answer::HistoryRange(Box::new(ledger.history_range(from, to, limit))),
            Self::RunHistory {
                ledger,
                run,
                before,
                limit,
            } => wire::Answer::History(Box::new(ledger.run_history(run, before, limit))),
            Self::Rounds { ledger, run } => match ledger.rounds_answer(run) {
                Ok(rounds) => wire::Answer::Rounds(Box::new(rounds)),
                Err(_unreadable_freeze) => unavailable(format!("Rounds({run})")),
            },
            Self::Evidence { ledger, run } => wire::Answer::Evidence(ledger.evidence_answer(run)),

            Self::Recalled { ledger, run } => match ledger.recalled(run) {
                Ok(summary) => wire::Answer::Run(Some(Box::new(summary))),
                Err(stopped) => unavailable_because(format!("RunView({run})"), &stopped),
            },
            Self::Preferences => Self::preferences_answer(),
            Self::Identity { city_root } => super::answering::identity::identity_answer(&city_root),
            Self::Automation { city_root } => {
                super::answering::automation::automation_answer(&city_root)
            }
            Self::GithubLogin { ask, host } => {
                super::answering::github::github_answer(ask, host.as_deref())
            }
            Self::Guide(city_root) => Self::guide_answer(city_root),
            Self::Provider(ask) => ask.answer(),
            Self::Release(newest) => Self::release_answer(newest),
            Self::Agents(asked) => asked.answer(),
            Self::Refused(refusal) => unavailable_because(refusal.action().to_owned(), &refusal),
            Self::Upstream { ask, item } => Self::upstream_answer(ask, item),
            Self::Listing { city_root, at } => {
                wire::Answer::Listing(listing_answer(&city_root, at))
            }
            Self::Find(root, under, text) => wire::Answer::Find(find_answer(&root, under, text)),

            Self::Document { city_root, at } => {
                wire::Answer::Document(Box::new(document_answer(&city_root, at)))
            }
            Self::Proposals { ledger, doc, open } => {
                super::proposals::proposals_answer(&ledger, doc, open)
            }
            Self::Range {
                city_root,
                version,
                range,
            } => super::answering::range::range_answer(&city_root, version, range),
            Self::Preview {
                city_root,
                version,
                viewport,
            } => super::answering::preview::preview_answer(&city_root, version, viewport),
            Self::Reply { text, state } => super::answering::reply::reply_answer(&text, state),
            Self::Stored(ask) => ask.answer(),
            Self::Versions(ask) => ask.answer(),
            Self::Layers { city_root, addr } => super::layers::layers_answer(&city_root, addr),
            Self::Catalog(ask) => ask.answer(),
            Self::Building {
                city_root,
                addr,
                plans,
            } => match plans_of(&plans, &city_root, BTreeSet::from([addr.clone()]))
                .remove(&addr)
                .and_then(|plan| read_building(&city_root, &addr, plan))
            {
                Some(answer) => wire::Answer::Building(Box::new(answer)),

                None => unavailable(format!("BuildingView({})", addr.as_str())),
            },

            Self::Changes {
                city_root,
                base,
                head,
            } => {
                let far = head.map_or(storage::Head::WorkingTree, storage::Head::Commit);
                match storage::between(&city_root, base, far) {
                    Ok(files) => wire::Answer::Changes(wire::ChangesAnswer { base, head, files }),
                    Err(_) => unavailable(format!("Changes({base})")),
                }
            }
            Self::Hunks {
                city_root,
                oid_a,
                oid_b,
                path,
            } => hunks_answer(&city_root, oid_a, oid_b, &path),
            Self::Content { city_root, locator } => match content_answer(&city_root, &locator) {
                Some(answer) => wire::Answer::Content(Box::new(answer)),
                None => unavailable(format!("Content({locator})")),
            },
            Self::Archives { city_root, needle } => search_archives(&city_root, &needle),
            Self::Skills {
                city_root,
                building,
                pins,
            } => match skills_answer(&city_root, &building, &pins) {
                Some(answer) => wire::Answer::Skills(Box::new(answer)),
                None => unavailable(format!("Skills({})", building.as_str())),
            },
            Self::McpHealth { live, addr } => {
                wire::Answer::McpHealth(Box::new(live.mcp_health_answer(&addr)))
            }
            Self::Toolkits(live) => wire::Answer::Toolkits(Box::new(live.toolkits_answer())),
            Self::Usage(ask) => ask.answer(),

            Self::Metrics { city_root, held } => match buildings_of(&city_root) {
                Ok(buildings) => wire::Answer::Metrics(Box::new(wire::MetricsAnswer {
                    buildings: u64::try_from(buildings.len()).unwrap_or(u64::MAX),
                    ..held
                })),
                Err(_unreadable_root) => unavailable("Metrics".to_owned()),
            },
        }
    }
}
