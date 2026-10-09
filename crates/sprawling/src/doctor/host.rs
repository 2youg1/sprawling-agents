// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The one door the rest of this binary asks what this machine has
//! through (`crates/sprawling/spec/Doctor.lean` §8-47).
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
use super::table::{CHROMEDRIVER, MSEDGEDRIVER, PWSH, PYTHON_WASI, REQUIREMENTS, SHELL};
use super::{Absence, Fault, Machine, PATIENCE, Platform, Presence, ThisMachine, Version};

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
/// started with (`crates/sprawling/spec/Doctor.lean` §8-58): this process's `PATH`,
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

/// Where this machine's search path finds `program`: the one reading the
/// doctor, the consent to an ACP agent and every optional program share.
pub(crate) fn find_program(program: &str) -> Option<PathBuf> {
    super::on_search_path(&search_path(), program)
}

/// Where one vendor's set-up directory, or another ACP client's file, is
/// on this machine: the variable its vendor documents when that is set,
/// else the path under the User's home (`crates/agent_protocols/Spec.lean`
/// D19).
pub(crate) fn place_set_up(dir: &agent_protocols::SetUpDir) -> Option<PathBuf> {
    // A machine with no home directory can still name a directory a
    // variable moved; a row under the home is then nowhere to look, and
    // detection looks only where it can.
    let variable = dir.variable.and_then(std::env::var_os);
    match accounting::home::Home::detect() {
        Ok(home) => dir.on(Some(home.path()), variable),
        Err(_no_home) => dir.on(None, variable),
    }
}

/// The file one item's install writes its output to, fresh for every
/// install (`crates/sprawling/spec/Doctor.lean` §8-64).
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

/// PowerShell, as `pwsh --version` answers on this machine's search path.
pub(crate) fn pwsh() -> Presence {
    look(PWSH)
}

/// The PowerShell 7 a run may be handed: a present `pwsh` whose version
/// line reads major 7 or later (`crates/runtime/Spec.lean` §8-13-2 D30).
/// A pwsh 6, and one whose version this machine could not read, is
/// `None`, because the flags and error ids the exec tool relies on are
/// PowerShell 7's.
pub(crate) fn usable_pwsh() -> Option<PathBuf> {
    seven_or_later(&pwsh())
}

/// The path of a present PowerShell whose version is 7 or later.
pub(super) fn seven_or_later(presence: &Presence) -> Option<PathBuf> {
    let Presence::Present { at, version } = presence else {
        return None;
    };
    let major = version
        .number()
        .split('.')
        .next()
        .and_then(|major| major.parse::<u32>().ok())?;
    (major >= 7).then(|| at.clone())
}

/// The CPython-WASI component a run may be handed: a present one, and
/// never a broken one (`crates/accounting/Spec.lean` §8-11).
pub(crate) fn usable_python_wasm() -> Option<PathBuf> {
    usable_path(&python_wasm())
}

/// The shell a run may be handed: a present one, and never a broken one
/// (`crates/accounting/Spec.lean` §8-11).
pub(crate) fn usable_shell() -> Option<PathBuf> {
    usable_path(&shell())
}

/// The path of an item a run may be handed. A broken one is `None`,
/// because the exec tool could not tell it from a working one.
fn usable_path(presence: &Presence) -> Option<PathBuf> {
    if !presence.usable() {
        return None;
    }
    presence.at().map(std::path::Path::to_path_buf)
}

/// Whether this build carries its engine, and whether it starts.
pub(super) fn built(carried: bool) -> Presence {
    if !carried {
        return Presence::Absent(Absence::NotInThisBuild);
    }
    let at = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("this binary"));
    match execution_engine() {
        Ok(_) => Presence::Present {
            at,
            version: Version::Said("wasmtime".to_owned()),
        },
        Err(err) => Presence::Broken {
            at,
            fault: Fault::WillNotStart(err.to_string()),
        },
    }
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

/// Finds a usable Linux daemon and retains its CLI grammar and capability evidence.
///
/// # Errors
/// Refuses when neither installed backend supplies the required cgroup v2 controllers.
pub(crate) fn container() -> Result<runtime::tools::ContainerRuntime, AxError> {
    let mut failures = Vec::new();
    for (name, engine) in [
        ("docker", runtime::tools::ContainerEngine::Docker),
        ("podman", runtime::tools::ContainerEngine::Podman),
    ] {
        if let Some(program) = find_program(name) {
            match runtime::tools::ContainerRuntime::probe(engine, program) {
                Ok(runtime) => {
                    let harness = std::env::current_exe().map_err(|err| AxError::failure(
                        kernel::AxCode::SandboxDenied, "locate the container guardian", err.to_string()
                    ).with_recovery("start the installed harness executable so its cleanup helper can be launched"))?;
                    return Ok(runtime.guarded(harness));
                }
                Err(err) => failures.push(err.to_string()),
            }
        }
    }
    Err(AxError::failure(kernel::AxCode::SandboxDenied, "probe the container daemon",
        format!("no accessible Linux cgroup v2 container daemon: {}", failures.join("; ")))
        .with_recovery("install and start Docker or Podman with CPU, memory and pids controllers; prepare a local immutable image"))
}

/// Constructs a named platform boundary without falling back to a weaker arm.
///
/// # Errors
/// Refuses a missing native mechanism; the caller supplies the other named arms directly.
pub(crate) fn confinement(arm: kernel::SandboxArm) -> Result<runtime::tools::Confined, AxError> {
    use runtime::tools::{Confined, Confinement};
    let scratch = std::env::temp_dir();
    let selected = match arm {
        kernel::SandboxArm::CopiedTree => Confinement::CopiedTree,
        kernel::SandboxArm::Native => {
            #[cfg(windows)]
            {
                Confinement::WindowsJobObject
            }
            #[cfg(target_os = "macos")]
            {
                Confinement::MacosSeatbelt {
                    wrapper: PathBuf::from("/usr/bin/sandbox-exec"),
                }
            }
            #[cfg(not(any(windows, target_os = "macos")))]
            {
                let wrapper = find_program("bwrap").ok_or_else(|| AxError::failure(kernel::AxCode::SandboxDenied,
                    "select the native sandbox", "bubblewrap is missing")
                    .with_recovery("install bubblewrap with unprivileged user namespaces or select another arm"))?;
                Confinement::LinuxNamespaces { wrapper }
            }
        }
        kernel::SandboxArm::None | kernel::SandboxArm::Container | kernel::SandboxArm::Python => {
            return Err(AxError::failure(
                kernel::AxCode::ConfigInvalid,
                "construct platform confinement",
                "this arm has a different execution path",
            )
            .with_recovery("use the selected arm's execution constructor"));
        }
    };
    Ok(Confined::with_arm(selected, Some(scratch)))
}
