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
//! in `crates/city/templates/RULES.toml`, not a copy of it kept here. One
//! string, one authority; the confidential template differs from the
//! ordinary one by the single line whose value the city refuses to
//! assume.
//!
//! What a new building starts with beyond its rules — the plan, the
//! memo, the handoff — is `crate::spine_files`'s to lay out.
//!
//! Specified by `crates/city/spec/Building.lean` §8-3.

use std::path::{Path, PathBuf};

use kernel::event::record::{BuildingConfigured, BuildingCreated};
use kernel::layout::CityLayout;
use kernel::{Address, AxCode, AxError, Payload};

use crate::policy::RULES_FILE;

pub(crate) mod removal;
pub(crate) mod template;

pub use removal::{Removed, remove, removed_payload};
pub use template::BuildingTemplate;

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
        CityLayout::new(city_root).scope(&self.addr)
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
/// with no `RULES.toml` is a building nobody has written rules for
/// yet - and a lister that hid it would hide the thing a person is
/// about to adopt.
///
/// # Errors
/// Propagates a city directory that cannot be read.
pub fn all(city_root: &Path) -> Result<Vec<Address>, AxError> {
    let unreadable = |err: &std::io::Error| {
        AxError::failure(
            AxCode::StorageFatal,
            "list the buildings of a city",
            format!("{}: {err}", city_root.display()),
        )
        .with_recovery("check the city directory is readable")
    };
    let entries = std::fs::read_dir(city_root).map_err(|err| unreadable(&err))?;
    let mut out = Vec::new();
    for entry in entries {
        // An entry this process cannot stat is reported rather than
        // skipped: a building missing from this list is a building a
        // person is told the city does not have.
        let entry = entry.map_err(|err| unreadable(&err))?;
        if !entry.path().is_dir() {
            continue;
        }
        if let Some(addr) = names_a_building(&entry.file_name().to_string_lossy()) {
            out.push(addr);
        }
    }
    out.sort_by(|left: &Address, right: &Address| left.as_str().cmp(right.as_str()));
    Ok(out)
}

/// The address a top-level directory named `name` has as a building,
/// or `None` when it can be none: a dot directory - the reserved subtree
/// among them - or a name the address grammar cannot spell, which
/// nothing could ever dispatch to.
///
/// The one rule [`all`] lists buildings by and `crate::wizard::survey`
/// offers folders by, so the folders a new city offers and the buildings
/// an old one lists cannot disagree.
pub(crate) fn names_a_building(name: &str) -> Option<Address> {
    if name.starts_with('.') {
        return None;
    }
    Address::parse(name).ok().filter(|addr| !addr.is_reserved())
}

/// The `RULES.toml` a raised building starts with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RulesText<'a> {
    /// The template's own bytes, with the building's name filled in.
    Template,
    /// What a person wrote, evaluated before anything lands.
    Written(&'a str),
}

/// How a raised building came to be at its address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Founding {
    /// Laid out where no directory stood.
    Built,
    /// Drawn over a directory that was already there, whose files are
    /// left as they were.
    Adopted,
}

/// What one raise is decided by: the address, the template whose bytes
/// the rules come from unless the person wrote them, and, when the
/// files come from somewhere else on this machine, the folder to copy.
pub struct Raising<'a> {
    pub addr: &'a Address,
    pub template: BuildingTemplate,
    pub rules: RulesText<'a>,
    /// An existing host directory to copy to `addr` before adopting it.
    pub source: Option<&'a Path>,
}

/// Raises the building a person asked for at `addr`: laid out where no
/// directory stands, or drawn over the one already there, and says which.
///
/// The directory decides, because whether it is there is a fact of the
/// disk and this city is its reader: a request that said "adopt" or
/// "create" itself would be a second answer, and the two could disagree.
/// Written rules are evaluated by the reader every dispatch uses
/// (`crate::policy::evaluate`) before anything is laid out, so a refused
/// text leaves no directory and no file behind. A `source` is the person's
/// other choice: that folder is copied in first, and the copy is adopted.
///
/// # Errors
/// Refuses what [`create`] refuses, a source that is not an absolute
/// directory outside the city, a source that already holds building
/// rules, and written rules the evaluator refuses.
pub fn raise(city_root: &Path, asking: Raising<'_>) -> Result<(Building, Founding), AxError> {
    let Raising {
        addr,
        template,
        rules,
        source,
    } = asking;
    let building = own_address(addr, "raise a building")?;
    let text = match rules {
        RulesText::Template => template.rules(addr)?,
        RulesText::Written(text) => {
            crate::policy::evaluate(addr, text)?;
            text.to_owned()
        }
    };
    if let Some(source) = source {
        import(city_root, &building, source, &text)?;
        return Ok((building, Founding::Adopted));
    }
    let founding = if building.root(city_root).is_dir() {
        Founding::Adopted
    } else {
        Founding::Built
    };
    lay_out(city_root, &building, &text)?;
    Ok((building, founding))
}

/// Copies one host directory into a new building, lays its rules into
/// the private staging tree, then makes the whole copy visible at once.
fn import(
    city_root: &Path,
    building: &Building,
    source: &Path,
    rules: &str,
) -> Result<(), AxError> {
    let source = import_source(city_root, source)?;
    let target = building.root(city_root);
    match std::fs::symlink_metadata(&target) {
        Ok(_) => {
            return Err(invalid_import(
                &target,
                "the target already exists; choose another address or adopt that folder without a source",
            ));
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => return Err(import_io(&target, &err)),
    }
    refuse_governed_source(&source)?;
    let staged = target.with_file_name(format!(".{}.importing", building.addr().as_str()));
    remove_staging(&staged)?;
    let placed = copy_directory(&source, &staged, 0)
        .and_then(|()| lay_out_at(&staged, building.addr(), rules))
        .and_then(|()| std::fs::rename(&staged, &target).map_err(|err| import_io(&target, &err)));
    if let Err(error) = placed {
        return Err(cleanup_import(&staged, error));
    }
    Ok(())
}

/// Reads the source's identity before copying and excludes every tree
/// that would make the copy read its own destination or another city.
fn import_source(city_root: &Path, source: &Path) -> Result<PathBuf, AxError> {
    if !source.is_absolute() {
        return Err(invalid_import(
            source,
            "the source must be an absolute path on the machine running the city",
        ));
    }
    let metadata = std::fs::symlink_metadata(source).map_err(|err| import_io(source, &err))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(invalid_import(
            source,
            "the source must be a real directory, not a file or a link",
        ));
    }
    let source = std::fs::canonicalize(source).map_err(|err| import_io(source, &err))?;
    let city = std::fs::canonicalize(city_root).map_err(|err| import_io(city_root, &err))?;
    if source == city || source.starts_with(&city) || city.starts_with(&source) {
        return Err(invalid_import(
            &source,
            "choose a directory outside the city; a folder already in the city is adopted without copying",
        ));
    }
    Ok(source)
}

/// A source that already holds building rules cannot be imported.
/// Adoption refuses to overwrite that file, so the refusal happens
/// before the copy and names the file in the source. A link at the same
/// path is left to the copy, which refuses every link with one sentence.
fn refuse_governed_source(source: &Path) -> Result<(), AxError> {
    let rules = source.join(kernel::RESERVED_PREFIX).join(RULES_FILE);
    match std::fs::symlink_metadata(&rules) {
        Ok(metadata) if metadata.file_type().is_symlink() => Ok(()),
        Ok(_) => Err(invalid_import(
            &rules,
            "this folder already has building rules; import a directory that is not already a building, or remove that rules file and import again",
        )),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(import_io(&rules, &err)),
    }
}

/// One directory copy, bounded against cycles and refusing links rather
/// than following them into content the person did not name. A real
/// `.git` directory is skipped: it is the host's version-control store,
/// not the workspace's files, and copying it blocks the city for a long
/// time on ordinary projects (city Building.lean §8-3, city D35). A file
/// or link by that name is not a store, so the file comes in and the
/// link meets the same refusal as every other link.
fn copy_directory(source: &Path, target: &Path, depth: u32) -> Result<(), AxError> {
    const DEPTH_MAX: u32 = 64;
    if depth > DEPTH_MAX {
        return Err(invalid_import(
            source,
            "the directory nests deeper than 64 levels",
        ));
    }
    std::fs::create_dir(target).map_err(|err| import_io(target, &err))?;
    let entries = std::fs::read_dir(source).map_err(|err| import_io(source, &err))?;
    for entry in entries {
        let entry = entry.map_err(|err| import_io(source, &err))?;
        let name = entry.file_name();
        let from = entry.path();
        let into = target.join(&name);
        let metadata = std::fs::symlink_metadata(&from).map_err(|err| import_io(&from, &err))?;
        if metadata.file_type().is_symlink() {
            return Err(invalid_import(
                &from,
                "links are not copied; replace this link with a real file or directory",
            ));
        }
        if metadata.is_dir() {
            if name == ".git" {
                continue;
            }
            let deeper = depth.checked_add(1).ok_or_else(|| {
                invalid_import(&from, "the directory depth cannot be represented")
            })?;
            copy_directory(&from, &into, deeper)?;
        } else if metadata.is_file() {
            std::fs::copy(&from, &into).map_err(|err| import_io(&from, &err))?;
        } else {
            return Err(invalid_import(
                &from,
                "only ordinary files and directories can be imported",
            ));
        }
    }
    Ok(())
}

/// A previous interrupted import may leave its private staging tree;
/// it never became the target, so replacing that staging is safe.
fn remove_staging(staged: &Path) -> Result<(), AxError> {
    match std::fs::symlink_metadata(staged) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {
            std::fs::remove_dir_all(staged).map_err(|err| import_io(staged, &err))
        }
        Ok(_) => std::fs::remove_file(staged).map_err(|err| import_io(staged, &err)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(import_io(staged, &err)),
    }
}

/// A failed import keeps the original refusal and adds the staging path
/// if cleaning the private copy also failed.
fn cleanup_import(staged: &Path, error: AxError) -> AxError {
    match std::fs::remove_dir_all(staged) {
        Ok(()) => error,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => error,
        Err(err) => {
            let recovery = error.recovery().to_owned();
            error.rewrite_recovery(format!(
                "{recovery}; remove the incomplete staging directory {} before trying again: {err}",
                staged.display()
            ))
        }
    }
}

fn invalid_import(path: &Path, recovery: &str) -> AxError {
    AxError::failure(
        AxCode::InvalidArgs,
        "import a workspace",
        path.display().to_string(),
    )
    .with_recovery(recovery)
}

fn import_io(path: &Path, err: &std::io::Error) -> AxError {
    let code = if err.kind() == std::io::ErrorKind::NotFound {
        AxCode::PathNotFound
    } else {
        AxCode::StorageFatal
    };
    AxError::failure(
        code,
        "import a workspace",
        format!("{}: {err}", path.display()),
    )
    .with_recovery(
        "check that the source exists and is readable, that the city is writable, and that the disk has room",
    )
}

/// The `RULES.toml` `template` lays a building out with at `addr`: what
/// a page shows a person before they raise it.
///
/// # Errors
/// Refuses a room address and the reserved subtree, as [`create`] does.
pub fn template_rules(addr: &Address, template: BuildingTemplate) -> Result<String, AxError> {
    own_address(addr, "read a building template")?;
    template.rules(addr)
}

/// Whether rules have been drawn over this building's directory. A
/// top-level folder without them is one [`all`] lists and nobody has
/// adopted yet; [`raise`] at its address adopts it.
///
/// # Errors
/// `E_STORAGE_FATAL` when the rules file is there and cannot be looked at.
pub fn has_rules(city_root: &Path, building: &Building) -> Result<bool, AxError> {
    let file = crate::policy::rules_path(city_root, building.addr());
    match std::fs::metadata(&file) {
        Ok(_) => Ok(true),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(err) => Err(AxError::failure(
            AxCode::StorageFatal,
            "look for a building's rules",
            format!("{}: {err}", file.display()),
        )
        .with_recovery("fix the file's permissions; a rules file that exists is read")),
    }
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
    let building = own_address(addr, "create a building")?;
    lay_out(city_root, &building, &template.rules(addr)?)?;
    Ok(building)
}

/// The building `addr` is the address of.
///
/// # Errors
/// Refuses the reserved subtree, and a room: a room is an address inside
/// a building, not one.
fn own_address(addr: &Address, action: &str) -> Result<Building, AxError> {
    let building = Building::of(addr)?;
    if building.addr() != addr {
        return Err(AxError::failure(
            AxCode::InvalidArgs,
            action.to_owned(),
            addr.as_str().to_owned(),
        )
        .with_recovery(format!(
            "create the building `{}` and dispatch to `{}` inside it; a room is not a building",
            building.addr().as_str(),
            addr.as_str()
        )));
    }
    Ok(building)
}

/// Writes `rules` as the building's `RULES.toml` and lays its spine files
/// where they are missing.
///
/// # Errors
/// Refuses a building that already has rules, and a path that will not
/// take the write.
fn lay_out(city_root: &Path, building: &Building, rules: &str) -> Result<(), AxError> {
    lay_out_at(&building.root(city_root), building.addr(), rules)
}

/// Lays out a building at a chosen root. Imports use a private staging
/// root; every other caller passes the building's actual root.
fn lay_out_at(root: &Path, addr: &Address, rules: &str) -> Result<(), AxError> {
    // The building's own reserved subtree, made before the file that
    // lives in it: what governs a building is not writable by what runs
    // inside it (`crates/kernel/Spec.lean` §8-28).
    let governed = root.join(kernel::RESERVED_PREFIX);
    std::fs::create_dir_all(&governed).map_err(|err| storage(&governed, &err))?;
    let file = governed.join(RULES_FILE);
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
            crate::spine_files::lay_out(root, addr)?;
            // Last, and appending rather than writing: a directory being
            // adopted usually has ignore rules of its own, and those are
            // the bytes adoption promises not to touch.
            crate::gitignore::place(root)?;
            Ok(())
        }
        Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => Err(AxError::failure(
            AxCode::InvalidArgs,
            "create a building",
            addr.as_str().to_owned(),
        )
        .with_recovery(format!(
            "this building already has rules; edit its {RULES_FILE}, or create a building \
             at an address nobody occupies"
        ))),
        Err(err) => Err(storage(&file, &err)),
    }
}

/// Adopts a directory that already exists - a checked-out repository, a
/// folder of notes - as a building. The same layout `create` writes, on
/// top of what is already there: `RULES.toml` must not exist yet, and
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

/// Takes back the rules [`raise`] or [`adopt`] drew over a folder that
/// was already there, for an adoption the city could not finish: the
/// folder waits to be adopted again (`crates/city/Spec.lean` §8-3).
///
/// Call it only for a building whose rules this adoption wrote and whose
/// `building_created` line has not landed. Only the rules file goes: the
/// adoption created it new, so nothing a person wrote is lost, and the
/// spine files and ignore rules stay because they were laid only where
/// missing and a second adoption lays nothing twice.
///
/// # Errors
/// `E_STORAGE_FATAL` naming the file when it cannot be removed.
pub fn withdraw_adoption(city_root: &Path, building: &Building) -> Result<(), AxError> {
    let file = crate::policy::rules_path(city_root, building.addr());
    std::fs::remove_file(&file).map_err(|err| storage(&file, &err))
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
    Payload::of(&BuildingCreated {
        addr: building.addr().clone(),
        template: template.name().to_owned(),
        adopted: false,
    })
}

/// The ledger record for an adoption, under the template its rules came
/// from.
///
/// # Errors
/// Propagates the payload's own refusal to hold what it was given.
pub fn adopted_payload(
    building: &Building,
    template: BuildingTemplate,
) -> Result<Payload, AxError> {
    Payload::of(&BuildingCreated {
        addr: building.addr().clone(),
        template: template.name().to_owned(),
        adopted: true,
    })
}

/// Which faces of a building's own layer one reconfiguration wrote.
///
/// One value rather than three parameters: they are decided together,
/// recorded together, and a call site spelling `(true, false, true)`
/// says nothing about which face is which.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Written {
    pub sandbox: bool,
    pub mcp: bool,
    /// The desktop allowlist, which lives in the building's reserved
    /// subtree rather than in `CONFIG.toml`.
    pub desktop: bool,
    /// Where the context reminder's second rung sits, `[context]` of
    /// the building's own layer.
    pub context: bool,
    /// The building's `[catalog]` table: how each admitted tool reaches
    /// its runs.
    pub catalog: bool,
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
pub fn configured_payload(building: &Building, wrote: Written) -> Result<Payload, AxError> {
    Payload::of(&BuildingConfigured {
        addr: building.addr().clone(),
        sandbox: wrote.sandbox,
        mcp: wrote.mcp,
        desktop: wrote.desktop,
        context: wrote.context,
        catalog: wrote.catalog,
    })
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
