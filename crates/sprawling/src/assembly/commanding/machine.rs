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
//! (accounting-SPEC.md 8-4); `doctor::ThisMachine` is the production
//! one, and `with_machine` is the one door that swaps it.

use kernel::AxError;

use super::super::RunWorker;

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
        let runnable = (self.recipe_for)(item)?.command(item)?;
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
        let found = self.machine.report();
        let here = found.items.iter().any(|each| {
            each.name == item && matches!(each.state, channels::DoctorState::Present { .. })
        });
        self.show_this_machine(found);
        if here {
            Ok(())
        } else {
            Err(still_absent(item))
        }
    }

    /// Asks this machine every question the requirement table holds,
    /// and hands the answer to whoever is showing it.
    ///
    /// A worker with no sink still probes and still writes the line
    /// saying so: the command line drives such a worker, and a verb
    /// that silently did nothing there would be a verb whose behaviour
    /// depended on who was watching.
    /// Replaces the requirement table's lookup, so a test can install
    /// an item this build does not carry without starting anything.
    #[cfg(test)]
    pub(in crate::assembly) fn recipe_for_with(
        &mut self,
        recipe_for: fn(&str) -> Result<&'static accounting::Recipe, AxError>,
    ) {
        self.recipe_for = recipe_for;
    }

    pub(in crate::assembly) fn look_at_this_machine(&mut self) {
        let found = self.machine.report();
        self.show_this_machine(found);
    }

    /// Hands one answer about this machine to whoever is showing it,
    /// and writes the line that says it was taken.
    fn show_this_machine(&mut self, found: channels::DoctorAnswer) {
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

/// The package manager said it succeeded and this city still cannot
/// find the item: the refusal that ends this install for a page walking
/// a list of them (sprawling-SPEC.md section 8-64).
fn still_absent(item: &str) -> AxError {
    AxError::failure(
        kernel::AxCode::ToolUnavailable,
        "install a tool",
        format!("{item}: the installer finished, and this city still cannot find it"),
    )
    .with_recovery(
        "restart this city so it reads the search path the installer changed, then check again",
    )
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
        RunWorker::over(
            city_root,
            gateway::Custodian::in_memory(),
            runtime::diagnostics::Diagnostics::off(),
            memory::JsonlLedger::open(
                &kernel::layout::CityLayout::new(city_root).ledger(),
                accounting::Clock::now(&crate::assembly::SystemClock).unwrap(),
            )
            .unwrap(),
        )
        .unwrap()
    }

    /// A name nobody offered is refused before any process starts, and
    /// the refusal says how to find a name that works.
    /// The one item the scripted table carries.
    const SCRIPTED: &str = "scripted-tool";

    fn scripted_table(item: &str) -> Result<&'static accounting::Recipe, kernel::AxError> {
        static RECIPE: accounting::Recipe = accounting::Recipe::Command {
            program: "scripted-installer",
            args: &["scripted-tool"],
        };
        if item == SCRIPTED {
            Ok(&RECIPE)
        } else {
            Err(kernel::AxError::failure(
                kernel::AxCode::InvalidArgs,
                "install a tool",
                item.to_owned(),
            )
            .with_recovery("nothing: this table is a script"))
        }
    }

    /// Installs nothing and remembers what it was asked to install.
    /// After an install it reports the item present, or nothing at all
    /// when it stands for a package manager that said it succeeded and
    /// left the program where this city does not look.
    struct Recording(std::sync::Arc<std::sync::Mutex<Vec<String>>>, Finds);

    enum Finds {
        WhatWasInstalled,
        Nothing,
    }

    impl accounting::Machine for Recording {
        fn report(&self) -> channels::DoctorAnswer {
            let items = match self.1 {
                Finds::Nothing => Vec::new(),
                Finds::WhatWasInstalled => self
                    .0
                    .lock()
                    .unwrap()
                    .iter()
                    .map(|_installed| channels::DoctorItem {
                        name: SCRIPTED.to_owned(),
                        tier: channels::DoctorTier::Develop,
                        need: channels::DoctorNeed::Required,
                        homepage: None,
                        state: channels::DoctorState::Present {
                            at: "/bin/scripted-tool".to_owned(),
                            version: channels::DoctorVersion::Silent,
                        },
                        install: channels::DoctorInstall::UnknownPlatform,
                        pinned: None,
                        pack: None,
                    })
                    .collect(),
            };
            channels::DoctorAnswer {
                items,
                tiers: Vec::new(),
                sandbox: channels::DoctorSandbox {
                    arm: channels::DoctorSandboxArm::CopiedTree,
                    coverage: Vec::new(),
                },
                custody: channels::DoctorCustody {
                    store: channels::DoctorCustodyStore::SessionMemory,
                    keeps: channels::DoctorCustodyLifetime::ThisProcess,
                    refusal: None,
                },
                core: channels::DoctorCore::HeldBySetting,
            }
        }

        fn install(
            &self,
            item: &str,
            runnable: &accounting::Runnable<'_>,
        ) -> Result<(), kernel::AxError> {
            self.0
                .lock()
                .unwrap()
                .push(format!("{item}: {}", runnable.spelled()));
            Ok(())
        }
    }

    /// The item is in no table this build carries, so a worker that
    /// still reads the requirement table itself refuses it, and the
    /// machine it was handed is never asked to install anything.
    #[test]
    fn an_install_takes_its_recipe_from_the_table_the_worker_was_handed() {
        let dir = tempfile::tempdir().unwrap();
        let installed = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut worker = worker(dir.path()).with_machine(Box::new(Recording(
            std::sync::Arc::clone(&installed),
            Finds::WhatWasInstalled,
        )));
        worker.recipe_for_with(scripted_table);

        let told = worker
            .doctor_install(SCRIPTED)
            .map_err(|refusal| *refusal.code());

        assert_eq!(
            (told, installed.lock().unwrap().clone()),
            (
                Ok(()),
                vec!["scripted-tool: scripted-installer scripted-tool".to_owned()]
            )
        );
    }

    /// A package manager that reports success while this city still
    /// cannot find the item ends the install with a refusal naming it,
    /// so a page walking a list of installs knows this one is over.
    #[test]
    fn an_install_that_leaves_the_item_absent_is_refused_by_name() {
        let dir = tempfile::tempdir().unwrap();
        let installed = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut worker = worker(dir.path()).with_machine(Box::new(Recording(
            std::sync::Arc::clone(&installed),
            Finds::Nothing,
        )));
        worker.recipe_for_with(scripted_table);

        let told = worker.doctor_install(SCRIPTED).map_err(|refusal| {
            (
                *refusal.code(),
                refusal.subject().starts_with("scripted-tool:"),
            )
        });

        assert_eq!(told, Err((kernel::AxCode::ToolUnavailable, true)));
    }

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
