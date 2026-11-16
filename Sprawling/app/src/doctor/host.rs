// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::ffi::OsString;
use std::path::PathBuf;

use kernel::AxError;

use super::family::GECKO;
use super::table::{
    CHROMEDRIVER, MSEDGEDRIVER, PWSH, PYTHON_GUEST_B3, PYTHON_WASI, PYTHON_WASM_VARIABLE,
    REQUIREMENTS, SHELL,
};
use super::{Absence, Fault, Machine, PATIENCE, Platform, Presence, ThisMachine, Version};

pub(crate) const ENGINE_CARRIED: bool = cfg!(feature = "sandbox");

static PYTHON_GUEST_CHECK: std::sync::OnceLock<Result<(), Fault>> = std::sync::OnceLock::new();

pub(crate) fn components_dir() -> Option<PathBuf> {
    match accounting::home::Home::detect() {
        Ok(home) => Some(home.components()),
        Err(_) => None,
    }
}

const USER_BINS: [&str; 4] = [".cargo/bin", ".elan/bin", ".local/bin", ".bun/bin"];

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

    match std::env::join_paths(dirs) {
        Ok(joined) => joined,
        Err(_unjoinable) => inherited,
    }
}

pub(crate) fn find_program(program: &str) -> Option<PathBuf> {
    super::on_search_path(&search_path(), program)
}

pub(crate) fn place_set_up(dir: &agent_protocols::SetUpDir) -> Option<PathBuf> {

    let variable = dir.variable.and_then(std::env::var_os);
    match accounting::home::Home::detect() {
        Ok(home) => dir.on(Some(home.path()), variable),
        Err(_no_home) => dir.on(None, variable),
    }
}

pub(crate) fn install_log(item: &str) -> PathBuf {
    std::env::temp_dir()
        .join("sprawling-install")
        .join(format!("{item}.log"))
}

pub(crate) fn firefox() -> Presence {
    look(GECKO)
}

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

pub(crate) fn python_wasm() -> Presence {
    look(PYTHON_WASI)
}

pub(super) fn python_component(
    variable: Option<OsString>,
    dir: Option<PathBuf>,
    file: &str,
) -> Presence {
    let source = variable
        .as_ref()
        .filter(|value| !value.is_empty())
        .map_or(PythonGuestSource::Pinned, |_| PythonGuestSource::Override);
    let found = super::probe::component_at(PYTHON_WASM_VARIABLE, variable, dir, file);
    judge_python_guest(found, source, PYTHON_GUEST_B3, &PYTHON_GUEST_CHECK)
}

#[derive(Clone, Copy)]
pub(super) enum PythonGuestSource {
    Pinned,
    Override,
}

pub(super) fn judge_python_guest(
    found: Presence,
    source: PythonGuestSource,
    expected: &str,
    checked: &std::sync::OnceLock<Result<(), Fault>>,
) -> Presence {
    let Presence::Present { at, version } = found else {
        return found;
    };
    if matches!(source, PythonGuestSource::Override) {
        return Presence::Present {
            at,
            version: Version::Said("unpinned override".to_owned()),
        };
    }
    match checked
        .get_or_init(|| verify_python_guest(&at, expected))
        .clone()
    {
        Ok(()) => {
            let lib = runtime::PythonGuest::at(at.clone())
                .stdlib()
                .host
                .join("python3.14");
            let installed = [
                "encodings/__init__.py",
                "collections/__init__.py",
                "difflib.py",
                "re/__init__.py",
                "os.py",
            ]
            .iter()
            .all(|name| lib.join(name).is_file());
            if installed {
                Presence::Present { at, version }
            } else {
                Presence::Broken {
                    at,
                    fault: Fault::HalfWritten,
                }
            }
        }
        Err(fault) => Presence::Broken { at, fault },
    }
}

fn verify_python_guest(at: &std::path::Path, expected: &str) -> Result<(), Fault> {
    let expected = expected.parse::<kernel::B3Hash>().map_err(|why| {
        Fault::Unreadable(format!("the registered Python guest pin is invalid: {why}"))
    })?;
    let bytes = std::fs::read(at)
        .map_err(|why| Fault::Unreadable(format!("read pinned guest {}: {why}", at.display())))?;
    let found = kernel::B3Hash::digest(&bytes);
    if found == expected {
        Ok(())
    } else {
        Err(Fault::DigestMismatch { expected, found })
    }
}

pub(crate) fn shell() -> Presence {
    look(SHELL)
}

pub(crate) fn pwsh() -> Presence {
    look(PWSH)
}

pub(crate) fn usable_pwsh() -> Option<PathBuf> {
    seven_or_later(&pwsh())
}

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

pub fn usable_python_wasm() -> Option<PathBuf> {
    usable_path(&python_wasm())
}

pub(crate) fn usable_shell() -> Option<PathBuf> {
    usable_path(&shell())
}

fn usable_path(presence: &Presence) -> Option<PathBuf> {
    if !presence.usable() {
        return None;
    }
    presence.at().map(std::path::Path::to_path_buf)
}

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

#[cfg(feature = "sandbox")]
pub(crate) fn execution_engine() -> Result<Box<dyn runtime::Sandbox>, AxError> {
    static ENGINE: std::sync::Mutex<Option<runtime::WasmtimeSandbox>> = std::sync::Mutex::new(None);
    let mut slot = ENGINE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(engine) = slot.as_ref() {
        return Ok(Box::new(engine.clone()));
    }
    let engine = runtime::WasmtimeSandbox::new()?;
    *slot = Some(engine.clone());
    Ok(Box::new(engine))
}

#[cfg(not(feature = "sandbox"))]
pub(crate) fn execution_engine() -> Result<Box<dyn runtime::Sandbox>, AxError> {
    Ok(Box::new(runtime::AbsentSandbox))
}

fn look(name: &str) -> Presence {
    let machine = ThisMachine::new(Platform::current(), PATIENCE);
    REQUIREMENTS
        .iter()
        .find(|requirement| requirement.name == name)
        .map_or(Presence::Absent(Absence::NotOnSearchPath), |requirement| {
            machine.look(requirement)
        })
}

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
