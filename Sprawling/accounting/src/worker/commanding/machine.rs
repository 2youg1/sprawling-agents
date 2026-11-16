// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::AxError;

use super::super::RunWorker;

impl RunWorker {

    #[must_use]
    pub fn with_machine(self, machine: Box<dyn crate::Machine + Send>) -> RunWorker {
        RunWorker { machine, ..self }
    }

    pub(in crate::worker) fn doctor_install(&mut self, item: &str) -> Result<(), AxError> {
        let runnable = (self.recipe_for)(item)?.command(item)?;

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
        let found = self.machine.report(&self.city_root);
        let here = found.items.iter().any(|each| {
            each.name == item && matches!(each.state, wire::DoctorState::Present { .. })
        });
        self.show_this_machine(found);
        if here {
            Ok(())
        } else {
            Err(still_absent(item))
        }
    }

    #[cfg(test)]
    pub(in crate::worker) fn recipe_for_with(
        &mut self,
        recipe_for: fn(&str) -> Result<&'static crate::Recipe, AxError>,
    ) {
        self.recipe_for = recipe_for;
    }

    pub(in crate::worker) fn look_at_this_machine(&mut self) {
        let found = self.machine.report(&self.city_root);
        self.show_this_machine(found);
    }

    fn show_this_machine(&mut self, found: wire::DoctorAnswer) {
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
    use crate::worker::RunWorker;

    fn worker(city_root: &std::path::Path) -> RunWorker {
        RunWorker::over(
            city_root,
            runtime::diagnostics::Diagnostics::off(),
            crate::worker::fixture::hands(),
            storage::JsonlLedger::open(
                &kernel::layout::CityLayout::new(city_root).ledger(),
                crate::Clock::now(&crate::worker::fixture::WallClock).unwrap(),
            )
            .unwrap(),
        )
        .unwrap()
    }

    const SCRIPTED: &str = "scripted-tool";

    fn scripted_table(item: &str) -> Result<&'static crate::Recipe, kernel::AxError> {
        static RECIPE: crate::Recipe = crate::Recipe::Command {
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

    struct Recording(std::sync::Arc<std::sync::Mutex<Vec<String>>>, Finds);

    enum Finds {
        WhatWasInstalled,
        Nothing,
    }

    impl crate::Machine for Recording {
        fn report(&self, _city: &std::path::Path) -> wire::DoctorAnswer {
            let items = match self.1 {
                Finds::Nothing => Vec::new(),
                Finds::WhatWasInstalled => self
                    .0
                    .lock()
                    .unwrap()
                    .iter()
                    .map(|_installed| wire::DoctorItem {
                        name: SCRIPTED.to_owned(),
                        tier: wire::DoctorTier::Develop,
                        need: wire::DoctorNeed::Required,
                        homepage: None,
                        state: wire::DoctorState::Present {
                            at: "/bin/scripted-tool".to_owned(),
                            version: wire::DoctorVersion::Silent,
                        },
                        install: wire::DoctorInstall::UnknownPlatform,
                        pinned: None,
                        pack: None,
                    })
                    .collect(),
            };
            wire::DoctorAnswer {
                items,
                tiers: Vec::new(),
                sandbox: wire::DoctorSandbox {
                    arm: wire::DoctorSandboxArm::CopiedTree,
                    named: wire::SandboxArm::CopiedTree,
                    coverage: Vec::new(),
                },
                custody: wire::DoctorCustody {
                    store: wire::DoctorCustodyStore::SessionMemory,
                    keeps: wire::DoctorCustodyLifetime::ThisProcess,
                    refusal: None,
                },
                core: wire::DoctorCore::HeldBySetting,
                scanning: wire::DoctorScanning::DoesNotApply,
            }
        }

        fn install(
            &self,
            item: &str,
            runnable: &crate::Runnable<'_>,
        ) -> Result<(), kernel::AxError> {
            self.0
                .lock()
                .unwrap()
                .push(format!("{item}: {}", runnable.spelled()));
            Ok(())
        }
    }

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
        let installed = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let mut worker = worker(dir.path()).with_machine(Box::new(Recording(
            std::sync::Arc::clone(&installed),
            Finds::WhatWasInstalled,
        )));
        worker.recipe_for_with(scripted_table);

        let told = worker
            .doctor_install("curl | sh")
            .map_err(|refusal| *refusal.code());

        assert_eq!(
            (told, installed.lock().unwrap().clone()),
            (Err(kernel::AxCode::InvalidArgs), Vec::new())
        );
    }
}
