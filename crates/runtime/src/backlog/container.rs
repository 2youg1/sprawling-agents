// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Register before create (`crates/runtime/spec/Tools/Exec/Container.lean`, D50).

use std::process::Command;

use kernel::{Address, AxError, ContainerLimits, RunId};

use crate::tools::{ContainerLease, ContainerRuntime};

use super::process::Process;
use super::{Backlog, Body, Claim, Member, Started, Tail, storage};

impl Backlog {
    pub(crate) fn run_container(
        &self,
        owner: RunId,
        scope: &Address,
        what: String,
        request: ContainerRequest,
    ) -> Result<Started, AxError> {
        let name = request
            .copy
            .file_name()
            .and_then(std::ffi::OsStr::to_str)
            .ok_or_else(|| {
                kernel::AxError::failure(
                    kernel::AxCode::SandboxDenied,
                    "name the owned container",
                    "the copy has no portable identity",
                )
                .with_recovery("choose a system temporary directory with a UTF-8 path")
            })?
            .to_owned();
        let lease = ContainerLease::registered(request.runtime, name, request.copy, request.limits);
        let id = self.mint()?;
        let dir = self.scratch.dir(id);
        std::fs::create_dir_all(&dir).map_err(|err| storage(&dir, &err))?;
        let out = std::fs::File::create(dir.join("out")).map_err(|err| storage(&dir, &err))?;
        let err = std::fs::File::create(dir.join("err")).map_err(|err| storage(&dir, &err))?;
        // The same table lock covers registration and start, so Halt cannot remove and then race a start.
        {
            let mut table = self.hold()?;
            let member = table.members.entry(id).or_insert(Member {
                scope: scope.clone(),
                what: what.clone(),
                body: Body::Command {
                    child: Process::Container(Box::new(lease)),
                    dir: dir.clone(),
                    claim: Claim::Window(owner),
                    tail: Tail::default(),
                },
            });
            if let Body::Command {
                child: Process::Container(lease),
                claim,
                ..
            } = &mut member.body
                && let Err(fault) =
                    lease.start(&request.target, out, err, &dir.join("guardian-ready"))
            {
                *claim = Claim::Nobody;
                // If removal fails the member remains owned and harvest retries it.
                lease.stop()?;
                table.members.remove(&id);
                std::fs::remove_dir_all(&dir).map_err(|err| storage(&dir, &err))?;
                return Err(fault);
            }
        }
        self.watch(id, owner, what, dir)
    }
}

pub(crate) struct ContainerRequest {
    pub(crate) runtime: ContainerRuntime,
    pub(crate) limits: ContainerLimits,
    pub(crate) copy: std::path::PathBuf,
    pub(crate) target: Command,
}
