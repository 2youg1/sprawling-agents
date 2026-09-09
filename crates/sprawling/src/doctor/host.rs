// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The one door the rest of this binary asks what this machine has
//! through (sprawling-SPEC.md section 8-47).
//!
//! The exec tool wants the python component, the shell and the engine;
//! the browser tool wants Firefox or a driver. Before this file each of
//! them read the machine its own way, and a person could be told by
//! `doctor` that Firefox was here while a run was refused for having
//! none. Every answer here comes from the same table and the same probe
//! the report is printed from, so the two cannot disagree.
//!
//! What this file does not do is decide: a `Presence` goes back as it
//! is, and the caller - which knows what the run may reach - turns it
//! into a tool argument or a refusal.

use std::path::PathBuf;

use kernel::AxError;

use super::table::{CHROMEDRIVER, FIREFOX, PYTHON_WASI, REQUIREMENTS, SHELL};
use super::{Machine, PATIENCE, Platform, Presence, ThisMachine};

/// Whether this binary was built with the `sandbox` feature. The one
/// spelling of that fact; the table reads it and the engine below
/// selects on it.
pub(crate) const ENGINE_CARRIED: bool = cfg!(feature = "sandbox");

/// The machine-level directory this city keeps components in. Never
/// inside a city: a city carried to another machine must not carry this
/// machine's components with it (kernel-SPEC.md section 8-22, P4.02).
pub(crate) fn components_dir() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(|home| PathBuf::from(home).join(".sprawling").join("components"))
}

/// Where Firefox is on this machine, and whether it starts.
pub(crate) fn firefox() -> Presence {
    look(FIREFOX)
}

/// Where chromedriver is on this machine, and whether it starts.
pub(crate) fn chromedriver() -> Presence {
    look(CHROMEDRIVER)
}

/// Where the CPython-WASI component is, by the variable or the
/// component directory.
pub(crate) fn python_wasm() -> Presence {
    look(PYTHON_WASI)
}

/// The interpreter this platform calls a shell.
pub(crate) fn shell() -> Presence {
    look(SHELL)
}

/// The sandbox this build carries, if it carries one.
///
/// # Errors
/// Propagates what starting the engine reports. A build that says it
/// carries one and cannot start it refuses the dispatch rather than
/// falling back: falling back is how a run that a person believed was
/// sandboxed turns out not to have been.
#[cfg(feature = "sandbox")]
pub(crate) fn execution_engine() -> Result<Box<dyn runtime::Sandbox>, AxError> {
    Ok(Box::new(runtime::WasmtimeSandbox::new()?))
}

/// The engine `exec` runs a program in: none, in a build without one.
///
/// # Errors
/// None today; the signature matches the arm that can fail so the call
/// site does not change shape with the feature.
#[cfg(not(feature = "sandbox"))]
pub(crate) fn execution_engine() -> Result<Box<dyn runtime::Sandbox>, AxError> {
    Ok(Box::new(runtime::AbsentSandbox))
}

/// One item of the table, asked of this machine.
///
/// A name the table does not carry is a programming error rather than a
/// machine fact; it is answered as absent so a caller still gets a
/// typed answer, and the tests hold every name above to a row.
fn look(name: &str) -> Presence {
    let machine = ThisMachine::new(Platform::current(), PATIENCE);
    REQUIREMENTS
        .iter()
        .find(|requirement| requirement.name == name)
        .map_or(
            Presence::Absent(super::Absence::NotOnSearchPath),
            |requirement| machine.look(requirement),
        )
}
