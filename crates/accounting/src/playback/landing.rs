// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A playback export written as a new file that lands whole or not at
//! all (accounting-SPEC.md 8-13, decision 25(h)).
//!
//! Both doors write through here: the person's `--out` and a resident's
//! export into the city's playback exports. The bytes go to a staged
//! file beside the target, which is hard-linked to the target name and
//! then removed, so the target never exists half-written, an existing
//! file is never replaced, and every failure removes the staged copy.

use std::io::Write;
use std::path::{Component, Path, PathBuf};

use kernel::layout::CityLayout;
use kernel::{AxCode, AxError, PROTECTED_METADATA};

/// Where an export lands.
#[derive(Debug, Clone, Copy)]
pub enum Place<'a> {
    /// A path the person chose: anywhere outside protected metadata and
    /// the files git tracks.
    Chosen(&'a Path),
    /// A file under the playback exports of the city at `city_root`:
    /// reached through no link, and kept out of git's history.
    Exports { city_root: &'a Path, file: &'a Path },
}

/// What git makes of a name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Standing {
    /// No repository holds it.
    Outside,
    /// A repository holds it and ignores it.
    Ignored,
    /// A repository holds it and would list it as new work.
    Carried,
    /// A repository's index tracks it.
    Tracked,
}

/// Writes `bytes` as a new file at `place`.
///
/// # Errors
/// `E_OUTSIDE_WRITE_DOMAIN` for a target in protected metadata, outside
/// the city's playback exports, or carried into git's history from
/// there; `E_INVALID_ARGS` for a target that exists or that git tracks;
/// the alias refusal for a link on the way to an export; `E_PATH_NOT_FOUND`
/// for a chosen parent that does not resolve; `E_STORAGE_FATAL` for any
/// write or link that fails. The staged file is removed on every path.
pub fn land(place: Place<'_>, bytes: &[u8]) -> Result<(), AxError> {
    let target = cleared(place)?;
    let refuse = |code: AxCode, recovery: &str| {
        AxError::failure(
            code,
            "write a playback export",
            target.display().to_string(),
        )
        .with_recovery(recovery.to_owned())
    };
    if std::fs::symlink_metadata(&target).is_ok() {
        return Err(refuse(
            AxCode::InvalidArgs,
            "choose a file name that does not exist yet; a playback export never overwrites",
        ));
    }
    match (standing(&target)?, place) {
        (Standing::Tracked, Place::Chosen(_) | Place::Exports { .. }) => Err(refuse(
            AxCode::InvalidArgs,
            "choose a name git does not track; writing it would change a file in history",
        )),
        (Standing::Carried, Place::Exports { .. }) => Err(refuse(
            AxCode::OutsideWriteDomain,
            "keep the city's reserved subtree in its root .gitignore, which every city \
             writer places there, so an export stays out of history",
        )),
        (Standing::Carried | Standing::Ignored | Standing::Outside, Place::Chosen(_))
        | (Standing::Ignored | Standing::Outside, Place::Exports { .. }) => {
            through_staged(&target, bytes)
        }
    }
}

/// The target `place` names, once the rules of its kind of place hold.
fn cleared(place: Place<'_>) -> Result<PathBuf, AxError> {
    match place {
        Place::Chosen(path) => chosen(path),
        Place::Exports { city_root, file } => exported(city_root, file),
    }
}

/// A chosen path, its parent resolved through every link, refused when
/// any segment is protected metadata.
fn chosen(target: &Path) -> Result<PathBuf, AxError> {
    let refuse = |code: AxCode, recovery: String| {
        AxError::failure(
            code,
            "write a playback export",
            target.display().to_string(),
        )
        .with_recovery(recovery)
    };
    let (Some(name), parent) = (target.file_name(), target.parent()) else {
        return Err(refuse(
            AxCode::InvalidArgs,
            "name a file for --out, not a directory".to_owned(),
        ));
    };
    let parent = parent.filter(|parent| !parent.as_os_str().is_empty());
    let resolved = std::fs::canonicalize(parent.unwrap_or(Path::new("."))).map_err(|err| {
        refuse(
            AxCode::PathNotFound,
            format!("create the directory first ({err})"),
        )
    })?;
    let landing = resolved.join(name);
    if protected(&landing) {
        return Err(refuse(
            AxCode::OutsideWriteDomain,
            "write the export outside `.sprawling` and `.git`: the ledger and git's own \
             metadata take no export"
                .to_owned(),
        ));
    }
    Ok(landing)
}

/// A file under the city's playback exports, reached through no link,
/// with the directories above it made.
fn exported(city_root: &Path, file: &Path) -> Result<PathBuf, AxError> {
    let exports = CityLayout::new(city_root).playback_exports();
    let (Some(name), Some(parent)) = (file.file_name(), file.parent()) else {
        return Err(outside_exports(file));
    };
    if !parent.starts_with(&exports) {
        return Err(outside_exports(file));
    }
    storage::WriteTarget::within("write a playback export", city_root, file)
        .map_err(storage::StorageError::into_ax)?;
    std::fs::create_dir_all(parent).map_err(|err| {
        AxError::failure(
            AxCode::StorageFatal,
            "write a playback export",
            parent.display().to_string(),
        )
        .with_recovery(format!(
            "a person has to make the city's reserved subtree writable ({err})"
        ))
    })?;
    let resolved = std::fs::canonicalize(parent).map_err(|err| {
        AxError::failure(
            AxCode::StorageFatal,
            "write a playback export",
            parent.display().to_string(),
        )
        .with_recovery(format!("the exports directory did not resolve ({err})"))
    })?;
    Ok(resolved.join(name))
}

fn outside_exports(file: &Path) -> AxError {
    AxError::failure(
        AxCode::OutsideWriteDomain,
        "write a playback export",
        file.display().to_string(),
    )
    .with_recovery("a resident's export lands under the city's playback exports and nowhere else")
}

/// Whether any segment of `path` is protected metadata, compared without
/// ASCII case the way `Address::is_reserved` compares it.
fn protected(path: &Path) -> bool {
    path.components().any(|component| match component {
        Component::Normal(segment) => segment.to_str().is_some_and(|segment| {
            PROTECTED_METADATA
                .iter()
                .any(|name| segment.eq_ignore_ascii_case(name))
        }),
        Component::Prefix(_) | Component::RootDir | Component::CurDir | Component::ParentDir => {
            false
        }
    })
}

/// What git makes of `target`, whose parent is already resolved.
fn standing(target: &Path) -> Result<Standing, AxError> {
    let failed = |err: git2::Error| {
        AxError::failure(
            AxCode::StorageFatal,
            "ask git about a playback export",
            target.display().to_string(),
        )
        .with_recovery(format!(
            "repair or remove the repository around this path ({})",
            err.message()
        ))
    };
    let Some(parent) = target.parent() else {
        return Ok(Standing::Outside);
    };
    let repo = match git2::Repository::discover(parent) {
        Ok(repo) => repo,
        Err(err) if err.code() == git2::ErrorCode::NotFound => return Ok(Standing::Outside),
        Err(err) => return Err(failed(err)),
    };
    let Some(workdir) = repo
        .workdir()
        .and_then(|dir| std::fs::canonicalize(dir).ok())
    else {
        return Ok(Standing::Outside);
    };
    let Ok(relative) = target.strip_prefix(&workdir) else {
        return Ok(Standing::Outside);
    };
    if repo
        .index()
        .map_err(failed)?
        .get_path(relative, 0)
        .is_some()
    {
        return Ok(Standing::Tracked);
    }
    Ok(if repo.is_path_ignored(relative).map_err(failed)? {
        Standing::Ignored
    } else {
        Standing::Carried
    })
}

/// Stages `bytes` beside `target` and links the stage into place.
fn through_staged(target: &Path, bytes: &[u8]) -> Result<(), AxError> {
    let refuse = |recovery: String| {
        AxError::failure(
            AxCode::StorageFatal,
            "write a playback export",
            target.display().to_string(),
        )
        .with_recovery(recovery)
    };
    let staged = staged_beside(target);
    let landed = stage(&staged, bytes).and_then(|()| std::fs::hard_link(&staged, target));
    let cleared = std::fs::remove_file(&staged);
    match (landed, cleared) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(err), _) => Err(refuse(format!(
            "the export did not land ({err}); nothing was left at the target"
        ))),
        (Ok(()), Err(err)) => Err(refuse(format!(
            "the export landed, and its staged copy {} could not be removed ({err}); \
             delete it by hand",
            staged.display()
        ))),
    }
}

/// The staged file beside `target`, named for this process so two
/// exports to one directory never share one.
fn staged_beside(target: &Path) -> PathBuf {
    let mut name = target.as_os_str().to_owned();
    name.push(format!(".partial-{}", std::process::id()));
    PathBuf::from(name)
}

/// Writes `bytes` to a file that must not exist yet, through to the disk.
fn stage(staged: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(staged)?;
    file.write_all(bytes)?;
    file.sync_all()
}
