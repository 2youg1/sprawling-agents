// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::{Path, PathBuf};
use std::process::Command;

use kernel::{AxCode, AxError, Interpreter};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Shell {
    Absent,

    Missing {
        asked: Interpreter,
    },

    Found {
        program: PathBuf,
        interpreter: Interpreter,
    },
}

impl Shell {
    pub(super) fn command(&self, text: &str) -> Result<(Command, String), AxError> {
        match self {
            Shell::Absent => Err(AxError::failure(
                AxCode::ToolUnavailable,
                "run shell",
                "this building offers no shell",
            )
            .with_recovery(
                "pass the command as one program with its arguments, with no pipes, redirects \
                 or wildcards",
            )),
            Shell::Missing { asked } => Err(missing(*asked)),
            Shell::Found {
                program,
                interpreter,
            } => {
                let mut command = child::command(program);
                command.args(flags(*interpreter)).arg(text);
                Ok((command, interpreter_name(program)))
            }
        }
    }

    pub(super) fn container_command(&self, text: &str) -> Result<(Command, String), AxError> {
        match self {
            Shell::Absent => self.command(text),
            Shell::Missing { asked }
            | Shell::Found {
                interpreter: asked, ..
            } => {
                let (program, arguments) = match asked {
                    Interpreter::System => ("/bin/sh", &["-c"][..]),
                    Interpreter::Pwsh => ("pwsh", flags(Interpreter::Pwsh)),
                };
                let mut command = child::command(program);
                command.args(arguments).arg(text);
                Ok((command, interpreter_name(Path::new(program))))
            }
        }
    }

    pub(super) fn statement(&self) -> String {
        match self {
            Shell::Absent => ABSENT.to_owned(),
            Shell::Missing { .. } => String::new(),
            Shell::Found { program, .. } => {
                format!(" A command runs under {}.", interpreter_name(program))
            }
        }
    }
}

pub(super) const ABSENT: &str = " This building has no shell: a command is one program and its \
     arguments, with no pipes, redirects or wildcards.";

impl Shell {}

fn flags(interpreter: Interpreter) -> &'static [&'static str] {
    match interpreter {
        Interpreter::System if cfg!(windows) => &["/C"],
        Interpreter::System => &["-c"],
        Interpreter::Pwsh => &["-NoLogo", "-NoProfile", "-NonInteractive", "-Command"],
    }
}

fn missing(asked: Interpreter) -> AxError {
    match asked {
        Interpreter::System => AxError::failure(
            AxCode::ToolUnavailable,
            "run shell",
            "this machine's own shell is missing or does not start",
        )
        .with_recovery(
            "ask the User to fix the shell `sprawling doctor` names, or to turn this building's \
             shell off so a command runs as one program with its arguments",
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

pub(super) fn interpreter_name(program: &Path) -> String {
    program
        .file_stem()
        .unwrap_or(program.as_os_str())
        .to_string_lossy()
        .to_lowercase()
}
