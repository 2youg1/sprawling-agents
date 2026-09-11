// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which building governs an address, and how a new one comes into being.
//!
//! A building is a top-level address. That is not a naming habit: an
//! address decides where a run may write, what it reads by default, and
//! who it reports to, and all three answers are read off the first
//! segment. A building nested inside a building would give those
//! questions a second answer.
//!
//! The bytes a new building starts with are the template a person reads
//! in `docs/templates/BUILDING.md`, not a copy of it kept here. One
//! string, one authority; the confidential template differs from the
//! ordinary one by the single line whose value the city refuses to
//! assume.
//!
//! What a new building starts with beyond its rules — the plan, the
//! memo, the handoff — is `crate::spine_files`'s to lay out.

use std::path::{Path, PathBuf};

use kernel::{Address, AxCode, AxError, Payload};

use crate::policy::BUILDING_FILE;

/// The rules a new building starts with. Instantiated at compile time so
/// that a moved or renamed template breaks the build rather than a city.
const TEMPLATE_RULES: &str = include_str!("../../../docs/templates/BUILDING.md");
/// City Hall's rules, fixed rather than derived: what its two residents
/// may do serves every other building, so it is a property of the city.
const HALL_RULES: &str = include_str!("../../../docs/templates/BUILDING-hall.md");
const NAME_PLACEHOLDER: &str = "<building name>";
const ORDINARY_LINE: &str = "`confidential: false`";
const CONFIDENTIAL_LINE: &str = "`confidential: true`";

/// What a new building is laid out as. Exhaustive: a template exists
/// because some kind of building needs different bytes on its first day,
/// and a kind nobody creates is an authority nobody reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildingTemplate {
    /// An ordinary building: data may leave, any model may answer.
    Minimal,
    /// Data enters and does not leave, the model pool is local, and
    /// writes stop at this building's own subtree.
    Confidential,
    /// City Hall: raised with the city, writing Markdown documents and
    /// never a plan file, holding no project of its own.
    Hall,
}

impl BuildingTemplate {
    /// Reads a template name as it arrived from the control surface.
    ///
    /// # Errors
    /// Refuses a name this version does not lay out, and says which ones
    /// it does: a caller that guessed needs the list, not a verdict.
    pub fn parse(name: &str) -> Result<BuildingTemplate, AxError> {
        match name {
            "minimal" => Ok(BuildingTemplate::Minimal),
            "confidential" => Ok(BuildingTemplate::Confidential),
            "hall" => Ok(BuildingTemplate::Hall),
            other => Err(AxError::failure(
                AxCode::InvalidArgs,
                "read a building template name",
                other.to_owned(),
            )
            .with_recovery("this version lays out `minimal`, `confidential` and `hall`")),
        }
    }

    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            BuildingTemplate::Minimal => "minimal",
            BuildingTemplate::Confidential => "confidential",
            BuildingTemplate::Hall => "hall",
        }
    }

    /// The `BUILDING.md` bytes this template starts a building with.
    ///
    /// # Errors
    /// Refuses when the template no longer carries the line a
    /// confidential building differs by: producing an ordinary building
    /// from the confidential template is the one failure here that
    /// nobody would notice until data left.
    fn rules(self, addr: &Address) -> Result<String, AxError> {
        let named = TEMPLATE_RULES.replace(NAME_PLACEHOLDER, addr.as_str());
        match self {
            BuildingTemplate::Minimal => Ok(named),
            // Fixed bytes, and the address is not substituted into them:
            // City Hall is one address in every city.
            BuildingTemplate::Hall => Ok(HALL_RULES.to_owned()),
            BuildingTemplate::Confidential => {
                if !named.contains(ORDINARY_LINE) {
                    return Err(AxError::failure(
                        AxCode::ConfigInvalid,
                        "lay out a confidential building",
                        format!("the template no longer carries {ORDINARY_LINE}"),
                    )
                    .with_recovery(
                        "restore that line in docs/templates/BUILDING.md; the confidential \
                         template is the ordinary one with that value flipped",
                    ));
                }
                Ok(named.replace(ORDINARY_LINE, CONFIDENTIAL_LINE))
            }
        }
    }
}

/// A building: the top-level address that governs a run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Building {
    addr: Address,
}

impl Building {
    /// Which building governs `addr`.
    ///
    /// # Errors
    /// Refuses the reserved subtree. `.sprawling/` holds the city's own
    /// ledger and configuration; treating it as a building would make
    /// the city's configuration readable as some building's own.
    pub fn of(addr: &Address) -> Result<Building, AxError> {
        let head = addr.as_str().split('/').next().unwrap_or(addr.as_str());
        let head = Address::parse(head)?;
        if head.is_reserved() {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "name the building that governs an address",
                addr.as_str().to_owned(),
            )
            .with_recovery(
                "address a building instead; the reserved subtree is the city's own account \
                 and belongs to no building",
            ));
        }
        Ok(Building { addr: head })
    }

    #[must_use]
    pub fn addr(&self) -> &Address {
        &self.addr
    }

    /// Where this building's own files live.
    #[must_use]
    pub fn root(&self, city_root: &Path) -> PathBuf {
        let mut path = city_root.to_path_buf();
        for segment in self.addr.as_str().split('/') {
            path.push(segment);
        }
        path
    }

    /// Whether `addr` is a room of this building.
    #[must_use]
    pub fn holds(&self, addr: &Address) -> bool {
        addr.is_within(&self.addr)
    }
}

/// The buildings this city has, in address order.
///
/// A building is a top-level directory whose name an address can hold.
/// The reserved subtree is not one, and neither is anything a dot
/// prefixes. Rules are not required here: [`adopt`] exists precisely to
/// draw them over a directory that was already there, so a directory
/// with no `BUILDING.md` is a building nobody has written rules for
/// yet - and a lister that hid it would hide the thing a person is
/// about to adopt.
///
/// # Errors
/// Propagates a city directory that cannot be read.
pub fn all(city_root: &Path) -> Result<Vec<Address>, AxError> {
    let entries = std::fs::read_dir(city_root).map_err(|err| {
        AxError::failure(
            AxCode::StorageFatal,
            "list the buildings of a city",
            format!("{}: {err}", city_root.display()),
        )
        .with_recovery("check the city directory is readable")
    })?;
    let mut out = Vec::new();
    for entry in entries.flatten() {
        if !entry.path().is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        if let Ok(addr) = Address::parse(&name) {
            out.push(addr);
        }
    }
    out.sort_by(|left: &Address, right: &Address| left.as_str().cmp(right.as_str()));
    Ok(out)
}

/// Lays out a new building and returns it.
///
/// # Errors
/// Refuses a room address, the reserved subtree, and a building that
/// already exists. The last one matters most: overwriting would replace
/// the rules a running building works under, and those rules may be the
/// ones that keep its data at home.
pub fn create(
    city_root: &Path,
    addr: &Address,
    template: BuildingTemplate,
) -> Result<Building, AxError> {
    let building = Building::of(addr)?;
    if building.addr() != addr {
        return Err(AxError::failure(
            AxCode::InvalidArgs,
            "create a building",
            addr.as_str().to_owned(),
        )
        .with_recovery(format!(
            "create the building `{}` and dispatch to `{}` inside it; a room is not a building",
            building.addr().as_str(),
            addr.as_str()
        )));
    }
    let root = building.root(city_root);
    // The building's own reserved subtree, made before the file that
    // lives in it: what governs a building is not writable by what runs
    // inside it (kernel-SPEC.md section 8-28).
    let governed = root.join(kernel::RESERVED_PREFIX);
    std::fs::create_dir_all(&governed).map_err(|err| storage(&governed, &err))?;
    let file = governed.join(BUILDING_FILE);
    let rules = template.rules(addr)?;
    // `create_new` rather than exists-then-write: the refusal and the
    // write are one operation, so no second caller lands in between.
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&file)
    {
        Ok(mut handle) => {
            std::io::Write::write_all(&mut handle, rules.as_bytes())
                .map_err(|err| storage(&file, &err))?;
            // After the refusal point, not before it: a building that
            // was refused leaves nothing of itself behind.
            crate::spine_files::lay_out(&root, addr)?;
            // Last, and appending rather than writing: a directory being
            // adopted usually has ignore rules of its own, and those are
            // the bytes adoption promises not to touch.
            crate::gitignore::place(&root)?;
            Ok(building)
        }
        Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => Err(AxError::failure(
            AxCode::InvalidArgs,
            "create a building",
            addr.as_str().to_owned(),
        )
        .with_recovery(
            "this building already has rules; edit its BUILDING.md, or create a building \
             at an address nobody occupies",
        )),
        Err(err) => Err(storage(&file, &err)),
    }
}

/// Adopts a directory that already exists - a checked-out repository, a
/// folder of notes - as a building. The same layout `create` writes, on
/// top of what is already there: `BUILDING.md` must not exist yet, and
/// the spine files are only laid where they are missing, so nothing the
/// directory holds is overwritten.
///
/// # Errors
/// Refuses an address with no directory (that is a create, not an
/// adopt), a room address, and a directory that is already a building.
pub fn adopt(city_root: &Path, addr: &Address) -> Result<Building, AxError> {
    let building = Building::of(addr)?;
    let root = building.root(city_root);
    if !root.is_dir() {
        return Err(AxError::failure(
            AxCode::InvalidArgs,
            "adopt a building",
            format!("{} holds no directory", addr.as_str()),
        )
        .with_recovery(
            "move or clone the directory under the city first, or use create for an empty \
             building",
        ));
    }
    create(city_root, addr, BuildingTemplate::Minimal)?;
    Ok(building)
}

/// What the ledger records about a building coming into being.
/// `adopted` says how: laid out empty, or drawn over an existing
/// directory - the history should not claim it built what it found.
///
/// # Errors
/// Propagates the payload's own refusal to hold what it was given.
pub fn created_payload(
    building: &Building,
    template: BuildingTemplate,
) -> Result<Payload, AxError> {
    let mut map = serde_json::Map::new();
    map.insert(
        "addr".to_owned(),
        serde_json::Value::String(building.addr().as_str().to_owned()),
    );
    map.insert(
        "template".to_owned(),
        serde_json::Value::String(template.name().to_owned()),
    );
    Payload::new(map)
}

/// The ledger record for an adoption.
///
/// # Errors
/// Propagates the payload's own refusal to hold what it was given.
pub fn adopted_payload(building: &Building) -> Result<Payload, AxError> {
    let mut map = serde_json::Map::new();
    map.insert(
        "addr".to_owned(),
        serde_json::Value::String(building.addr().as_str().to_owned()),
    );
    map.insert(
        "template".to_owned(),
        serde_json::Value::String(BuildingTemplate::Minimal.name().to_owned()),
    );
    map.insert("adopted".to_owned(), serde_json::Value::Bool(true));
    Payload::new(map)
}

/// The ledger record for a reconfiguration: which faces of a building's
/// own layer were written.
///
/// **What it deliberately does not carry is the configuration.**
/// `CONFIG.toml` is the authority for what a run is governed by, and a
/// copy of its contents on the ledger would be a second one. The fact
/// the file cannot hold is that somebody changed it, and that is what
/// this is for: a reader watching the city learns the building moved and
/// goes back to the file to see how.
///
/// # Errors
/// Propagates the payload's own refusal to hold what it was given.
pub fn configured_payload(
    building: &Building,
    sandbox: bool,
    mcp: bool,
) -> Result<Payload, AxError> {
    let mut map = serde_json::Map::new();
    map.insert(
        "addr".to_owned(),
        serde_json::Value::String(building.addr().as_str().to_owned()),
    );
    map.insert("sandbox".to_owned(), serde_json::Value::Bool(sandbox));
    map.insert("mcp".to_owned(), serde_json::Value::Bool(mcp));
    Payload::new(map)
}

fn storage(path: &Path, err: &std::io::Error) -> AxError {
    AxError::failure(
        AxCode::StorageFatal,
        "lay out a building",
        format!("{}: {err}", path.display()),
    )
    .with_recovery("fix the path's permissions, then create the building again")
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
