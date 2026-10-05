// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Frozen execution policy reaches the admitted host mechanism here.

use super::super::super::{Assignment, mounts_under};
use super::super::engine::machine_half;
use super::super::{Laying, Site};
use kernel::AxError;
use runtime::ExecTool;

impl Laying {
    /// Builds the execution boundary.
    ///
    /// What the run may reach is the frozen configuration's answer;
    /// where the engine and the interpreter live is the machine's, so a
    /// city carried elsewhere does not carry this machine's paths with
    /// it.
    ///
    /// # Errors
    /// Propagates a build with no execution engine and whatever the
    /// tool says about its own construction.
    pub(super) fn exec_tool(
        &self,
        site: &Site,
        at: &Assignment,
        policy: runtime::PolicyReader,
    ) -> Result<ExecTool, AxError> {
        let machine = machine_half(&site.config.sandbox, &self.exec_host)?;
        let addr = &at.addr;
        let tool = ExecTool::new(
            runtime::ExecSetup {
                workdir: site.write_root.join(addr.as_str()),
                mounts: mounts_under(&site.write_root, &site.config.sandbox.mounts),
                python_wasm: machine.python_wasm,
                shell: machine.shell,
                fuel: runtime::Fuel(site.config.sandbox.fuel),
                // What a child may inherit is the building's own
                // declaration: the four-name floor is enough to run a
                // program and not enough to link one.
                env_passthrough: site.config.sandbox.env_passthrough.clone(),
                domain: addr.clone(),
                run: site.run_id,
                // The run's write limit, read at each command: under
                // `Create` a command runs only in the copy (`crates/runtime/Spec.lean` §8-55).
                policy,
            },
            machine.engine,
            self.backlog.clone(),
        )?;
        match site.config.sandbox.selected_arm()? {
            Some(kernel::SandboxArm::Container) => match &site.config.sandbox.container {
                Some(limits) => {
                    Ok(tool.with_container(limits.clone(), (self.exec_host.container)()?))
                }
                None => Err(kernel::AxError::failure(
                    kernel::AxCode::ConfigInvalid,
                    "construct container execution",
                    "limits are absent",
                )
                .with_recovery("provide sandbox.container limits")),
            },
            Some(arm @ (kernel::SandboxArm::Native | kernel::SandboxArm::CopiedTree)) => {
                Ok(tool.confined((self.exec_host.confinement)(arm)?))
            }
            Some(kernel::SandboxArm::None) => Ok(tool.on_host()),
            Some(kernel::SandboxArm::Python) => Ok(tool.python_only()),
            None => Ok(tool),
        }
    }
}
