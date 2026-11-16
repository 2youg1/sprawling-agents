// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::io::Write;
use std::path::{Component, Path, PathBuf};

use kernel::layout::CityLayout;
use kernel::{AxCode, AxError, PROTECTED_METADATA};

#[derive(Debug, Clone, Copy)]
pub enum Place<'a> {
    Chosen(&'a Path),

    Exports { city_root: &'a Path, file: &'a Path },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Standing {
    Outside,

    Ignored,

    Carried,

    Tracked,
}

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

fn cleared(place: Place<'_>) -> Result<PathBuf, AxError> {
    match place {
        Place::Chosen(path) => chosen(path),
        Place::Exports { city_root, file } => exported(city_root, file),
    }
}

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
            "the User has to make the city's reserved subtree writable ({err})"
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

fn staged_beside(target: &Path) -> PathBuf {
    let mut name = target.as_os_str().to_owned();
    name.push(format!(".partial-{}", std::process::id()));
    PathBuf::from(name)
}

fn stage(staged: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(staged)?;
    file.write_all(bytes)?;
    file.sync_all()
}
