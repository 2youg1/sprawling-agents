// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A worker looking at the machine it was handed
//! (`crates/accounting/Spec.lean` section 2).
//!
//! The scripted machine answers with one item, where the machine under
//! any test answers with every item the requirement table holds, so a
//! worker that still asks the host reports a different count. The
//! scripted install records what it was handed instead of starting a
//! package manager, so a worker that installs by any other door
//! records nothing.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use std::sync::{Arc, Mutex};

use kernel::{AxError, IdemKey, RunId, Seq};
// The city is formed with an in-memory vault, because these tests are
// about the worker and must not write to the credential service of the
// machine that runs them.
use accounting::worker::genesis::{Adopt, form};
use sprawling::assembly;

struct OneItem;

impl accounting::Machine for OneItem {
    fn report(&self, _city: &std::path::Path) -> wire::DoctorAnswer {
        wire::DoctorAnswer {
            items: vec![wire::DoctorItem {
                name: "scripted".to_owned(),
                tier: wire::DoctorTier::Use,
                need: wire::DoctorNeed::Optional,
                homepage: None,
                state: wire::DoctorState::Absent {
                    absence: wire::DoctorAbsence::NotOnSearchPath,
                },
                install: wire::DoctorInstall::Manual {
                    how: "script it".to_owned(),
                },
                pinned: None,
                pack: None,
            }],
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

    fn install(&self, item: &str, _runnable: &accounting::Runnable<'_>) -> Result<(), AxError> {
        panic!("a refresh installed {item}")
    }
}

#[test]
fn a_refresh_counts_the_items_the_machine_it_was_handed_answered() {
    let dir = tempfile::tempdir().unwrap();
    form(
        dir.path(),
        Adopt::Nothing,
        assembly::hands(gateway::Custodian::in_memory()),
    )
    .unwrap();
    let lines = Arc::new(Mutex::new(Vec::<String>::new()));
    let heard = Arc::clone(&lines);
    let log = runtime::diagnostics::Diagnostics::new(
        runtime::diagnostics::Level::Effect,
        Box::new(move |entry| heard.lock().unwrap().push(entry.message.to_owned())),
    );
    let mut worker = accounting::worker::RunWorker::new(
        dir.path(),
        log,
        assembly::hands(gateway::Custodian::in_memory()),
    )
    .unwrap()
    .with_machine(Box::new(OneItem));

    worker
        .handle(wire::Command::DoctorRefresh {
            idem: IdemKey::derive(&RunId::CITY, Seq::FIRST, b"refresh"),
        })
        .unwrap();

    let looked: Vec<String> = lines
        .lock()
        .unwrap()
        .iter()
        .filter(|line| line.starts_with("looked at this machine again"))
        .cloned()
        .collect();
    assert_eq!(
        looked,
        vec!["looked at this machine again: 1 item(s)".to_owned()]
    );
}

/// Answers like `OneItem` and records every install it is handed, so a
/// worker that installs through anything but its machine records nothing.
struct Recording(Arc<Mutex<Vec<(String, String)>>>);

impl accounting::Machine for Recording {
    /// What it installed is present afterwards, so the install ends
    /// with the item found rather than with the refusal a package
    /// manager that left it unfindable earns.
    fn report(&self, city: &std::path::Path) -> wire::DoctorAnswer {
        let mut answer = OneItem.report(city);
        answer.items.extend(
            self.0
                .lock()
                .unwrap()
                .iter()
                .map(|(item, _)| wire::DoctorItem {
                    name: item.clone(),
                    tier: wire::DoctorTier::Use,
                    need: wire::DoctorNeed::Optional,
                    homepage: None,
                    state: wire::DoctorState::Present {
                        at: format!("/bin/{item}"),
                        version: wire::DoctorVersion::Silent,
                    },
                    install: wire::DoctorInstall::UnknownPlatform,
                    pinned: None,
                    pack: None,
                }),
        );
        answer
    }

    fn install(&self, item: &str, runnable: &accounting::Runnable<'_>) -> Result<(), AxError> {
        self.0
            .lock()
            .unwrap()
            .push((item.to_owned(), runnable.spelled()));
        Ok(())
    }
}

#[test]
fn an_install_hands_the_table_command_to_the_machine_it_was_handed() {
    let dir = tempfile::tempdir().unwrap();
    form(
        dir.path(),
        Adopt::Nothing,
        assembly::hands(gateway::Custodian::in_memory()),
    )
    .unwrap();
    let lines = Arc::new(Mutex::new(Vec::<String>::new()));
    let heard = Arc::clone(&lines);
    let log = runtime::diagnostics::Diagnostics::new(
        runtime::diagnostics::Level::Effect,
        Box::new(move |entry| heard.lock().unwrap().push(entry.message.to_owned())),
    );
    let installs = Arc::new(Mutex::new(Vec::new()));
    let mut worker = accounting::worker::RunWorker::new(
        dir.path(),
        log,
        assembly::hands(gateway::Custodian::in_memory()),
    )
    .unwrap()
    .with_machine(Box::new(Recording(Arc::clone(&installs))));

    let outcome = worker.handle(wire::Command::DoctorInstall {
        idem: IdemKey::derive(&RunId::CITY, Seq::FIRST, b"install"),
        item: "ffmpeg".to_owned(),
    });

    // The table's ffmpeg recipe is a command on Windows and macOS and a
    // printed line on Linux, which the worker refuses before the machine.
    let expected = match sprawling::doctor::Platform::current() {
        Some(sprawling::doctor::Platform::Windows) => {
            Some("winget install --id Gyan.FFmpeg -e --scope user")
        }
        Some(sprawling::doctor::Platform::MacOs) => Some("brew install ffmpeg"),
        Some(sprawling::doctor::Platform::Linux) | None => None,
    };
    let recorded = installs.lock().unwrap().clone();
    match expected {
        Some(spelled) => {
            outcome.unwrap();
            assert_eq!(recorded, vec![("ffmpeg".to_owned(), spelled.to_owned())]);
            assert!(
                lines
                    .lock()
                    .unwrap()
                    .iter()
                    .any(|line| line.starts_with("installed ffmpeg"))
            );
        }
        None => {
            assert!(outcome.is_err());
            assert_eq!(recorded, Vec::new());
        }
    }
}
