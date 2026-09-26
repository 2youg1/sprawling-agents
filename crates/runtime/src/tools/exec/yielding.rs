// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The priority a dispatched command starts at: one level below the core.
//!
//! A build or a test an agent starts competes for the same cores as the
//! harness that accounts for it, shows it, and serves its socket; the
//! harness has to win that contest, so every host command the exec tool
//! dispatches starts one level down instead of inheriting the core's
//! level. Both platforms are reached through a safe interface, so this
//! file needs no foreign call (runtime-SPEC §8-13-3).

use std::process::Command;

#[cfg(unix)]
use kernel::AxCode;
use kernel::AxError;

/// `BELOW_NORMAL_PRIORITY_CLASS` from `WinBase.h`: an absolute class,
/// one step under the class a process gets when nobody chose one.
#[cfg(windows)]
const BELOW_NORMAL_PRIORITY_CLASS: u32 = 0x0000_4000;

/// How many niceness units a dispatched command sits under the core.
/// The increment is relative, so it holds whatever the core runs at, and
/// lowering a priority never needs a privilege.
#[cfg(unix)]
const NICENESS_BELOW_THE_CORE: &str = "10";

/// The command, set to start below the core's priority.
#[cfg(windows)]
///
/// # Errors
///
/// None on Windows: a missing program still fails at spawn, where the
/// backlog reports it; the `Result` is the Unix arm's.
pub(super) fn one_level_down(mut command: Command) -> Result<Command, AxError> {
    use std::os::windows::process::CommandExt;
    command.creation_flags(BELOW_NORMAL_PRIORITY_CLASS);
    Ok(command)
}

/// The command, run under `nice` so that it starts below the core's
/// priority. The working directory and every variable move to the outer
/// command, which is the one that is spawned.
///
/// # Errors
///
/// `E_TOOL_UNAVAILABLE` when the program is not an executable file on the
/// core's `PATH` (or at the path it names): `nice` itself would still
/// start, and the missing program would reach the caller only as exit 127
/// with the reason in stderr.
#[cfg(unix)]
pub(super) fn one_level_down(command: Command) -> Result<Command, AxError> {
    let program = command.get_program();
    if !is_executable_on_path(program, command.get_current_dir()) {
        return Err(AxError::failure(
            AxCode::ToolUnavailable,
            "start a command",
            format!(
                "{}: no executable file of that name on PATH",
                std::path::Path::new(program).display()
            ),
        )
        .with_recovery("check the program name, or use the shell arm"));
    }
    let mut lowered = Command::new("nice");
    lowered
        .arg("-n")
        .arg(NICENESS_BELOW_THE_CORE)
        .arg(program)
        .args(command.get_args());
    if let Some(dir) = command.get_current_dir() {
        lowered.current_dir(dir);
    }
    for (name, value) in command.get_envs() {
        match value {
            Some(value) => lowered.env(name, value),
            None => lowered.env_remove(name),
        };
    }
    Ok(lowered)
}

/// Whether `program` names an executable file the way `execvp` finds it:
/// a name with a separator is taken relative to the working directory, a
/// bare name is searched on the core's `PATH`. A file that cannot be read
/// is not executable, which is the answer the spawn would give.
#[cfg(unix)]
fn is_executable_on_path(program: &std::ffi::OsStr, dir: Option<&std::path::Path>) -> bool {
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;
    let executable = |candidate: &Path| {
        std::fs::metadata(candidate)
            .is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
    };
    let named = Path::new(program);
    if named.components().count() > 1 {
        return executable(&dir.map_or_else(|| named.to_path_buf(), |dir| dir.join(named)));
    }
    std::env::var_os("PATH").is_some_and(|path| {
        std::env::split_paths(&path).any(|entry| executable(&entry.join(named)))
    })
}
