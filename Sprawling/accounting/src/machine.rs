// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxCode, AxError};

pub trait Machine {

    fn report(&self, city: &std::path::Path) -> wire::DoctorAnswer;

    fn install(&self, item: &str, runnable: &Runnable<'_>) -> Result<(), AxError>;
}

pub enum Recipe {

    Command {
        program: &'static str,
        args: &'static [&'static str],
    },

    Print(&'static str),

    Manual(&'static str),
}

impl Recipe {

    #[must_use]
    pub fn spelled(&self) -> String {
        match self {
            Recipe::Command { program, args } => Runnable::new(program, args).spelled(),
            Recipe::Print(line) => (*line).to_owned(),
            Recipe::Manual(how) => format!("manual: {how}"),
        }
    }

    pub fn command(&self, item: &str) -> Result<Runnable<'_>, AxError> {
        let recovery = match self {
            Recipe::Command { program, args } => return Ok(Runnable::new(program, args)),
            Recipe::Print(_) => {
                "run the printed line yourself: a script piped into a shell is code nobody read, \
                 and this city does not read it for you"
            }
            Recipe::Manual(_) => {
                "follow the printed instruction: nothing here can install this one"
            }
        };
        Err(AxError::failure(
            AxCode::ToolUnavailable,
            "install a tool",
            format!("{item}: {}", self.spelled()),
        )
        .with_recovery(recovery))
    }
}

pub struct Runnable<'a> {
    program: &'a str,
    args: &'a [&'a str],
}

impl<'a> Runnable<'a> {
    fn new(program: &'a str, args: &'a [&'a str]) -> Runnable<'a> {
        Runnable { program, args }
    }

    #[must_use]
    pub fn program(&self) -> &'a str {
        self.program
    }

    #[must_use]
    pub fn args(&self) -> &'a [&'a str] {
        self.args
    }

    #[must_use]
    pub fn spelled(&self) -> String {
        std::iter::once(self.program.to_owned())
            .chain(self.args.iter().map(|arg| {
                if arg.contains(' ') {
                    format!("\"{arg}\"")
                } else {
                    (*arg).to_owned()
                }
            }))
            .collect::<Vec<_>>()
            .join(" ")
    }
}
