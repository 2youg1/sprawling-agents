// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Windows native exec, specified by
//! `crates/runtime/spec/Tools/Exec/NativeWindows.lean`.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::num::{NonZeroU16, NonZeroUsize};
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::backlog::Shares;
use desktop_ffi::confinement::{Launch, OwnedProcess};
use kernel::{AxCode, AxError};

/// Native CPU cap is independent of the optional User memory ceiling.
const DEFAULT_CPU_RATE: u16 = 5_000;
const REPARSE_POINT_ATTRIBUTE: u32 = 0x400;

pub(crate) struct Limits {
    pub(crate) memory: Option<NonZeroUsize>,
    pub(crate) cpu_rate: NonZeroU16,
}

pub(crate) fn limits(shares: Shares) -> Result<Limits, AxError> {
    let memory = match shares {
        Shares::Unset | Shares::Cpu => None,
        Shares::CpuAndMemory { limit } => Some(
            NonZeroUsize::new(
                usize::try_from(limit.get())
                    .map_err(|err| denied("read native memory limit", err))?,
            )
            .ok_or_else(|| denied("read native memory limit", "zero bytes"))?,
        ),
    };
    Ok(Limits {
        memory,
        cpu_rate: NonZeroU16::new(DEFAULT_CPU_RATE)
            .ok_or_else(|| denied("read native CPU limit", "zero rate"))?,
    })
}

/// The caller holds the backlog table lock and therefore the parent Job until
/// the new process has been placed into the same table.
pub(crate) struct LaunchFailure {
    pub(crate) error: AxError,
    pub(crate) resources: Option<Box<OwnedProcess>>,
}

impl From<AxError> for LaunchFailure {
    fn from(error: AxError) -> Self {
        Self {
            error,
            resources: None,
        }
    }
}

pub(crate) fn launch(
    mut command: Command,
    output: &Path,
    limits: &Limits,
    parent_job: NonZeroUsize,
) -> Result<OwnedProcess, LaunchFailure> {
    let directory = command
        .get_current_dir()
        .ok_or_else(|| denied("start native command", "no disposable working directory"))?
        .to_path_buf();
    let program = resolved(&command, &directory)?;
    let root = desktop_ffi::confinement::windows_root()
        .map_err(|error| denied("read Windows root", error))?;
    for name in ["SystemRoot", "windir"] {
        command.env(name, &root);
    }
    for name in ["USERPROFILE", "APPDATA", "LOCALAPPDATA", "TEMP", "TMP"] {
        command.env(name, &directory);
    }
    let environment: BTreeMap<OsString, OsString> = command
        .get_envs()
        .filter_map(|(name, value)| value.map(|value| (name.to_os_string(), value.to_os_string())))
        .collect();
    let profile = output
        .file_name()
        .ok_or_else(|| denied("name native profile", "output directory has no name"))?;
    let mut toolchain_roots = environment
        .iter()
        .filter(|(name, _)| {
            name.eq_ignore_ascii_case("CARGO_HOME") || name.eq_ignore_ascii_case("RUSTUP_HOME")
        })
        .map(|(_, value)| {
            PathBuf::from(value)
                .canonicalize()
                .map_err(|err| denied("resolve declared toolchain home", err))
                .and_then(ordinary_path)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let declared_roots = toolchain_roots.len();
    let mut pending = toolchain_roots.clone();
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory)
            .map_err(|err| denied("enumerate declared toolchain home", err))?
        {
            let entry = entry.map_err(|err| denied("read declared toolchain entry", err))?;
            let metadata = entry
                .metadata()
                .map_err(|err| denied("read toolchain entry type", err))?;
            if metadata.file_attributes() & REPARSE_POINT_ATTRIBUTE != 0 {
                continue;
            }
            let path = entry.path();
            if metadata.is_dir() {
                pending.push(path.clone());
            }
            toolchain_roots.push(path);
        }
    }
    let request = Launch {
        toolchain_roots,
        declared_roots,
        program,
        args: command
            .get_args()
            .map(std::ffi::OsStr::to_os_string)
            .collect(),
        directory,
        environment,
        cpu_rate: limits.cpu_rate,
        parent_job,
        profile: format!(
            "sprawling.{}.{}",
            std::process::id(),
            profile.to_string_lossy()
        ),
        stdout: output.join("out"),
        stderr: output.join("err"),
    };
    desktop_ffi::confinement::launch(&request).map_err(|mut failure| LaunchFailure {
        error: denied("start native command", &failure),
        resources: failure.resources.take(),
    })
}

fn resolved(command: &Command, directory: &Path) -> Result<PathBuf, AxError> {
    let program = Path::new(command.get_program());
    if program.is_absolute() {
        return program
            .canonicalize()
            .map_err(|err| denied("resolve native program", err))
            .and_then(ordinary_path);
    }
    if program.components().count() > 1 {
        return directory
            .join(program)
            .canonicalize()
            .map_err(|err| denied("resolve native program", err))
            .and_then(ordinary_path);
    }
    let path = command.get_envs().find_map(|(name, value)| {
        if name.eq_ignore_ascii_case("PATH") {
            value
        } else {
            None
        }
    });
    let path = path.ok_or_else(|| {
        denied(
            "resolve native program",
            "no PATH in the admitted environment",
        )
    })?;
    for folder in std::env::split_paths(path) {
        let candidate = folder.join(program);
        for candidate in [candidate.clone(), candidate.with_extension("exe")] {
            if candidate.is_file() {
                return candidate
                    .canonicalize()
                    .map_err(|err| denied("resolve native program", err))
                    .and_then(ordinary_path);
            }
        }
    }
    Err(denied(
        "resolve native program",
        format!("{} is absent from the admitted PATH", program.display()),
    ))
}

fn ordinary_path(path: PathBuf) -> Result<PathBuf, AxError> {
    let units: Vec<u16> = path.as_os_str().encode_wide().collect();
    if units.get(..4) != Some(&[92, 92, 63, 92]) {
        return Ok(path);
    }
    let units = if units.get(5) == Some(&58) {
        units
            .get(4..)
            .ok_or_else(|| denied("normalize native path", "missing DOS path"))?
            .to_vec()
    } else if units.get(4..8) == Some(&[85, 78, 67, 92]) {
        let mut ordinary = vec![92, 92];
        ordinary.extend(
            units
                .get(8..)
                .ok_or_else(|| denied("normalize native path", "missing UNC path"))?,
        );
        ordinary
    } else {
        return Err(denied(
            "normalize native path",
            "unsupported device namespace; use a DOS or UNC file path",
        ));
    };
    Ok(PathBuf::from(OsString::from_wide(&units)))
}

pub(crate) fn denied(action: &str, detail: impl std::fmt::Display) -> AxError {
    AxError::failure(AxCode::SandboxDenied, action, detail.to_string())
        .with_recovery("check Windows AppContainer/Job support, the configured limits and disposable directory permissions; retry the sandbox command")
}

#[cfg(test)]
mod child;

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "disposable native conformance test code"
)]
mod tests;

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "disposable framework initialization diagnostics"
)]
mod initialization;
