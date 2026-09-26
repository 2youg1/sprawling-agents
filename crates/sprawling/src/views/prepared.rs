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
//! one how the read is done while it does not.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use kernel::{Address, GitOid, Locator, RunId, Seq};

use super::archives::search_archives;
use super::building_page::read_building;
use super::city::CityAsk;
use super::document::document_answer;
use super::git_status::GitStatusAsk;
use super::hunks::hunks_answer;
use super::lines::buildings_of;
use super::lines::config_answer;
use super::listing::listing_answer;
use super::prefix::{PrefixAsk, content_answer};
use super::skills::{SkillPins, skills_answer};
use accounting::plan_view::{PlanView, plans_of};

/// The answer to a question this city could not look up.
///
/// One shape, named once: a reader that met an empty city and a reader
/// that met a city which could not look have to be able to tell the
/// difference, and every caller spelling the refusal itself is how the
/// two start looking alike.
pub(super) fn unavailable(query: String) -> channels::Answer {
    channels::Answer::Unavailable { query }
}

/// What a read of now - a tool server's handshake, the broker's shelf -
/// needs from the views, copied out so the read runs with the snapshot
/// let go.
pub(crate) struct LiveAsk {
    pub(super) city_root: PathBuf,
    pub(super) city: Option<Address>,
    pub(super) vault: Option<Arc<Mutex<gateway::Custodian>>>,
}

/// A query's answer split at the snapshot: what the views settled while
/// it was held, or the small data a read of the disk, git or network
/// needs, copied out so that read runs with the snapshot let go.
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
    Release(Option<fn() -> channels::ReleaseAnswer>),
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
    /// The vital signs: every figure the fold holds, and the building
    /// count, which only the directory can give, still to read.
    Metrics {
        city_root: PathBuf,
        held: channels::MetricsAnswer,
    },
    /// One level of the tree.
    Listing {
        city_root: PathBuf,
        at: Option<Address>,
    },
    /// The head of one file.
    Document { city_root: PathBuf, at: Address },
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
/// readers wait on (sprawling-SPEC.md 8-92).
pub(crate) struct LedgerAsk {
    pub(super) city_root: PathBuf,
    pub(super) index: Arc<Mutex<memory::LedgerIndex>>,
}

impl Prepared {
    /// Does the read the views left for after the snapshot, and answers.
    pub(crate) fn finish(self) -> channels::Answer {
        match self {
            Self::Held(answer) => answer,
            Self::GitStatus(ask) => ask.read(),
            Self::Prefix(ask) => ask.read(),
            Self::City(ask) => ask.read(),
            Self::History {
                ledger,
                before,
                limit,
            } => channels::Answer::History(Box::new(ledger.history(before, limit))),
            Self::HistoryRange {
                ledger,
                from,
                to,
                limit,
            } => channels::Answer::HistoryRange(Box::new(ledger.history_range(from, to, limit))),
            Self::RunHistory {
                ledger,
                run,
                before,
                limit,
            } => channels::Answer::History(Box::new(ledger.run_history(run, before, limit))),
            Self::Rounds { ledger, run } => {
                channels::Answer::Rounds(Box::new(ledger.rounds_answer(run)))
            }
            Self::Evidence { ledger, run } => {
                channels::Answer::Evidence(ledger.evidence_answer(run))
            }
            // An evicted run always has records in the Ledger, so a
            // recall that cannot read them is "I could not look".
            Self::Recalled { ledger, run } => match ledger.recalled(run) {
                Some(summary) => channels::Answer::Run(Some(summary)),
                None => unavailable(format!("RunView({run})")),
            },
            // A settings file that cannot be read is "I could not
            // look", not an empty set of preferences.
            Self::Preferences => match accounting::person::read() {
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
            // Leaves this machine, and only on a press (channels-SPEC 8-36),
            // through the registry a served city handed the views.
            Self::Release(Some(newest)) => channels::Answer::Release(Box::new(newest())),
            Self::Release(None) => unavailable("NewestRelease".to_owned()),
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
            Self::Building {
                city_root,
                addr,
                plans,
            } => match plans_of(&plans, &city_root, BTreeSet::from([addr.clone()]))
                .remove(&addr)
                .and_then(|plan| read_building(&city_root, &addr, plan))
            {
                Some(answer) => channels::Answer::Building(Box::new(answer)),
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
                let far = head.map_or(memory::Head::WorkingTree, memory::Head::Commit);
                match memory::between(&city_root, base, far) {
                    Ok(files) => {
                        channels::Answer::Changes(channels::ChangesAnswer { base, head, files })
                    }
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
                Some(answer) => channels::Answer::Content(Box::new(answer)),
                None => unavailable(format!("Content({locator})")),
            },
            Self::Archives { city_root, needle } => {
                channels::Answer::Archive(search_archives(&city_root, &needle))
            }
            Self::Skills {
                city_root,
                building,
                pins,
            } => match skills_answer(&city_root, &building, &pins) {
                Some(answer) => channels::Answer::Skills(Box::new(answer)),
                None => unavailable(format!("Skills({})", building.as_str())),
            },
            // A count that cannot be expressed is reported as the largest
            // count this wire can carry, for the reason every figure of
            // `Views::metrics` is.
            Self::McpHealth { live, addr } => {
                channels::Answer::McpHealth(Box::new(live.mcp_health_answer(&addr)))
            }
            Self::Toolkits(live) => channels::Answer::Toolkits(Box::new(live.toolkits_answer())),
            Self::Metrics { city_root, held } => {
                channels::Answer::Metrics(Box::new(channels::MetricsAnswer {
                    buildings: u64::try_from(buildings_of(&city_root).len()).unwrap_or(u64::MAX),
                    ..held
                }))
            }
        }
    }
}
