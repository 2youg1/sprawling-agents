// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use kernel::AxError;

use super::family::GECKO;
use super::table::{CHROMEDRIVER, MSEDGEDRIVER, PWSH, PYTHON, REQUIREMENTS, SHELL};
use super::{Absence, Machine, PATIENCE, Platform, Presence, ThisMachine};

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

pub fn usable_python() -> Option<PathBuf> {
    usable_path(&look(PYTHON))
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

pub(crate) fn usable_shell() -> Option<PathBuf> {
    usable_path(&shell())
}

fn usable_path(presence: &Presence) -> Option<PathBuf> {
    if !presence.usable() {
        return None;
    }
    presence.at().map(Path::to_path_buf)
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
                    let harness = std::env::current_exe().map_err(|err| {
                        AxError::failure(
                            kernel::AxCode::SandboxDenied,
                            "locate the container guardian",
                            err.to_string(),
                        )
                        .with_recovery(
                            "start the installed harness executable so its cleanup helper can be launched",
                        )
                    })?;
                    return Ok(runtime.guarded(harness));
                }
                Err(err) => failures.push(err.to_string()),
            }
        }
    }
    Err(AxError::failure(
        kernel::AxCode::SandboxDenied,
        "probe the container daemon",
        format!(
            "no accessible Linux cgroup v2 container daemon: {}",
            failures.join("; ")
        ),
    )
    .with_recovery(
        "install and start Docker or Podman with CPU, memory and pids controllers; prepare a local immutable image",
    ))
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
                let wrapper = find_program("bwrap").ok_or_else(|| {
                    AxError::failure(
                        kernel::AxCode::SandboxDenied,
                        "select the native sandbox",
                        "bubblewrap is missing",
                    )
                    .with_recovery(
                        "install bubblewrap with unprivileged user namespaces or select another arm",
                    )
                })?;
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
