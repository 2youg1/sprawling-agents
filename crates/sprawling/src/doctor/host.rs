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

use std::ffi::OsString;
use std::path::PathBuf;

use kernel::AxError;

use super::family::GECKO;
use super::table::{CHROMEDRIVER, MSEDGEDRIVER, PYTHON_WASI, REQUIREMENTS, SHELL};
use super::{Absence, Machine, PATIENCE, Platform, Presence, ThisMachine};

/// Whether this binary was built with the `sandbox` feature. The one
/// spelling of that fact; the table reads it and the engine below
/// selects on it.
pub(crate) const ENGINE_CARRIED: bool = cfg!(feature = "sandbox");

/// The machine-level directory this city keeps components in, or
/// nothing when this environment states no home directory.
///
/// The absence is an answer here rather than a failure: a probe that
/// cannot look reports the component as missing, with
/// `Absence::NoHome` carrying the reason to the report
/// (`accounting::home` owns the derivation).
pub(crate) fn components_dir() -> Option<PathBuf> {
    match accounting::home::Home::detect() {
        Ok(home) => Some(home.components()),
        Err(_) => None,
    }
}

/// The directories under a person's home that installers put programs
/// in without asking for elevation: cargo, elan, pipx and uv, and bun.
const USER_BINS: [&str; 4] = [".cargo/bin", ".elan/bin", ".local/bin", ".bun/bin"];

/// The search path every probe reads and every install program is
/// started with (sprawling-SPEC.md section 8-58): this process's `PATH`,
/// then each per-user bin directory it does not already name, and on
/// Windows the directory winget links a user-scope package into.
///
/// An installer writes the new directory into the registry or a shell
/// startup file, which this process read once, when it started; without
/// these the program rustup just installed is not found by the `cargo
/// install` on the next row, and a fresh machine needs a restarted city
/// to finish.
pub(crate) fn search_path() -> OsString {
    let inherited = std::env::var_os("PATH").unwrap_or_default();
    let mut dirs: Vec<PathBuf> = std::env::split_paths(&inherited).collect();
    let home = accounting::home::Home::detect()
        .map(|home| home.path().to_path_buf())
        .into_iter()
        .flat_map(|home| USER_BINS.map(|bin| home.join(bin)));
    let links = std::env::var_os("LOCALAPPDATA").map(|local| {
        PathBuf::from(local)
            .join("Microsoft")
            .join("WinGet")
            .join("Links")
    });
    for dir in home.chain(links) {
        if !dirs.contains(&dir) {
            dirs.push(dir);
        }
    }
    // A directory whose name holds the separator cannot be joined; the
    // inherited path is then the whole answer, as it was before any
    // directory was added.
    match std::env::join_paths(dirs) {
        Ok(joined) => joined,
        Err(_unjoinable) => inherited,
    }
}

/// Where this machine's search path finds `program`: the one reading
/// the harness page and the doctor share (accounting-SPEC.md 8-10).
pub(crate) fn find_program(program: &str) -> Option<PathBuf> {
    super::on_search_path(&search_path(), program)
}

/// The file one item's install writes its output to, fresh for every
/// install (sprawling-SPEC.md section 8-64).
///
/// Under the system's temporary directory rather than the component
/// directory, because a directory named for an item there is what
/// `Detection::Component` looks for.
pub(crate) fn install_log(item: &str) -> PathBuf {
    std::env::temp_dir()
        .join("sprawling-install")
        .join(format!("{item}.log"))
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
