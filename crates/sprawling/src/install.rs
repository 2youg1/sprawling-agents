// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Putting this binary where a shell will find it, and taking it back
//! out again (sprawling-SPEC.md section 8-9).
//!
//! Two things happen and exactly those two are reversed: the running
//! binary is copied into the per-user program directory, and that
//! directory is put on the user's own search path. Nothing outside the
//! person's profile is touched, so nothing here wants administrator
//! rights.
//!
//! The judgements live in `plan_append` and `plan_remove`, which are
//! pure functions over the search path string. They compile on Windows
//! alone, because Windows is the only platform whose installer edits
//! that string: elsewhere the line a person would add travels back in
//! the outcome instead, and a judgement with no caller is dead code the
//! zero-warning build is right to refuse. Everything below them is a
//! copy, a registry write and a broadcast.

use kernel::{AxCode, AxError};
use std::path::{Path, PathBuf};

/// The word this binary is installed as, whatever the archive called the
/// file. Making `sprawling` typeable cannot depend on who unpacked it.
const INSTALLED_STEM: &str = "sprawling";

/// The search path separator, which is the platform's and not ours.
#[cfg(target_os = "windows")]
const SEPARATOR: char = ';';
#[cfg(not(target_os = "windows"))]
const SEPARATOR: char = ':';

/// What installing does to the search path.
///
/// Two states rather than a bool, because "it was already there" and "I
/// put it there" are different things to tell a person afterwards.
#[cfg(target_os = "windows")]
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum PathEdit {
    AlreadyPresent,
    Append(String),
}

/// What uninstalling does to the search path.
#[cfg(target_os = "windows")]
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum PathRemoval {
    Absent,
    Rewrite(String),
}

/// What actually happened, in the order a person needs to hear it.
pub(crate) struct Report {
    pub(crate) binary: PathBuf,
    pub(crate) path: PathOutcome,
    /// Present when something worked only halfway: the search path was
    /// written but the desktop was not told.
    pub(crate) notice: Option<String>,
}

pub(crate) enum PathOutcome {
    /// The search path already said what it needed to say.
    Unchanged,
    /// The search path was rewritten and running shells will not see it.
    #[cfg_attr(
        not(target_os = "windows"),
        expect(
            dead_code,
            reason = "only the Windows search path is rewritten rather than handed back"
        )
    )]
    Rewritten,
    /// This platform does not have the search path edited for it; the
    /// line a person adds themselves travels with the outcome.
    #[cfg_attr(
        target_os = "windows",
        expect(
            dead_code,
            reason = "only the non-Windows search path hands the line back to the person"
        )
    )]
    SelfService(String),
}

/// The per-user program directory: where a binary a person installed for
/// themselves belongs on this platform.
///
/// `None` means neither location could be derived, which is the honest
/// answer on a machine with no home and no `LOCALAPPDATA`.
pub(crate) fn program_dir(local_app_data: Option<&Path>, home: Option<&Path>) -> Option<PathBuf> {
    if cfg!(target_os = "windows") {
        return local_app_data
            .map(|root| root.join("Programs").join(INSTALLED_STEM))
            .or_else(|| home.map(|home| home.join(".local").join("bin")));
    }
    home.map(|home| home.join(".local").join("bin"))
}

/// The file name this binary is installed under.
pub(crate) fn installed_name() -> String {
    format!("{INSTALLED_STEM}{}", std::env::consts::EXE_SUFFIX)
}

/// Whether a search path entry names the same directory as `dir`.
///
/// Compares case-insensitively on Windows because its file system does,
/// and ignores a trailing separator because `C:\x` and `C:\x\` are the
/// same directory to every shell that reads this string.
fn same_directory(entry: &str, dir: &str) -> bool {
    let normalise = |raw: &str| {
        let trimmed = raw.trim().trim_end_matches(['\\', '/']);
        if cfg!(target_os = "windows") {
            trimmed.to_lowercase()
        } else {
            trimmed.to_owned()
        }
    };
    !dir.trim().is_empty() && normalise(entry) == normalise(dir)
}

/// What the search path becomes when `dir` joins it.
///
/// Appends rather than prepends: a directory a person installed into
/// should not shadow what their system already resolves.
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

/// What the search path becomes when `dir` leaves it.
///
/// Every other entry survives byte for byte, including an empty one:
/// this reverses one append and audits nothing else.
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

/// Whether a directory is already reachable from this search path.
pub(crate) fn on_search_path(search_path: &str, dir: &str) -> bool {
    search_path
        .split(SEPARATOR)
        .any(|entry| same_directory(entry, dir))
}

/// Copies the running binary into `dir` under the installed name.
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
    Ok(target)
}

/// Removes the copy this put there, and says whether there was one.
fn displace(dir: &Path) -> Result<bool, AxError> {
    let target = dir.join(installed_name());
    match std::fs::remove_file(&target) {
        Ok(()) => Ok(true),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(err) => Err(AxError::failure(
            AxCode::StorageFatal,
            "remove the installed binary",
            format!("{}: {err}", target.display()),
        )
        .with_recovery("close any running sprawling and try again")),
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
    let home = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from);
    (local_app_data, home)
}

/// Installs, or reverses an install.
///
/// # Errors
/// Fails when there is nowhere to install to, when the copy cannot be
/// made or removed, or when the search path cannot be read or written.
pub(crate) fn install(uninstall: bool) -> Result<Report, AxError> {
    let (local_app_data, home) = dirs();
    let dir = program_dir(local_app_data.as_deref(), home.as_deref()).ok_or_else(no_home)?;
    let shown = dir.display().to_string();
    if uninstall {
        let removed = displace(&dir)?;
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
