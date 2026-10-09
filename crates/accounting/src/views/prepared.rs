// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The second half of a query that reads the disk, git or the network:
//! what `Views::prepare` copied out of a snapshot of the views, and the
//! read [`Prepared::finish`] does once the snapshot is let go.
//!
//! Apart from `answering` because the two change for different reasons:
//! that module decides what a query takes while the fold waits, and this
//! one how the read is done while it does not. The reads whose answer is
//! not this city's own record - the person's settings file, the
//! first-run guide, the registry, the upstream check and the search
//! path - are answered in `leaving`, beside the
//! refusal each of them decides; this module keeps the one list of reads
//! and routes each of them to the module that does it.

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
use super::prefix::{PrefixAsk, content_answer};
use super::skills::{SkillPins, skills_answer};
use crate::plan_view::{PlanView, plans_of};

/// The answer to a question this city could not look up.
///
/// One shape, named once: a reader that met an empty city and a reader
/// that met a city which could not look have to be able to tell the
/// difference, and every caller spelling the refusal itself is how the
/// two start looking alike.
pub(super) fn unavailable(query: String) -> wire::Answer {
    wire::Answer::Unavailable {
        query,
        reason: None,
    }
}

/// The answer to a question this city tried to look up and could not,
/// with what stopped it (`crates/wire/spec/Server.lean` D47).
pub(super) fn unavailable_because(query: String, stopped: &kernel::AxError) -> wire::Answer {
    wire::Answer::Unavailable {
        query,
        reason: Some(stopped.to_string()),
    }
}

/// A query's answer split at the snapshot: what the views settled while
/// it was held, or the small data a read of the disk, git or network
/// needs, copied out so that read runs with the snapshot let go.
pub enum Prepared {
    /// Answered from the views alone.
    Held(wire::Answer),
    /// The working tree of one building against its last checkpoint.
    GitStatus(GitStatusAsk),
    /// One commit, or a page of them, still to be given their parents.
    Commits(CommitsAsk),
    /// The person's own settings file.
    Preferences,
    /// The two identity areas, read from disk.
    Identity { city_root: PathBuf },
    /// The schedule and the watch table, read from disk.
    Automation { city_root: PathBuf },
    /// The GitHub CLI asked for one host's login.
    GithubLogin {
        ask: Option<fn(&str) -> wire::GithubReading>,
        host: Option<String>,
    },
    /// The first-run guide's progress of the city at this root.
    Guide(PathBuf),
    /// The endpoint book or the configuration ladder, each with the keys
    /// its accounts name still to read from the vault.
    Provider(super::providers::ProviderAsk),
    /// The release page, which leaves this machine.
    Release(Option<fn() -> wire::ReleaseAnswer>),
    /// The harness page, which walks this machine's search path.
    Harnesses(Option<super::lines::HarnessReach>),
    /// The ACP page's catalog, read from the city's `CONFIG.toml`, with
    /// this machine's evidence when the served city handed its paths in.
    Agents(PathBuf, Option<super::lines::HarnessReach>),
    /// A question the views refused outright, which [`Prepared::answer`]
    /// carries as the refusal.
    Refused(kernel::AxError),
    /// One item's publisher, which leaves this machine too.
    Upstream {
        ask: Option<fn(&str) -> wire::DoctorUpstream>,
        item: String,
    },
    /// What changed between a checkpoint and a later one, or the
    /// working tree.
    Changes {
        city_root: PathBuf,
        base: GitOid,
        head: Option<GitOid>,
    },
    /// The patch text of one file between two checkpoints.
    Hunks {
        city_root: PathBuf,
        oid_a: GitOid,
        oid_b: GitOid,
        path: String,
    },
    /// One object of the store.
    Content {
        city_root: PathBuf,
        locator: Locator,
    },
    /// Every building's archive shelves, searched for one needle.
    Archives { city_root: PathBuf, needle: String },
    /// One building's shelves, beside which runs pinned each holding.
    Skills {
        city_root: PathBuf,
        building: Address,
        pins: SkillPins,
    },
    /// Where each tool server one address reaches stands: the
    /// configuration read and every handshake run after the snapshot
    /// is let go, because a handshake waits up to its patience.
    McpHealth { live: LiveAsk, addr: Address },
    /// The broker's shelf, read after the snapshot is let go.
    Toolkits(LiveAsk),
    /// The skill and tool server usage, folded from the whole ledger.
    Usage(super::usage::UsageAsk),
    /// The vital signs: every figure the fold holds, and the building
    /// count, which only the directory can give, still to read.
    Metrics {
        city_root: PathBuf,
        held: wire::MetricsAnswer,
    },
    /// One level of the tree.
    Listing {
        city_root: PathBuf,
        at: Option<Address>,
    },
    /// The files under one address whose name holds a text: city root, address, text.
    Find(PathBuf, Address, String),
    /// One file of the tree, as a version.
    Document { city_root: PathBuf, at: Address },
    /// One document's open proposal cards, with its version still to
    /// read.
    Proposals {
        ledger: LedgerAsk,
        doc: Address,
        open: Vec<documents::Offer>,
    },
    /// One window of a stored document version.
    Range {
        city_root: PathBuf,
        version: B3Hash,
        range: documents::Span,
    },
    /// One window of a stored Markdown version, laid out.
    Preview {
        city_root: PathBuf,
        version: B3Hash,
        viewport: documents::Span,
    },
    /// One stored object's bytes, or a stored version exported.
    Stored(super::answering::stored::StoredAsk),
    /// One document's versions: its saves, with the disk still to read.
    Versions(super::versions::VersionsAsk),
    /// A reply's text the page sent, still to be laid out.
    Reply {
        text: String,
        state: documents::ReplyState,
    },
    /// Every building's progress and every pursuit's verdict, with the
    /// buildings still to list and their plans still to read.
    City(CityAsk),
    /// One building's directory and its plan.
    Building {
        city_root: PathBuf,
        addr: Address,
        plans: Arc<Mutex<PlanView>>,
    },
    /// The prompt one run was frozen with: a ledger line and the store.
    Prefix(PrefixAsk),
    /// A bounded slice of the history ending before a cursor.
    History {
        ledger: LedgerAsk,
        before: Option<Seq>,
        limit: u32,
    },
    /// One named range of the history.
    HistoryRange {
        ledger: LedgerAsk,
        from: Seq,
        to: Seq,
        limit: u32,
    },
    /// One run's own records ending before a cursor.
    RunHistory {
        ledger: LedgerAsk,
        run: RunId,
        before: Option<Seq>,
        limit: u32,
    },
    /// One run's records, folded into the rounds a person reads.
    Rounds { ledger: LedgerAsk, run: RunId },
    /// Every locator one run left behind.
    Evidence { ledger: LedgerAsk, run: RunId },
    /// The summary of a run the hot view evicted, folded from its own
    /// records in the Ledger.
    Recalled { ledger: LedgerAsk, run: RunId },
}

/// The ledger as a history reader carries it out of the snapshot: where
/// it lives, and its index, which has a lock of its own that only
/// readers wait on (`crates/sprawling/Spec.lean` §8-100).
pub struct LedgerAsk {
    pub(super) city_root: PathBuf,
    pub(super) index: Arc<Mutex<storage::LedgerIndex>>,
}

impl LedgerAsk {
    /// The ledger's directory, and its index brought up to the segments
    /// on disk and held for one read, or the refresh's failure.
    ///
    /// A poisoned lock means a refresh was cut short and may have left an
    /// offset pointing at another line, so the index is replaced by an
    /// empty one and the poison cleared: the refresh that follows scans
    /// the whole ledger once, and later reads refresh incrementally again
    /// (`crates/sprawling/Spec.lean` §8-100).
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
    /// Does the read the views left for after the snapshot, and answers.
    pub fn finish(self) -> wire::Answer {
        match self {
            Self::Held(answer) => answer,
            Self::GitStatus(ask) => ask.read(),
            Self::Commits(ask) => ask.read(),
            Self::Prefix(ask) => ask.read(),
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
            // An evicted run always has records in the Ledger, so a
            // recall that cannot read them is "I could not look".
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
            Self::Harnesses(reach) => Self::harnesses_answer(reach),
            Self::Agents(city_root, reach) => super::agents::catalog_answer(&city_root, reach),
            Self::Refused(refusal) => unavailable_because(refusal.action().to_owned(), &refusal),
            Self::Upstream { ask, item } => Self::upstream_answer(ask, item),
            Self::Listing { city_root, at } => {
                wire::Answer::Listing(listing_answer(&city_root, at))
            }
            Self::Find(root, under, text) => wire::Answer::Find(find_answer(&root, under, text)),
            // Missing, unreadable and empty are answers of their own
            // (`crates/wire/Spec.lean` §8-69), so this read always answers a document.
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
            Self::Building {
                city_root,
                addr,
                plans,
            } => match plans_of(&plans, &city_root, BTreeSet::from([addr.clone()]))
                .remove(&addr)
                .and_then(|plan| read_building(&city_root, &addr, plan))
            {
                Some(answer) => wire::Answer::Building(Box::new(answer)),
                // A building nobody raised is not an empty building. The
                // page needs to be able to tell those apart.
                None => unavailable(format!("BuildingView({})", addr.as_str())),
            },
            // A checkpoint this city does not hold is `Unavailable`, not
            // an empty change list: "nothing moved" and "I could not
            // look" are different answers and a reader acts differently
            // on each.
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
            // A count that cannot be expressed is reported as the largest
            // count this wire can carry, for the reason every figure of
            // `Views::metrics` is.
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
