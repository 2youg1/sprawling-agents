// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxCode, AxError};
use sprawling::doctor::Platform;
use std::path::{Path, PathBuf};

const INSTALLED_STEM: &str = "sprawling";

#[cfg(target_os = "windows")]
const SEPARATOR: char = ';';
#[cfg(not(target_os = "windows"))]
const SEPARATOR: char = ':';

#[cfg(target_os = "windows")]
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum PathEdit {
    AlreadyPresent,
    Append(String),
}

#[cfg(target_os = "windows")]
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum PathRemoval {
    Absent,
    Rewrite(String),
}

pub(crate) struct Report {
    pub(crate) binary: PathBuf,
    pub(crate) path: PathOutcome,

    pub(crate) notice: Option<String>,
}

pub(crate) enum PathOutcome {
    Unchanged,

    #[cfg_attr(
        not(target_os = "windows"),
        expect(
            dead_code,
            reason = "only the Windows search path is rewritten rather than handed back"
        )
    )]
    Rewritten,

    #[cfg_attr(
        target_os = "windows",
        expect(
            dead_code,
            reason = "only the non-Windows search path hands the line back to the person"
        )
    )]
    SelfService(String),
}

pub(crate) fn program_dir(local_app_data: Option<&Path>, home: Option<&Path>) -> Option<PathBuf> {
    let under_home = || home.map(|home| home.join(".local").join("bin"));
    match Platform::current() {
        Some(Platform::Windows) => local_app_data
            .map(|root| root.join("Programs").join(INSTALLED_STEM))
            .or_else(under_home),

        Some(Platform::MacOs | Platform::Linux) | None => under_home(),
    }
}

pub(crate) fn installed_name() -> String {
    format!("{INSTALLED_STEM}{}", std::env::consts::EXE_SUFFIX)
}

fn same_directory(entry: &str, dir: &str) -> bool {
    let normalise = |raw: &str| {
        let trimmed = raw.trim().trim_end_matches(['\\', '/']);
        match Platform::current() {
            Some(Platform::Windows) => trimmed.to_lowercase(),
            Some(Platform::MacOs | Platform::Linux) | None => trimmed.to_owned(),
        }
    };
    !dir.trim().is_empty() && normalise(entry) == normalise(dir)
}

#[cfg(target_os = "windows")]
pub(crate) fn plan_append(current: &str, dir: &str) -> PathEdit {
    if on_search_path(current, dir) {
        return PathEdit::AlreadyPresent;
    }
    if current.trim().is_empty() {
        return PathEdit::Append(dir.to_owned());
    }
    let joined = format!("{}{SEPARATOR}{dir}", current.trim_end_matches(SEPARATOR));
    PathEdit::Append(joined)
}

#[cfg(target_os = "windows")]
pub(crate) fn plan_remove(current: &str, dir: &str) -> PathRemoval {
    let kept: Vec<&str> = current
        .split(SEPARATOR)
        .filter(|entry| !same_directory(entry, dir))
        .collect();
    if kept.len() == current.split(SEPARATOR).count() {
        return PathRemoval::Absent;
    }
    PathRemoval::Rewrite(kept.join(&SEPARATOR.to_string()))
}

pub(crate) fn on_search_path(search_path: &str, dir: &str) -> bool {
    search_path
        .split(SEPARATOR)
        .any(|entry| same_directory(entry, dir))
}

fn place(dir: &Path) -> Result<PathBuf, AxError> {
    let source = std::env::current_exe().map_err(|err| {
        AxError::failure(AxCode::PathNotFound, "find this binary", err.to_string())
            .with_recovery("run the binary by its path rather than through a shim")
    })?;
    std::fs::create_dir_all(dir).map_err(|err| {
        AxError::failure(
            AxCode::StorageFatal,
            "create the program directory",
            format!("{}: {err}", dir.display()),
        )
        .with_recovery("check that this account may write there")
    })?;
    let target = dir.join(installed_name());
    if target != source {
        std::fs::copy(&source, &target).map_err(|err| {
            AxError::failure(
                AxCode::StorageFatal,
                "copy this binary into place",
                format!("{}: {err}", target.display()),
            )
            .with_recovery("close any running sprawling and try again")
        })?;
    }
    std::fs::write(dir.join(sprawling::release::ARCHIVE_ORIGIN_FILE), b"").map_err(|err| {
        AxError::failure(
            AxCode::StorageFatal,
            "record the archive install origin",
            err.to_string(),
        )
        .with_recovery("check write access in the program directory and repeat sprawling install")
    })?;
    Ok(target)
}

fn displace(dir: &Path) -> Result<bool, AxError> {
    let target = dir.join(installed_name());
    remove_installed_entry(&target).map_err(|err| {
        AxError::failure(
            AxCode::StorageFatal,
            "remove the installed binary",
            format!("{}: {err}", target.display()),
        )
        .with_recovery(
            "close any other running sprawling and try again; if the installed name was already moved, repeat install from the original archive",
        )
    })
}

fn remove_installed_entry(target: &Path) -> std::io::Result<bool> {
    #[cfg(target_os = "windows")]
    {
        let entry = match std::fs::symlink_metadata(target) {
            Ok(entry) => entry,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(err) => return Err(err),
        };

        if !entry.is_symlink()
            && target.canonicalize()? == std::env::current_exe()?.canonicalize()?
        {
            let dir = target.parent().ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "the installed binary has no parent directory",
                )
            })?;
            self_replace::self_delete_outside_path(dir)?;
            return Ok(true);
        }
    }
    match std::fs::remove_file(target) {
        Ok(()) => Ok(true),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(err) => Err(err),
    }
}

fn no_home() -> AxError {
    AxError::failure(
        AxCode::PathNotFound,
        "find a per-user program directory",
        "neither LOCALAPPDATA nor a home directory is set",
    )
    .with_recovery("set HOME, or copy the binary somewhere on PATH yourself")
}

fn dirs() -> (Option<PathBuf>, Option<PathBuf>) {
    let local_app_data = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    let home = match accounting::home::Home::detect() {
        Ok(home) => Some(home.path().to_path_buf()),
        Err(_) => None,
    };
    (local_app_data, home)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Direction {
    Install,
    Uninstall,
}

pub(crate) fn install(direction: Direction) -> Result<Report, AxError> {
    let (local_app_data, home) = dirs();
    let dir = program_dir(local_app_data.as_deref(), home.as_deref()).ok_or_else(no_home)?;
    let shown = dir.display().to_string();
    if direction == Direction::Uninstall {
        let removed = displace(&dir)?;
        match std::fs::remove_file(dir.join(sprawling::release::ARCHIVE_ORIGIN_FILE)) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => {
                return Err(AxError::failure(
                    AxCode::StorageFatal,
                    "remove the archive install origin",
                    err.to_string(),
                )
                .with_recovery(
                    "check write access in the program directory and repeat sprawling uninstall",
                ));
            }
        }
        let path = retract(&shown)?;
        return Ok(Report {
            binary: dir.join(installed_name()),
            path,
            notice: (!removed).then(|| format!("nothing was installed in {shown}")),
        });
    }
    let binary = place(&dir)?;
    let (path, notice) = extend(&shown)?;
    Ok(Report {
        binary,
        path,
        notice,
    })
}

#[cfg(not(target_os = "windows"))]
mod search_path_elsewhere;
#[cfg(target_os = "windows")]
mod search_path_windows;

#[cfg(not(target_os = "windows"))]
use search_path_elsewhere::{extend, retract};
#[cfg(target_os = "windows")]
use search_path_windows::{extend, retract};

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests;
