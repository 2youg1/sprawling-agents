// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The machine the city runs on, asked what it has and handed one
//! install for the worker rather than by it (accounting-SPEC.md 8-4).
//!
//! **Holding a `Runnable` proves the recipe was a `Command`.** Its
//! constructor is private to this crate, and `Recipe::command` is the
//! one place that calls it, refusing a printed recipe and a manual one
//! first. It does not prove permission: `Recipe::Command` has public
//! fields, so what keeps a program outside the requirement table from
//! running is the worker's `doctor_install`, which looks the name up in
//! that table before it builds a `Runnable`.

use kernel::{AxCode, AxError};

/// What the worker asks of the machine under the city.
///
/// The worker has already refused a name the requirement table does not
/// carry and a platform with no recipe by the time it asks; an
/// implementation decides only how the machine is looked at and how the
/// command is run.
pub trait Machine {
    /// Asks this machine every question the requirement table holds,
    /// in the shape a page reads.
    fn report(&self) -> wire::DoctorAnswer;

    /// Runs one install command the person has just agreed to.
    ///
    /// # Errors
    /// A program this machine cannot start, one that ended in failure,
    /// and one still running when its patience ran out.
    fn install(&self, item: &str, runnable: &Runnable<'_>) -> Result<(), AxError>;
}

/// What installing one item costs on one platform.
pub enum Recipe {
    /// A command this machine may run, once the person has agreed to it.
    /// Every one of them is a per-user install; none asks for elevation.
    Command {
        program: &'static str,
        args: &'static [&'static str],
    },
    /// A command printed and never run. A script piped into a shell is
    /// code nobody read, so this city prints it and the person decides.
    Print(&'static str),
    /// Nothing here can install it; the line says what a person does.
    Manual(&'static str),
}

impl Recipe {
    /// The command as a person would type it, or the manual instruction.
    #[must_use]
    pub fn spelled(&self) -> String {
        match self {
            Recipe::Command { program, args } => Runnable::new(program, args).spelled(),
            Recipe::Print(line) => (*line).to_owned(),
            Recipe::Manual(how) => format!("manual: {how}"),
        }
    }

    /// The program this city may start for `item`, or the refusal that
    /// says what the person does instead.
    ///
    /// **This is the only place a recipe is refused for not being
    /// runnable.** Every caller - the terminal, the page, the machine
    /// adapter - asks here, so a person is told the same thing about a
    /// printed script whichever door they arrived at.
    ///
    /// # Errors
    /// `E_TOOL_UNAVAILABLE` for a printed recipe, because a script piped
    /// into a shell is code nobody read, and for a manual one, because
    /// nothing here can install it.
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

/// A program and its arguments from a recipe that is a command.
///
/// Constructed only by `Recipe::command` and `Recipe::spelled`, which
/// live beside it: a value of this type came from a `Command` recipe.
/// Whether this city may run it is the requirement-table lookup the
/// worker makes first.
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

    /// The command as a person would type it into their own terminal.
    /// Every message about this program quotes this, so what a person
    /// is told to run is what this city ran. An argument with a space in
    /// it is set in double quotes, which every shell a person pastes it
    /// into reads as one argument, as the program received it.
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
