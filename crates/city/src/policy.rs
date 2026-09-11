// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! `BUILDING.md` evaluated into rules a machine can hold.
//!
//! A confidential building means three things at once, and each of them
//! is held somewhere that cannot be talked out of it: the model pool is
//! local, the write domain stops at the building's own subtree, and data
//! does not leave. This module decides the first two from the file; the
//! third is the egress door's.
//!
//! A building with no `BUILDING.md` is an ordinary building. A
//! `BUILDING.md` that exists and does not say whether it is confidential
//! is an error: defaulting a privacy decision quietly is the failure this
//! whole surface exists to prevent.

use std::path::{Path, PathBuf};

use kernel::{Address, AxCode, AxError, BuildingPolicy, EgressAllowlist, WriteDomain};

mod evaluate;
mod reach;

pub use evaluate::evaluate;
pub use reach::DomainReach;

/// The file a building's rules live in, at the building root.
pub const BUILDING_FILE: &str = "BUILDING.md";

const CONFIDENTIAL_KEY: &str = "confidential:";
const WRITE_KEY: &str = "write:";
const REVIEW_KEY: &str = "review:";
const BROWSER_KEY: &str = "browser:";
const DESKTOP_KEY: &str = "desktop:";
const WRITE_HEADING: &str = "write domain";
const EGRESS_HEADING: &str = "egress";
const READING_HEADING: &str = "reading room";

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
                    "remove that prefix, or drop `confidential: true` and say why in the file",
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
pub fn building_path(city_root: &Path, addr: &Address) -> PathBuf {
    scope_path(city_root, addr)
        .join(kernel::RESERVED_PREFIX)
        .join(BUILDING_FILE)
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
    scope_path(city_root, addr).join(crate::spine_files::AGENTS_FILE)
}

/// The allowlist the desktop connector is started under, window by
/// window. Its syntax belongs to the server that reads it
/// (`desktop/src/scope.rs`); this side only decides where it lives.
pub const DESKTOP_SCOPE_FILE: &str = "DESKTOP.toml";

/// Where a building's desktop allowlist lives: beside its rules, in the
/// building's own reserved subtree.
///
/// It is a governing document rather than a product. It states, window
/// by window, what this building's runs may touch on somebody's own
/// machine — so by the reading `DomainReach` rests on, the
/// file that decides what residents may do is never a file residents
/// write. `is_reserved` is true for any address with `.sprawling` in it,
/// so no write domain reaches this path, including `Everything`'s.
#[must_use]
pub fn desktop_scope_path(city_root: &Path, addr: &Address) -> PathBuf {
    scope_path(city_root, addr)
        .join(kernel::RESERVED_PREFIX)
        .join(DESKTOP_SCOPE_FILE)
}

/// Replaces a building's desktop allowlist with what a person wrote.
///
/// Whole rather than patched, for the reason `city::governed` gives: a
/// person edits this in one box and saves it once, and a partial write
/// would leave the connector scoped by half a line.
///
/// **Nothing here parses it.** The authority on this file's syntax is
/// the server that reads it at startup, and that server fails closed —
/// a file it cannot read permits nothing. A second parser on this side
/// would be a second authority, and one of two authorities eventually
/// reads a file as meaning something the other does not.
///
/// # Errors
/// Propagates a reserved subtree that cannot be created or written.
pub fn write_desktop_scope(
    city_root: &Path,
    addr: &Address,
    text: &str,
) -> Result<PathBuf, AxError> {
    let path = desktop_scope_path(city_root, addr);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|err| {
            AxError::failure(
                AxCode::StorageFatal,
                "write a building's desktop allowlist",
                format!("{}: {err}", dir.display()),
            )
            .with_recovery("fix the directory's permissions")
        })?;
    }
    std::fs::write(&path, text).map_err(|err| {
        AxError::failure(
            AxCode::StorageFatal,
            "write a building's desktop allowlist",
            format!("{}: {err}", path.display()),
        )
        .with_recovery("fix the file's permissions")
    })?;
    Ok(path)
}

/// Where a building's rules used to live, for the one refusal that says
/// so. Nothing reads the file at this path.
fn legacy_building_path(city_root: &Path, addr: &Address) -> PathBuf {
    scope_path(city_root, addr).join(BUILDING_FILE)
}

fn scope_path(city_root: &Path, addr: &Address) -> PathBuf {
    let mut path = city_root.to_path_buf();
    for segment in addr.as_str().split('/') {
        path.push(segment);
    }
    path
}

/// Loads and evaluates a building's rules.
///
/// # Errors
/// Propagates an unreadable file, and refuses one that does not state
/// whether the building is confidential.
pub fn load(city_root: &Path, addr: &Address) -> Result<BuildingRules, AxError> {
    let path = building_path(city_root, addr);
    match std::fs::read_to_string(&path) {
        Ok(text) => evaluate(addr, &text),
        // An absent file is an ordinary building - unless the rules are
        // sitting at the address this layout moved away from, in which
        // case reading "absent" would turn a confidential building into
        // an ordinary one without anybody being told.
        Err(err)
            if err.kind() == std::io::ErrorKind::NotFound
                && legacy_building_path(city_root, addr).is_file() =>
        {
            Err(AxError::failure(
                AxCode::ConfigInvalid,
                "read a building's rules",
                addr.as_str().to_owned(),
            )
            .with_recovery(format!(
                "move {}/{BUILDING_FILE} to {}/{}/{BUILDING_FILE}; a building's rules live where \
                 its own runs cannot write them",
                addr.as_str(),
                addr.as_str(),
                kernel::RESERVED_PREFIX,
            )))
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(BuildingRules {
            addr: addr.clone(),
            policy: BuildingPolicy::default(),
            write_prefixes: Vec::new(),
            reach: DomainReach::Everything,
            egress: EgressAllowlist::default(),
            review: false,
            browser: false,
            desktop: false,
            reading_room: Vec::new(),
        }),
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
    let path = building_path(city_root, addr);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|err| {
            AxError::failure(
                AxCode::StorageFatal,
                "write a building's rules",
                format!("{}: {err}", dir.display()),
            )
            .with_recovery("fix the directory's permissions")
        })?;
    }
    std::fs::write(&path, text).map_err(|err| {
        AxError::failure(
            AxCode::StorageFatal,
            "write a building's rules",
            format!("{}: {err}", path.display()),
        )
        .with_recovery("fix the file's permissions")
    })?;
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
