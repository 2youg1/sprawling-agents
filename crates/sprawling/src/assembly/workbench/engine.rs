// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The machine half of the exec tool: the three things about `exec`
//! that are this machine's rather than the city's, asked of the host
//! the worker was handed and shaped by what the frozen configuration
//! allows (sprawling-SPEC.md section 8-47, accounting-SPEC.md 8-11).
//!
//! Nothing here reads the search path, a variable or a feature flag.
//! What it holds is the judgement between the host's answer and the
//! bench: a shell reaches the bench only where a layer asked for one, a
//! component that is not here leaves the python arm to refuse at the
//! call, and an engine that will not start refuses the dispatch.

use std::path::PathBuf;

use kernel::{AxError, SandboxLimits};

use crate::assembly::hands::ExecHost;

/// What the exec tool takes from this machine.
pub(super) struct MachineHalf {
    pub(super) python_wasm: Option<PathBuf>,
    pub(super) shell: Option<PathBuf>,
    pub(super) engine: Box<dyn runtime::Sandbox>,
}

/// Asks the host for the component, the shell and the engine; the
/// shell only where a layer asked for one.
///
/// A broken component or shell arrives here as `None`: the exec tool's
/// own refusal names the arm, and `sprawling doctor --explain
/// E_TOOL_UNAVAILABLE` names the fault.
///
/// # Errors
/// Propagates an engine this build claims to carry and cannot start.
pub(super) fn machine_half(
    limits: &SandboxLimits,
    host: &ExecHost,
) -> Result<MachineHalf, AxError> {
    let shell = if limits.shell { (host.shell)() } else { None };
    Ok(MachineHalf {
        python_wasm: (host.python_wasm)(),
        shell,
        engine: (host.engine)()?,
    })
}
