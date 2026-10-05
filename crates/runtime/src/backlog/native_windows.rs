// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Native Windows registration is atomic with launch, so cancellation cannot
//! release the run Job between admission and the suspended assignment.

use super::process::Process;
use super::{Backlog, Body, Claim, Member, Started, Tail};
use crate::tools::native_windows;
use kernel::{Address, AxError, RunId};
use std::process::Command;

impl Backlog {
    pub(crate) fn run_native(
        &self,
        owner: RunId,
        scope: &Address,
        what: String,
        command: Command,
    ) -> Result<Started, AxError> {
        let id = self.mint()?;
        let dir = self.scratch.dir(id);
        std::fs::create_dir_all(&dir).map_err(|err| super::storage(&dir, &err))?;
        let limits = native_windows::limits(self.shares)?;
        {
            let mut table = self.hold()?;
            let job = match table.jobs.native_job(owner, limits.memory, self.affinity) {
                Ok(job) => job,
                Err(error) => {
                    table.jobs.keep_failed_native(
                        owner,
                        RetiringNative {
                            process: None,
                            output: dir.clone(),
                        },
                    );
                    return Err(error);
                }
            };
            let child = match native_windows::launch(command, &dir, &limits, job) {
                Ok(child) => child,
                Err(failure) => {
                    table.jobs.keep_failed_native(
                        owner,
                        RetiringNative {
                            process: failure.resources.map(|process| *process),
                            output: dir.clone(),
                        },
                    );
                    return Err(failure.error);
                }
            };
            table.members.insert(
                id,
                Member {
                    scope: scope.clone(),
                    what: what.clone(),
                    body: Body::Command {
                        child: Process::Native(Box::new(child)),
                        dir: dir.clone(),
                        claim: Claim::Window(owner),
                        tail: Tail::default(),
                    },
                },
            );
        }
        self.watch(id, owner, what, dir)
    }
}

pub(super) struct RetiringNative {
    process: Option<desktop_ffi::confinement::OwnedProcess>,
    output: std::path::PathBuf,
}

impl RetiringNative {
    pub(super) fn cleanup(&mut self) -> Result<(), AxError> {
        if let Some(process) = &mut self.process {
            process
                .retry_cleanup()
                .map_err(|error| native_windows::denied("clean failed native launch", error))?;
        }
        self.process = None;
        match std::fs::remove_dir_all(&self.output) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(native_windows::denied(
                "remove native output directory",
                error,
            )),
        }
    }
}

impl Drop for RetiringNative {
    fn drop(&mut self) {
        if let Err(error) = self.cleanup() {
            eprintln!("{error}");
        }
    }
}
