// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! `RULES.toml` evaluated into rules a machine can hold.
//!
//! A confidential building means three things at once, and each of them
//! is held somewhere that cannot be talked out of it: the model pool is
//! local, the write domain stops at the building's own subtree, and data
//! does not leave. This module decides the first two from the file; the
//! third is the egress door's.
//!
//! A building with no `RULES.toml` is an ordinary building. One that
//! exists and does not say whether it is confidential is an error:
//! defaulting a privacy decision quietly is the failure this whole
//! surface exists to prevent.
//!
//! **One document, read by the city and by every resident.** The
//! prefix carries this file's own bytes, so what an agent reads is what
//! the city enforces. It was Markdown until the reader had to find keys
//! in prose and matched them on any line: a sentence that began
//! `desktop: true` granted this machine's desktop, and a `write:` line
//! nobody wrote resolved to `Everything`. Both failed towards the
//! permissive side. Splitting the prose into a second document would
//! have fixed that and bought a worse thing — two descriptions of one
//! building, free to disagree.

use std::path::{Path, PathBuf};

use kernel::layout::CityLayout;
use kernel::{Address, AxCode, AxError, BuildingPolicy, EgressAllowlist, WriteDomain};

mod desktop;
mod evaluate;
mod reach;

pub use desktop::{DESKTOP_SCOPE_FILE, desktop_scope_path, write_desktop_scope};
pub use evaluate::evaluate;
pub use reach::DomainReach;

/// What a building is and what it may do: the one file this module
/// parses, and the one a resident is given.
pub const RULES_FILE: &str = "RULES.toml";

/// The name this version no longer writes or reads. It exists for the
/// single refusal that tells a person where their rules went; nothing
/// opens a file at this name.
const SUPERSEDED_FILE: &str = "BUILDING.md";

mod user_browser;

pub use user_browser::{UserBrowser, UserBrowserEndpoint};

/// Which models a run in this building may reach. Exhaustive rather than
/// a bool, because "any" and "local only" are two policies and a third
/// (a named pool) is a change to this enum, not a new flag beside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelPool {
    Any,
    LocalOnly,
}

/// A building's rules, as evaluated from its file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildingRules {
    addr: Address,
    policy: BuildingPolicy,
    write_prefixes: Vec<Address>,
    reach: DomainReach,
    egress: EgressAllowlist,
    review: bool,
    /// Whether residents here are given the browser tool.
    browser: bool,
    /// Whether residents here are given the tool that drives the
    /// browser the person is already using, and where that browser
    /// answers.
    usersbrowser: Option<UserBrowser>,
    /// Whether residents here are given this machine's own desktop.
    desktop: bool,
    /// The skills this building takes into its catalog, by name. The
    /// city's shelves may hold a thousand; what costs resident bytes is
    /// this list, and a person writes it.
    reading_room: Vec<String>,
}

impl BuildingRules {
    /// The policy value that rides along on every model call.
    #[must_use]
    pub fn policy(&self) -> &BuildingPolicy {
        &self.policy
    }

    #[must_use]
    pub fn addr(&self) -> &Address {
        &self.addr
    }

    /// The domains this building may reach.
    ///
    /// A confidential building has none, whatever its file says: "data
    /// enters and does not leave" is the meaning of the setting, so a
    /// declared list under it would be a contradiction the reader would
    /// have to resolve. The list it declared is refused at evaluation.
    #[must_use]
    pub fn egress(&self) -> &EgressAllowlist {
        &self.egress
    }

    /// Whether work done here has to be checked by someone else before
    /// it reaches the building.
    ///
    /// A building that says yes gives each run its own tree, and nothing
    /// a run writes is visible until another resident merges it. Absent
    /// the line, the answer is no: a person who dispatches one agent to
    /// one room and watches it work should see the file change, and
    /// requiring a second agent for that would be a discipline nobody
    /// asked for.
    #[must_use]
    pub fn review(&self) -> bool {
        self.review
    }

    /// Whether a resident of this building may drive a browser.
    ///
    /// Absent the line, no. That is the opposite reading from
    /// `confidential`, and the difference is which way each one fails:
    /// a privacy setting read as the permissive side is an accident,
    /// while a tool nobody was given is one tool fewer. A confidential
    /// building never gets one, because a browser that opens any URL is
    /// an egress path and "data does not leave" is the whole of what
    /// that setting means.
    #[must_use]
    pub fn browser(&self) -> bool {
        self.browser
    }

    /// What this building says about driving the browser a person is
    /// already using, and where that browser answers.
    ///
    /// Absent the line, nothing: the same reading `browser` gets, and
    /// for a stronger reason — a browser this city started holds one
    /// building's logins, while the person's own browser holds every
    /// account that person has. A confidential building never gets the
    /// tool, because the attachment is exactly what its per-building
    /// isolation does not survive.
    #[must_use]
    pub fn usersbrowser(&self) -> Option<&UserBrowser> {
        self.usersbrowser.as_ref()
    }

    /// Whether a resident of this building may reach this machine's own
    /// desktop, through the connector `sprawling-desktop` offers.
    ///
    /// Absent the line, no — the same reading `browser` gets, and for a
    /// stronger reason: what `desktop.act` presses on somebody's own
    /// machine carries no way back, so a building that was never asked
    /// about it has not agreed to it. A confidential building never gets
    /// one, because a desktop holds other programs, other windows and
    /// one shared clipboard, none of which are this building's.
    ///
    /// **This decides whether the connector may be attached at all, not
    /// what it may then touch.** That second question is the
    /// `DESKTOP.toml` allowlist's, window by window, and the two doors
    /// are deliberately not merged: one asks whether this building does
    /// this kind of work, the other asks which windows on this machine —
    /// and one door asking both would leave one of the questions unasked.
    #[must_use]
    pub fn desktop(&self) -> bool {
        self.desktop
    }

    /// The names under `## Reading room`, in the order the file lists
    /// them. Absent section means an empty reading room: a building
    /// starts with the tools it is given and nothing else, because the
    /// alternative is every building paying for every skill the city has
    /// ever settled.
    #[must_use]
    pub fn reading_room(&self) -> &[String] {
        &self.reading_room
    }

    /// What this building's residents may write inside their prefixes.
    #[must_use]
    pub fn reach(&self) -> DomainReach {
        self.reach
    }

    #[must_use]
    pub fn model_pool(&self) -> ModelPool {
        if self.policy.confidential {
            ModelPool::LocalOnly
        } else {
            ModelPool::Any
        }
    }

    /// The write domain a run in this building gets.
    ///
    /// # Errors
    /// Refuses a confidential building that declares a prefix outside
    /// itself. Trimming it silently would leave the file saying one thing
    /// and the city doing another; refusing says which line to change.
    pub fn write_domain(&self) -> Result<WriteDomain, AxError> {
        let mut prefixes = Vec::new();
        for prefix in &self.write_prefixes {
            if self.policy.confidential && !prefix.is_within(&self.addr) {
                return Err(AxError::failure(
                    AxCode::GateDenied,
                    "build the write domain of a confidential building",
                    format!("{} reaches outside {}", prefix.as_str(), self.addr.as_str()),
                )
                .with_recovery(
                    "remove that prefix, or drop `confidential = true` and say why in the file",
                ));
            }
            prefixes.push(prefix.clone());
        }
        if prefixes.is_empty() {
            prefixes.push(self.addr.clone());
        }
        match self.reach {
            DomainReach::Everything => WriteDomain::new(prefixes),
            DomainReach::Documents => WriteDomain::documents(prefixes),
        }
    }
}

/// Where a building's rules live: in the building's own reserved
/// subtree, which no write domain reaches.
///
/// A person writes this file and the runs it governs only read it. That
/// was the first line of the file and nothing held it until the path
/// moved (ARCHITECTURE.md section 6).
#[must_use]
pub fn rules_path(city_root: &Path, addr: &Address) -> PathBuf {
    governed(city_root, addr).join(RULES_FILE)
}

/// Where a project keeps the conventions it came with.
///
/// The building's own root rather than its reserved subtree, and that
/// placement is the point: the file belongs to the project, a person
/// edits it, and the tools a resident already has can reach it. The
/// reserved subtree is where the *city* keeps what it owns, and this is
/// not that.
#[must_use]
pub fn agents_path(city_root: &Path, addr: &Address) -> PathBuf {
    CityLayout::new(city_root)
        .scope(addr)
        .join(crate::spine_files::AGENTS_FILE)
}

/// more, if one is on this disk.
///
/// Two of them can be: the rules were Markdown before they were TOML,
/// and before that they sat at the building's root rather than under
/// its reserved subtree. One question rather than one guard per move,
/// because every answer has the same consequence — a confidential
/// building loading as an ordinary one.
fn superseded(city_root: &Path, addr: &Address) -> Option<PathBuf> {
    let layout = CityLayout::new(city_root);
    [
        governed(city_root, addr).join(SUPERSEDED_FILE),
        layout.scope(addr).join(SUPERSEDED_FILE),
    ]
    .into_iter()
    .find(|path| path.is_file())
}

/// A scope's own reserved subtree: where what governs that scope sits,
/// out of reach of every write domain.
pub(super) fn governed(city_root: &Path, addr: &Address) -> PathBuf {
    CityLayout::new(city_root)
        .scope(addr)
        .join(kernel::RESERVED_PREFIX)
}

/// Loads and evaluates a building's rules.
///
/// # Errors
/// Propagates an unreadable file, and refuses one that does not state
/// whether the building is confidential.
pub fn load(city_root: &Path, addr: &Address) -> Result<BuildingRules, AxError> {
    let path = rules_path(city_root, addr);
    match std::fs::read_to_string(&path) {
        Ok(text) => evaluate(addr, &text),
        // An absent file is an ordinary building - unless a document
        // this version no longer reads is holding the rules, in which
        // case reading "absent" would turn a confidential building into
        // an ordinary one without anybody being told.
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            match superseded(city_root, addr) {
                Some(stale) => Err(AxError::failure(
                    AxCode::ConfigInvalid,
                    "read a building's rules",
                    addr.as_str().to_owned(),
                )
                .with_recovery(format!(
                    "move the settings in {} into {}, leaving the prose where it is; this \
                     version reads a building's rules as TOML in its reserved subtree, and \
                     rules it cannot find would load a confidential building as an ordinary \
                     one",
                    stale.display(),
                    path.display(),
                ))),
                None => Ok(BuildingRules {
                    addr: addr.clone(),
                    policy: BuildingPolicy::default(),
                    write_prefixes: Vec::new(),
                    reach: DomainReach::Everything,
                    egress: EgressAllowlist::default(),
                    review: false,
                    browser: false,
                    usersbrowser: None,
                    desktop: false,
                    reading_room: Vec::new(),
                }),
            }
        }
        Err(err) => Err(AxError::failure(
            AxCode::StorageFatal,
            "read a building's rules",
            format!("{}: {err}", path.display()),
        )
        .with_recovery("fix the file's permissions; a building's rules are not optional")),
    }
}

/// Replaces a building's rules with a proposal that evaluates.
///
/// The evaluation comes first and the write only follows it, so the file
/// on disk is always one this build can read: a governance document that
/// stopped parsing halfway through a rewrite would take its building
/// with it. What comes back is what the next run will be governed by,
/// read from the proposal rather than from the file, because those two
/// are the same bytes and only one of them can be returned.
///
/// The path is inside the scope's reserved subtree, which no write
/// domain reaches - which is exactly why this is a door of its own and
/// not an `edit`.
///
/// # Errors
/// Propagates the evaluation's refusal, and a directory or file that
/// cannot be written.
pub fn write_rules(city_root: &Path, addr: &Address, text: &str) -> Result<BuildingRules, AxError> {
    let rules = evaluate(addr, text)?;
    crate::document::replace(&rules_path(city_root, addr), text.as_bytes())?;
    Ok(rules)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
