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

use super::family::GECKO;
use super::table::{CHROMEDRIVER, MSEDGEDRIVER, PYTHON_WASI, REQUIREMENTS, SHELL};
use super::{Absence, Machine, PATIENCE, Platform, Presence, ThisMachine};

/// Whether this binary was built with the `sandbox` feature. The one
/// spelling of that fact; the table reads it and the engine below
/// selects on it.
pub(crate) const ENGINE_CARRIED: bool = cfg!(feature = "sandbox");

/// The machine-level directory this city keeps components in. Never
/// inside a city: a city carried to another machine must not carry this
/// machine's components with it (kernel-SPEC.md section 8-22).
pub(crate) fn components_dir() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(|home| PathBuf::from(home).join(".sprawling").join("components"))
}

/// The Gecko browser this machine has, whichever brand it is: Firefox,
/// Zen, LibreWolf, Waterfox, Floorp or another fork (`doctor::family`).
/// `SPRAWLING_BROWSER` names one over all of them.
///
/// The engine takes the path out of this answer, so the browser a
/// person was told about and the browser a run starts are one program.
pub(crate) fn firefox() -> Presence {
    look(GECKO)
}

/// The Chromium driver this machine has - `chromedriver` for Chrome,
/// Brave, Chromium and Vivaldi, `msedgedriver` for Edge - and whether
/// it starts. Either one is a way into a Chromium session, so the first
/// that answers is the answer.
pub(crate) fn chromedriver() -> Presence {
    let driver = look(CHROMEDRIVER);
    match driver.usable() {
        true => driver,
        false => match look(MSEDGEDRIVER) {
            second if second.usable() => second,
            _ => driver,
        },
    }
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
        .map_or(Presence::Absent(Absence::NotOnSearchPath), |requirement| {
            machine.look(requirement)
        })
}
