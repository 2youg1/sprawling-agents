// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::super::super::Assignment;
use super::super::engine::machine_half;
use super::super::{Laying, Site};
use kernel::AxError;
use runtime::ExecTool;

impl Laying {
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
                python: machine.python,
                shell: machine.shell,
                env_passthrough: site.config.sandbox.env_passthrough.clone(),
                domain: addr.clone(),
                run: site.run_id,
                policy,
            },
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
