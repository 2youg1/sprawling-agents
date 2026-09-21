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

use kernel::{AxCode, AxError};

use crate::doctor::{Machine, PATIENCE, Platform, REQUIREMENTS, ThisMachine};

use super::super::RunWorker;

impl RunWorker {
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
        ThisMachine::new(Some(platform), PATIENCE).install(item, &runnable)?;
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
        let found = crate::doctor::report();
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
    use crate::assembly::{RunWorker, ledger_dir, now_ms};

    /// A worker over an empty city, which is all this verb needs: it
    /// refuses before it touches the machine.
    fn worker(city_root: &std::path::Path) -> RunWorker {
        let (ledger, _opened) =
            memory::JsonlLedger::open(&ledger_dir(city_root), now_ms().unwrap()).unwrap();
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
