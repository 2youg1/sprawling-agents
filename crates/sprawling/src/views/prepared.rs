// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The second half of a query that reads the disk, git or the network:
//! what `Views::prepare` copied out under the view lock, and the read
//! [`Prepared::finish`] does once the lock is released.
//!
//! Apart from `answering` because the two change for different reasons:
//! that module decides what a query takes while the fold waits, and this
//! one how the read is done while it does not.

use std::path::PathBuf;

use kernel::{Address, GitOid, Locator};

use super::archives::search_archives;
use super::document::document_answer;
use super::git_status::GitStatusAsk;
use super::hunks::hunks_answer;
use super::lines::buildings_of;
use super::lines::config_answer;
use super::listing::listing_answer;
use super::prefix::content_answer;
use super::skills::{SkillPins, skills_answer};
use crate::assembly::read_building;
use crate::plan_view::PlanReading;

/// The answer to a question this city could not look up.
///
/// One shape, named once: a reader that met an empty city and a reader
/// that met a city which could not look have to be able to tell the
/// difference, and every caller spelling the refusal itself is how the
/// two start looking alike.
pub(super) fn unavailable(query: String) -> channels::Answer {
    channels::Answer::Unavailable { query }
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
    /// One building's directory, beside the plan the views folded.
    Building {
        city_root: PathBuf,
        addr: Address,
        plan: PlanReading,
    },
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
            Self::Building {
                city_root,
                addr,
                plan,
            } => match read_building(&city_root, &addr, plan) {
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
            Self::Metrics { city_root, held } => {
                channels::Answer::Metrics(Box::new(channels::MetricsAnswer {
                    buildings: u64::try_from(buildings_of(&city_root).len()).unwrap_or(u64::MAX),
                    ..held
                }))
            }
        }
    }
}
