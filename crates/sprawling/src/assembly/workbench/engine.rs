// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The execution engine this build carries, and the shell a layer may
//! ask for: the two things about `exec` that are the machine's rather
//! than the city's.

use kernel::AxError;

/// The sandbox this build carries, if it carries one.
///
/// # Errors
/// Propagates what starting the engine reports. A build that says it
/// carries one and cannot start it refuses the dispatch rather than
/// falling back: falling back is how a run that a person believed was
/// sandboxed turns out not to have been.
#[cfg(feature = "sandbox")]
pub(super) fn execution_engine() -> Result<Box<dyn runtime::Sandbox>, AxError> {
    Ok(Box::new(runtime::WasmtimeSandbox::new()?))
}

/// The engine `exec` runs a program in: none, in a build without one.
///
/// # Errors
/// None today; the signature matches the arm that can fail so the call
/// site does not change shape with the feature.
#[cfg(not(feature = "sandbox"))]
pub(super) fn execution_engine() -> Result<Box<dyn runtime::Sandbox>, AxError> {
    Ok(Box::new(runtime::AbsentSandbox))
}

pub(super) fn host_shell() -> Option<std::path::PathBuf> {
    let named = if cfg!(windows) { "COMSPEC" } else { "SHELL" };
    if let Ok(path) = std::env::var(named)
        && !path.is_empty()
    {
        return Some(std::path::PathBuf::from(path));
    }
    let fallback = if cfg!(windows) {
        std::path::PathBuf::from("cmd.exe")
    } else {
        std::path::PathBuf::from("/bin/sh")
    };
    Some(fallback)
}
