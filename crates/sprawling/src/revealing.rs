// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Showing a person the file a page is talking about, in the file
//! manager they already use.
//!
//! **A path printed on a page is a fact a person cannot act on.** They
//! can read `hall/.sprawling/DESKTOP.toml`, copy it, and then find their
//! own way to it; every step after the reading is work the city made
//! them do. This hands the whole path to the desktop's own file manager,
//! selected rather than merely opened, so what they were reading about
//! is what is highlighted when the window appears.
//!
//! **The grammar is the guard.** The address is an `Address`, which
//! cannot climb out of the city, and the path handed to the file manager
//! is the city root joined with it. A caller cannot ask this to reveal
//! something outside the city because there is no way to spell one.

use std::path::{Path, PathBuf};

use kernel::{Address, AxCode, AxError};

/// Opens the desktop's file manager with this address selected.
///
/// # Errors
/// When the address points at nothing on disk, or when the desktop's
/// file manager cannot be started. Both are refusals a person can act
/// on: the first says the city has no such file, the second says this
/// machine has no handler.
pub(crate) fn reveal(city_root: &Path, at: &Address) -> Result<(), AxError> {
    let path = city_root.join(at.as_str());
    if !path.exists() {
        return Err(AxError::failure(
            AxCode::PathNotFound,
            "reveal a path",
            at.as_str().to_owned(),
        )
        .with_recovery("this city holds no such file; the page may be older than the tree"));
    }
    manager(&path).status().map(drop).map_err(|err| {
        AxError::failure(AxCode::ToolUnavailable, "reveal a path", err.to_string())
            .with_recovery("this desktop has no file manager to hand a path to")
    })
}

/// What selects a path in this platform's file manager.
///
/// Windows and macOS both take the file itself and select it. Everything
/// else opens the directory: `xdg-open` has no selection argument, and
/// the desktops that do have one spell it differently per file manager,
/// which is a table this city would then have to keep current.
#[cfg(target_os = "windows")]
fn manager(path: &PathBuf) -> std::process::Command {
    let mut command = std::process::Command::new("explorer");
    // No space after the comma: `explorer` parses `/select,<path>` as one
    // argument and opens the person's home directory when it is two.
    command.arg(format!("/select,{}", path.display()));
    command
}

#[cfg(target_os = "macos")]
fn manager(path: &PathBuf) -> std::process::Command {
    let mut command = std::process::Command::new("open");
    command.args(["-R", &path.display().to_string()]);
    command
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn manager(path: &PathBuf) -> std::process::Command {
    let mut command = std::process::Command::new("xdg-open");
    command.arg(path.parent().unwrap_or(path).display().to_string());
    command
}

#[cfg(test)]
#[expect(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "test code"
)]
mod tests {
    use super::*;

    #[test]
    fn a_path_this_city_does_not_hold_is_refused_rather_than_opened() {
        let root = tempfile::tempdir().unwrap();
        let at = Address::parse("hall/nothing.md").unwrap();
        let refused = reveal(root.path(), &at).unwrap_err();
        assert_eq!(refused.code(), &AxCode::PathNotFound);
        assert!(
            refused.recovery().contains("no such file"),
            "{}",
            refused.recovery()
        );
    }

    /// The argument a file manager is handed, without starting one. The
    /// Windows spelling is the whole finding here: a space after the
    /// comma opens the home directory instead, silently and with a
    /// zero exit status.
    #[test]
    fn the_selected_path_travels_as_one_argument() {
        let path = PathBuf::from("C:/city/hall/JOB.md");
        let command = manager(&path);
        let written: Vec<String> = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        assert!(
            written.iter().any(|arg| arg.contains("JOB.md")),
            "the file manager is told which file: {written:?}"
        );
        if cfg!(target_os = "windows") {
            assert_eq!(written.len(), 1, "one argument, not two: {written:?}");
            assert!(
                written
                    .first()
                    .is_some_and(|arg| arg.starts_with("/select,")),
                "{written:?}"
            );
        }
    }
}
