// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::{Path, PathBuf};

use kernel::event::record::{BuildingConfigured, BuildingCreated};
use kernel::layout::CityLayout;
use kernel::{Address, AxCode, AxError, Payload};

use crate::policy::RULES_FILE;

pub(crate) mod removal;
pub(crate) mod template;

pub use removal::{Removed, remove, removed_payload};
pub use template::BuildingTemplate;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Building {
    addr: Address,
}

impl Building {
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

    #[must_use]
    pub fn root(&self, city_root: &Path) -> PathBuf {
        CityLayout::new(city_root).scope(&self.addr)
    }

    #[must_use]
    pub fn holds(&self, addr: &Address) -> bool {
        addr.is_within(&self.addr)
    }
}

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

pub(crate) fn names_a_building(name: &str) -> Option<Address> {
    if name.starts_with('.') {
        return None;
    }
    Address::parse(name).ok().filter(|addr| !addr.is_reserved())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RulesText<'a> {
    Template,

    Written(&'a str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Founding {
    Built,

    Adopted,
}

pub struct Raising<'a> {
    pub addr: &'a Address,
    pub template: BuildingTemplate,
    pub rules: RulesText<'a>,

    pub source: Option<&'a Path>,
}

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

pub fn template_rules(addr: &Address, template: BuildingTemplate) -> Result<String, AxError> {
    own_address(addr, "read a building template")?;
    template.rules(addr)
}

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

pub fn create(
    city_root: &Path,
    addr: &Address,
    template: BuildingTemplate,
) -> Result<Building, AxError> {
    let building = own_address(addr, "create a building")?;
    lay_out(city_root, &building, &template.rules(addr)?)?;
    Ok(building)
}

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

fn lay_out(city_root: &Path, building: &Building, rules: &str) -> Result<(), AxError> {
    lay_out_at(&building.root(city_root), building.addr(), rules)
}

fn lay_out_at(root: &Path, addr: &Address, rules: &str) -> Result<(), AxError> {
    let governed = root.join(kernel::RESERVED_PREFIX);
    std::fs::create_dir_all(&governed).map_err(|err| storage(&governed, &err))?;
    let file = governed.join(RULES_FILE);

    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&file)
    {
        Ok(mut handle) => {
            std::io::Write::write_all(&mut handle, rules.as_bytes())
                .map_err(|err| storage(&file, &err))?;

            crate::spine_files::lay_out(root, addr)?;

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

pub fn withdraw_adoption(city_root: &Path, building: &Building) -> Result<(), AxError> {
    let file = crate::policy::rules_path(city_root, building.addr());
    std::fs::remove_file(&file).map_err(|err| storage(&file, &err))
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Written {
    pub sandbox: bool,
    pub mcp: bool,

    pub desktop: bool,

    pub context: bool,

    pub catalog: bool,
}

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
