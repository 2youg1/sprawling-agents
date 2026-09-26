// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The two verbs that are about the machine rather than about the city.
//!
//! **Neither writes to the Ledger.** What this machine has is not
//! something that happened in this city: it is a fact about the
//! computer the city is running on, it is different after somebody
//! installs something outside this process, and a history that carried
//! it would be a history that could be wrong. It travels to the views
//! the same way the start-up look does, through a sink set where the
//! city is served.
//!
//! **They run on the worker thread, and that is the cost.** Probing is
//! a dozen programs started and asked their version, and an install is
//! a package manager; both hold the queue every other command waits in.
//! That is the right place for them anyway: the alternative is a read
//! that starts processes, which would hold the one thread every other
//! read is answered on and would do it without anybody asking.
//!
//! **The worker reaches the machine through `accounting::Machine`**
//! (accounting-SPEC.md 8-4); `Doctor` is the production one, and
//! `with_machine` is the one door that swaps it.

use kernel::{AxCode, AxError};

use crate::doctor::{Machine, PATIENCE, Platform, REQUIREMENTS, ThisMachine};

use super::super::RunWorker;

/// The machine this process runs on, as the doctor sees it: the one
/// authority on what this machine has, and the one place this binary
/// starts an install program.
pub(in crate::assembly) struct Doctor;

impl accounting::Machine for Doctor {
    fn report(&self) -> channels::DoctorAnswer {
        crate::doctor::report()
    }

    fn install(&self, item: &str, runnable: &accounting::Runnable<'_>) -> Result<(), AxError> {
        ThisMachine::new(Platform::current(), PATIENCE).install(item, runnable)
    }
}

impl RunWorker {
    /// The same worker, looking at and installing onto `machine`
    /// instead of the machine it runs on (accounting-SPEC.md 8-4).
    ///
    /// The door citysim and the tests drive a worker through: what the
    /// machine answers and what an install does are theirs to script,
    /// while refusing a name the requirement table does not carry and a
    /// recipe this city may not run stay the worker's.
    #[must_use]
    pub fn with_machine(self, machine: Box<dyn accounting::Machine + Send>) -> RunWorker {
        RunWorker { machine, ..self }
    }

    /// Gets one thing this machine lacks, then looks again.
    ///
    /// The look is part of the verb rather than a second frame the page
    /// has to remember to send: an install that left the city answering
    /// the start-up snapshot would tell the person their new tool is
    /// still missing.
    ///
    /// **The rule the terminal follows is the rule here.**
    /// `doctor::screen` offers a person every absent item one at a time
    /// and runs only a `Recipe::Command`; this asks the same
    /// `Recipe::command` and runs the same `Machine::install`, so a
    /// script piped into a shell stays code nobody read whichever door
    /// the request arrived at. What is added here is the naming: a page
    /// sends a name, and a name the requirement table does not carry is
    /// refused rather than searched for.
    ///
    /// # Errors
    /// Refuses a name this city checks for nothing under, a platform
    /// this project states no recipes for, a recipe this city may not
    /// run, and whatever starting the program reports.
    pub(in crate::assembly) fn doctor_install(&mut self, item: &str) -> Result<(), AxError> {
        // A name the table does not carry is refused with the fact that
        // settles it: the page is asking about an item this build does
        // not know, so re-reading what the city answered is the way out.
        let requirement = REQUIREMENTS
            .iter()
            .find(|requirement| requirement.name == item)
            .ok_or_else(|| {
                AxError::failure(
                    AxCode::InvalidArgs,
                    "install a tool",
                    format!("{item}: this city checks for no such item"),
                )
                .with_recovery(
                    "ask this machine again and install one of the items it answered with",
                )
            })?;
        let Some(platform) = Platform::current() else {
            return Err(AxError::failure(
                AxCode::ToolUnavailable,
                "install a tool",
                format!("{item}: this platform has no recipe in this project"),
            )
            .with_recovery("install it the way this operating system installs software"));
        };
        let runnable = requirement.recipe.at(platform).command(item)?;
        // The log is the progress channel, so a line is written as the
        // install reaches it rather than at the end, which is when a
        // person has stopped waiting.
        self.note(
            runtime::diagnostics::Level::Effect,
            "bin::doctor",
            &format!("installing {item}: {}", runnable.spelled()),
        );
        self.machine.install(item, &runnable)?;
        self.note(
            runtime::diagnostics::Level::Effect,
            "bin::doctor",
            &format!("installed {item}; this city looked again"),
        );
        self.look_at_this_machine();
        Ok(())
    }

    /// Asks this machine every question the requirement table holds,
    /// and hands the answer to whoever is showing it.
    ///
    /// A worker with no sink still probes and still writes the line
    /// saying so: the command line drives such a worker, and a verb
    /// that silently did nothing there would be a verb whose behaviour
    /// depended on who was watching.
    pub(in crate::assembly) fn look_at_this_machine(&mut self) {
        let found = self.machine.report();
        let items = found.items.len();
        if let Some(serving) = self.serving.as_ref() {
            (serving.machine)(found);
        }
        self.note(
            runtime::diagnostics::Level::Effect,
            "bin::doctor",
            &format!("looked at this machine again: {items} item(s)"),
        );
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use crate::assembly::RunWorker;

    /// A worker over an empty city, which is all this verb needs: it
    /// refuses before it touches the machine.
    fn worker(city_root: &std::path::Path) -> RunWorker {
        let (ledger, _opened) = memory::JsonlLedger::open(
            &kernel::layout::CityLayout::new(city_root).ledger(),
            accounting::Clock::now(&crate::assembly::SystemClock).unwrap(),
        )
        .unwrap();
        RunWorker::over(
            city_root,
            gateway::Custodian::in_memory(),
            runtime::diagnostics::Diagnostics::off(),
            ledger,
        )
        .unwrap()
    }

    /// A name nobody offered is refused before any process starts, and
    /// the refusal says how to find a name that works.
    #[test]
    fn an_item_the_table_does_not_carry_starts_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let refused = worker(dir.path()).doctor_install("curl | sh").unwrap_err();
        assert_eq!(refused.code(), &kernel::AxCode::InvalidArgs);
        assert!(
            refused
                .recovery()
                .contains("install one of the items it answered with"),
            "the refusal says where a working name comes from: {}",
            refused.recovery()
        );
    }
}
