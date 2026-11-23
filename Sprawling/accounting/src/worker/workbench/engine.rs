// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::PathBuf;

use kernel::{AxError, Interpreter, SandboxLimits};
use runtime::Shell;

use crate::worker::hands::ExecHost;

pub(super) struct MachineHalf {
    pub(super) python: Option<PathBuf>,
    pub(super) shell: Shell,
}

pub(super) fn machine_half(
    limits: &SandboxLimits,
    host: &ExecHost,
) -> Result<MachineHalf, AxError> {
    let shell = match (limits.shell, limits.interpreter) {
        (false, _) => Shell::Absent,
        (true, asked) => {
            let found = match asked {
                Interpreter::System => (host.shell)(),
                Interpreter::Pwsh => (host.pwsh)(),
            };
            found.map_or(Shell::Missing { asked }, |program| Shell::Found {
                program,
                interpreter: asked,
            })
        }
    };
    Ok(MachineHalf {
        python: (host.python)(),
        shell,
    })
}
