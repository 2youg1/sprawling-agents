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

use kernel::AxError;

use super::super::RunWorker;

impl RunWorker {
    /// Gets one thing this machine lacks, then looks again.
    ///
    /// The look is part of the verb rather than a second frame the page
    /// has to remember to send: an install that left the city answering
    /// the start-up snapshot would tell the person their new tool is
    /// still missing.
    ///
    /// # Errors
    /// Propagates the refusal of a name this city checks for nothing
    /// under, of a recipe it may not run, and of a program that would
    /// not start.
    pub(in crate::assembly) fn doctor_install(&mut self, item: &str) -> Result<(), AxError> {
        // The log is the progress channel, so the lines are written as
        // the install reaches them rather than collected and reported
        // at the end, which is when a person has stopped waiting.
        let mut said: Vec<String> = Vec::new();
        let outcome = crate::doctor::installing::install(item, &mut |line| {
            said.push(line.to_owned());
        });
        for line in said {
            self.note(runtime::diagnostics::Level::Effect, "bin::doctor", &line);
        }
        outcome?;
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
        if let Some(sink) = self.machine.as_ref() {
            sink(found);
        }
        self.note(
            runtime::diagnostics::Level::Effect,
            "bin::doctor",
            &format!("looked at this machine again: {items} item(s)"),
        );
    }
}
