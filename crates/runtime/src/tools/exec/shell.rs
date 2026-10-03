// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which interpreter a shell line runs under, and the command line each
//! one takes (`crates/runtime/Spec.lean` §8-13-2 D30).
//!
//! The configuration names an interpreter and the machine says whether
//! it has it; this file only holds the answer the two arrive at. A
//! building that asked for PowerShell 7 on a machine without it is
//! refused by name rather than handed the platform's shell, because a
//! line written for one interpreter is a different language under the
//! other, and a clear refusal beats a failure nobody can read.

use std::path::{Path, PathBuf};
use std::process::Command;

use kernel::{AxCode, AxError, Interpreter};

/// The shell arm as this run's machine and configuration leave it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Shell {
    /// No layer of this run's configuration offers the shell arm.
    Absent,
    /// A layer offers it under `asked`, and this machine has no usable
    /// program for that interpreter.
    Missing { asked: Interpreter },
    /// The program a shell line is handed to, and which interpreter it is.
    Found {
        program: PathBuf,
        interpreter: Interpreter,
    },
}

impl Shell {
    /// The command that runs `text`, and the name the result records the
    /// interpreter under.
    ///
    /// # Errors
    /// `ToolUnavailable` when the arm is not offered or its interpreter
    /// is missing, each with the recovery that would work.
    pub(super) fn command(&self, text: &str) -> Result<(Command, String), AxError> {
        match self {
            Shell::Absent => Err(AxError::failure(
                AxCode::ToolUnavailable,
                "run shell",
                "no shell interpreter was found",
            )
            .with_recovery("use the program arm with an explicit executable")),
            Shell::Missing { asked } => Err(missing(*asked)),
            Shell::Found {
                program,
                interpreter,
            } => {
                let mut command = Command::new(program);
                command.args(flags(*interpreter)).arg(text);
                Ok((command, interpreter_name(program)))
            }
        }
    }

    /// The sentence the tool's description ends with: which interpreter
    /// a shell line runs under, in the spelling the result records, or
    /// nothing when the arm is not usable, whose refusal says why at the
    /// call. A model writes a different language for each interpreter.
    pub(super) fn statement(&self) -> String {
        match self {
            Shell::Absent | Shell::Missing { .. } => String::new(),
            Shell::Found { program, .. } => {
                format!(" A shell line runs under {}.", interpreter_name(program))
            }
        }
    }
}

/// The arguments that put one line in front of each interpreter.
///
/// PowerShell is told to load no profile and ask nothing, because a
/// profile is one person's machine and a prompt would hold the command
/// until its window closed.
fn flags(interpreter: Interpreter) -> &'static [&'static str] {
    match interpreter {
        Interpreter::System if cfg!(windows) => &["/C"],
        Interpreter::System => &["-c"],
        Interpreter::Pwsh => &["-NoLogo", "-NoProfile", "-NonInteractive", "-Command"],
    }
}

/// The refusal for an interpreter a building asked for and this machine
/// does not have.
fn missing(asked: Interpreter) -> AxError {
    match asked {
        Interpreter::System => AxError::failure(
            AxCode::ToolUnavailable,
            "run shell",
            "this machine's own shell is missing or does not start",
        )
        .with_recovery(
            "use the program arm with an explicit executable; `sprawling doctor` names what is \
             wrong with the shell",
        ),
        Interpreter::Pwsh => AxError::failure(
            AxCode::ToolUnavailable,
            "run shell",
            "this building runs shell lines under PowerShell 7 (pwsh), and this machine has no \
             pwsh 7 on its search path",
        )
        .with_recovery(
            "install PowerShell 7, or set `[sandbox] interpreter = \"system\"` in this building's \
             CONFIG.toml",
        ),
    }
}

/// The name a result records an interpreter under: the program's file
/// name without its extension, in lower case (`cmd`, `pwsh`, `sh`,
/// `bash`, `zsh`).
///
/// The program's name rather than the configured word, because the
/// tally asks which program ran, and `system` is a different program on
/// every platform.
pub(super) fn interpreter_name(program: &Path) -> String {
    program
        .file_stem()
        .unwrap_or(program.as_os_str())
        .to_string_lossy()
        .to_lowercase()
}
