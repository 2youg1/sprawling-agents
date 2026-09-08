// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! This machine answering (sprawling-SPEC.md section 8-40).
//!
//! Two questions and one act live here, and nothing else does: is the
//! program on the search path or at a place this platform installs it,
//! what does it say when asked its version, and - after a person has
//! agreed to one named command - running that command with this
//! terminal's own stdio.
//!
//! **A version call has a deadline.** A tool installed half-way can hang
//! on start-up, and a doctor that hangs is worse than a tool that is
//! missing. The shape is `bin::mcp_stdio`'s: the read happens on a
//! thread, the wait happens on a channel that has a deadline, and a
//! deadline that passes kills the child. The deadline itself arrives as
//! a parameter, so no clock is sampled here.

use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::sync::mpsc::RecvTimeoutError;
use std::time::Duration;

use kernel::{AxCode, AxError};

use super::{Detection, Platform, Presence, Recipe, Requirement};

/// What is asked of the machine under this city. Two implementations:
/// `ThisMachine`, and the scripted one the tests drive, which is what
/// lets a verdict be judged without the machine that produced it.
pub(crate) trait Machine {
    /// Whether this item is here, and what it says about its version.
    fn look(&self, requirement: &Requirement) -> Presence;

    /// Runs one install command the person has just agreed to.
    ///
    /// # Errors
    /// Refuses a recipe this city may not run, a command this machine
    /// cannot start, and a command that ended in failure.
    fn install(&self, name: &str, recipe: &Recipe) -> Result<(), AxError>;
}

/// The machine this process is running on.
pub(crate) struct ThisMachine {
    platform: Option<Platform>,
    patience: Duration,
}

impl ThisMachine {
    pub(crate) fn new(platform: Option<Platform>, patience: Duration) -> ThisMachine {
        ThisMachine { platform, patience }
    }
}

impl Machine for ThisMachine {
    fn look(&self, requirement: &Requirement) -> Presence {
        match &requirement.detect {
            Detection::Environment { variable } => match std::env::var_os(variable) {
                Some(value) if Path::new(&value).exists() => {
                    Presence::Present(Path::new(&value).display().to_string())
                }
                _ => Presence::Absent,
            },
            Detection::Program {
                program,
                version_arg,
                places,
            } => {
                let found = on_search_path(program).or_else(|| {
                    self.platform
                        .and_then(|platform| at_a_known_place(places.at(platform)))
                });
                match found {
                    None => Presence::Absent,
                    Some(path) => {
                        Presence::Present(said_version(&path, version_arg, self.patience))
                    }
                }
            }
        }
    }

    fn install(&self, name: &str, recipe: &Recipe) -> Result<(), AxError> {
        let Recipe::Command { program, args } = recipe else {
            return Err(AxError::failure(
                AxCode::ToolUnavailable,
                "install a tool",
                format!("{name}: this platform has no command this city may run"),
            )
            .with_recovery("run the printed line yourself"));
        };
        let status = std::process::Command::new(program)
            .args(*args)
            .status()
            .map_err(|err| {
                AxError::failure(
                    AxCode::ToolUnavailable,
                    "install a tool",
                    format!("{name}: {program}: {err}"),
                )
                .with_recovery("install the package manager first, or run the printed line")
            })?;
        if status.success() {
            return Ok(());
        }
        Err(AxError::failure(
            AxCode::ToolUnavailable,
            "install a tool",
            format!("{name}: {} ended in failure", recipe.spelled()),
        )
        .with_recovery("run the printed line yourself to see what it reported"))
    }
}

/// The file names one program can carry on this platform, in the order
/// a shell resolves them.
///
/// The extensionless name comes last on Windows, and that ordering is
/// load-bearing: a package manager can leave both `bun` (a shell script
/// no Windows process starts) and `bun.cmd` in one directory, and
/// taking the first of them reports an installed tool as one that will
/// not say its version.
pub(super) fn names_of(program: &str) -> Vec<String> {
    if cfg!(target_os = "windows") {
        return ["exe", "cmd", "bat"]
            .into_iter()
            .map(|extension| format!("{program}.{extension}"))
            .chain(std::iter::once(program.to_owned()))
            .collect();
    }
    vec![program.to_owned()]
}

/// The first directory on the search path holding this program.
fn on_search_path(program: &str) -> Option<PathBuf> {
    let separator = if cfg!(target_os = "windows") {
        ';'
    } else {
        ':'
    };
    let search_path = std::env::var_os("PATH")?;
    let search_path = search_path.to_str()?.to_owned();
    let names = names_of(program);
    search_path
        .split(separator)
        .filter(|entry| !entry.trim().is_empty())
        .flat_map(|entry| names.iter().map(move |name| Path::new(entry).join(name)))
        .find(|candidate| candidate.is_file())
}

/// The first of this platform's standard install locations holding it.
/// Firefox is why this exists: on Windows and macOS it is installed
/// where no shell resolves it.
fn at_a_known_place(places: &[&str]) -> Option<PathBuf> {
    places
        .iter()
        .map(PathBuf::from)
        .find(|candidate| candidate.is_file())
}

/// What the program says when asked its version, or a sentence saying
/// that it said nothing. Presence is already settled by this point: a
/// program that will not talk is still installed.
fn said_version(program: &Path, version_arg: &str, patience: Duration) -> String {
    let spawned = std::process::Command::new(program)
        .arg(version_arg)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn();
    let Ok(mut child) = spawned else {
        return "version unknown".to_owned();
    };
    let Some(stdout) = child.stdout.take() else {
        let _ = child.kill();
        return "version unknown".to_owned();
    };
    let (sender, answers) = std::sync::mpsc::channel();
    let reader = std::thread::Builder::new()
        .name("doctor-version".to_owned())
        .spawn(move || {
            let first = std::io::BufReader::new(stdout).lines().next();
            // Either end finishing ends the reader: a closed pipe means
            // the program is gone, a closed channel means nobody waits.
            let _ = sender.send(first.and_then(Result::ok));
        });
    if reader.is_err() {
        let _ = child.kill();
        return "version unknown".to_owned();
    }
    let heard = answers.recv_timeout(patience);
    // The child is done with either way: a late line would be read as
    // the answer to a question nobody is asking any more.
    let _ = child.kill();
    let _ = child.wait();
    match heard {
        Ok(Some(line)) if !line.trim().is_empty() => line.trim().to_owned(),
        Ok(_) | Err(RecvTimeoutError::Disconnected) => "version unknown".to_owned(),
        Err(RecvTimeoutError::Timeout) => "no version within the deadline".to_owned(),
    }
}
