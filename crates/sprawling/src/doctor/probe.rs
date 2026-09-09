// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! This machine answering (sprawling-SPEC.md sections 8-40 and 8-47).
//!
//! Four questions and one act live here, and nothing else does: is the
//! program on the search path or at a place this platform installs it,
//! is the component where the variable or the component directory says,
//! is the interpreter the platform names there, does this build carry
//! its engine - and, after a person has agreed to one named command,
//! running that command with this terminal's own stdio.
//!
//! **A version call has a deadline.** A tool installed half-way can hang
//! on start-up, and a doctor that hangs is worse than a tool that is
//! missing. The shape is `bin::mcp_stdio`'s: the read happens on a
//! thread, the wait happens on a channel that has a deadline, and a
//! deadline that passes kills the child. The deadline itself arrives as
//! a parameter, so no clock is sampled here.
//!
//! The functions below `ThisMachine` take what they read as parameters:
//! the search path, a variable's value, the component directory. The
//! tests hand them a temporary directory and never touch this process's
//! environment.

use std::ffi::OsString;
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::sync::mpsc::RecvTimeoutError;
use std::time::Duration;

use kernel::{AxCode, AxError};

use super::{Absence, Detection, Fault, Platform, Presence, Recipe, Requirement, Version};

/// What is asked of the machine under this city. Two implementations:
/// `ThisMachine`, and the scripted one the tests drive, which is what
/// lets a verdict be judged without the machine that produced it.
pub(crate) trait Machine {
    /// Whether this item is here, and in what condition.
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
        let search_path = std::env::var_os("PATH").unwrap_or_default();
        match &requirement.detect {
            Detection::Program {
                program,
                version_arg,
                places,
            } => {
                let found = on_search_path(&search_path, program).or_else(|| {
                    self.platform
                        .and_then(|platform| at_a_known_place(places.at(platform)))
                });
                match found {
                    None => Presence::Absent(Absence::NotOnSearchPath),
                    Some(path) => ask_version(&path, version_arg, self.patience),
                }
            }
            Detection::Component { variable, file } => {
                let dir = super::host::components_dir().map(|dir| dir.join(requirement.name));
                component_at(variable, std::env::var_os(variable), dir, file)
            }
            Detection::Interpreter { variable, fallback } => {
                let Some(platform) = self.platform else {
                    return Presence::Absent(Absence::NotOnSearchPath);
                };
                let named = interpreter_named(
                    std::env::var_os(variable.at(platform)),
                    fallback.at(platform),
                );
                let found = if named.is_absolute() {
                    named.is_file().then_some(named)
                } else {
                    on_search_path(&search_path, &named.display().to_string())
                };
                match found {
                    None => Presence::Absent(Absence::NotOnSearchPath),
                    Some(path) => ask_version(&path, "--version", self.patience),
                }
            }
            Detection::Built { carried } => built(*carried),
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
pub(super) fn on_search_path(search_path: &OsString, program: &str) -> Option<PathBuf> {
    let separator = if cfg!(target_os = "windows") {
        ';'
    } else {
        ':'
    };
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

/// The interpreter a variable names, or the platform's own when the
/// variable is unset or empty.
fn interpreter_named(value: Option<OsString>, fallback: &str) -> PathBuf {
    match value {
        Some(named) if !named.is_empty() => PathBuf::from(named),
        _ => PathBuf::from(fallback),
    }
}

/// Where a component is: the variable when set, the directory otherwise.
///
/// A set variable that names nothing is an answer, not a fall-through:
/// a person pointed somewhere, and the fault they need to hear about is
/// that the pointer is wrong.
pub(super) fn component_at(
    variable: &'static str,
    value: Option<OsString>,
    dir: Option<PathBuf>,
    file: &str,
) -> Presence {
    if let Some(named) = value.filter(|named| !named.is_empty()) {
        let path = PathBuf::from(named);
        return match std::fs::metadata(&path) {
            Ok(meta) if meta.is_file() => Presence::Present {
                at: path,
                version: Version::Silent,
            },
            Ok(_) => Presence::Broken {
                at: path,
                fault: Fault::Unreadable("not a file".to_owned()),
            },
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                Presence::Absent(Absence::VariableNamesNothing { variable, path })
            }
            Err(err) => Presence::Broken {
                at: path,
                fault: Fault::Unreadable(err.to_string()),
            },
        };
    }
    let Some(dir) = dir else {
        return Presence::Absent(Absence::NoHome);
    };
    match std::fs::metadata(&dir) {
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            Presence::Absent(Absence::NoComponent { dir })
        }
        Err(err) => Presence::Broken {
            at: dir,
            fault: Fault::Unreadable(err.to_string()),
        },
        Ok(_) => {
            let path = dir.join(file);
            match std::fs::metadata(&path) {
                Ok(meta) if meta.is_file() => Presence::Present {
                    at: path,
                    version: Version::Silent,
                },
                Ok(_) => Presence::Broken {
                    at: path,
                    fault: Fault::Unreadable("not a file".to_owned()),
                },
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => Presence::Broken {
                    at: dir,
                    fault: Fault::HalfWritten,
                },
                Err(err) => Presence::Broken {
                    at: path,
                    fault: Fault::Unreadable(err.to_string()),
                },
            }
        }
    }
}

/// Whether this build carries its engine, and whether it starts.
fn built(carried: bool) -> Presence {
    if !carried {
        return Presence::Absent(Absence::NotInThisBuild);
    }
    let at = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("this binary"));
    match super::host::execution_engine() {
        Ok(_) => Presence::Present {
            at,
            version: Version::Said("wasmtime".to_owned()),
        },
        Err(err) => Presence::Broken {
            at,
            fault: Fault::WillNotStart(err.to_string()),
        },
    }
}

/// Starts the program and takes the first line it writes, under a
/// deadline. A program that will not start is broken, not absent; one
/// that starts and says nothing is present all the same.
pub(super) fn ask_version(program: &Path, version_arg: &str, patience: Duration) -> Presence {
    let spawned = std::process::Command::new(program)
        .arg(version_arg)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn();
    let mut child = match spawned {
        Ok(child) => child,
        Err(err) => {
            return Presence::Broken {
                at: program.to_path_buf(),
                fault: Fault::WillNotStart(err.to_string()),
            };
        }
    };
    let version = match child.stdout.take() {
        Some(stdout) => first_line(stdout, patience),
        None => Version::Silent,
    };
    // The child is done with either way: a late line would be read as
    // the answer to a question nobody is asking any more.
    let _ = child.kill();
    let _ = child.wait();
    Presence::Present {
        at: program.to_path_buf(),
        version,
    }
}

/// The first line a pipe yields before the deadline.
fn first_line(stdout: std::process::ChildStdout, patience: Duration) -> Version {
    let (sender, answers) = std::sync::mpsc::channel();
    let reader = std::thread::Builder::new()
        .name("doctor-version".to_owned())
        .spawn(move || {
            let first = std::io::BufReader::new(stdout).lines().next();
            // Either end finishing ends the reader: a closed pipe means
            // the program is gone, a closed channel means nobody waits.
            let _ = sender.send(first);
        });
    if reader.is_err() {
        return Version::Silent;
    }
    match answers.recv_timeout(patience) {
        Ok(Some(Ok(line))) if !line.trim().is_empty() => Version::Said(line.trim().to_owned()),
        Ok(Some(Ok(_)) | None) => Version::Silent,
        Ok(Some(Err(_))) => Version::Unreadable,
        Err(RecvTimeoutError::Disconnected) => Version::Silent,
        Err(RecvTimeoutError::Timeout) => Version::Late,
    }
}
