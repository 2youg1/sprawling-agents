// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::process::Command;

#[cfg(unix)]
use kernel::AxCode;
use kernel::AxError;

use crate::backlog::Shares;

#[cfg(windows)]
const BELOW_NORMAL_PRIORITY_CLASS: u32 = 0x0000_4000;

#[cfg(unix)]
const NICENESS_BELOW_THE_CORE: &str = "10";

#[cfg(target_os = "linux")]
const IO_BELOW_THE_CORE: [&str; 4] = ["-c", "2", "-n", "7"];

#[cfg(windows)]

pub(super) fn one_level_down(mut command: Command, _shares: Shares) -> Result<Command, AxError> {
    use std::os::windows::process::CommandExt;
    command.creation_flags(child::NO_WINDOW | BELOW_NORMAL_PRIORITY_CLASS);
    Ok(command)
}

#[cfg(unix)]
pub(super) fn one_level_down(command: Command, shares: Shares) -> Result<Command, AxError> {
    require_executable(&command)?;
    let program = command.get_program();
    let mut lowered = start_below_the_core(shares);
    lowered.arg(program).args(command.get_args());
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

#[cfg(unix)]
pub(super) fn require_executable(command: &Command) -> Result<(), AxError> {
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
        .with_recovery(
            "check the program name; a shell builtin such as `cd` or `echo` is no program",
        ));
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn start_below_the_core(_shares: Shares) -> Command {
    if is_executable_on_path(std::ffi::OsStr::new("ionice"), None) {
        let mut both = child::command("ionice");
        both.args(IO_BELOW_THE_CORE).arg("nice");
        both.args(["-n", NICENESS_BELOW_THE_CORE, "--"]);
        return both;
    }
    nice_below_the_core()
}

#[cfg(target_os = "macos")]
const TASKPOLICY: &str = "/usr/sbin/taskpolicy";

#[cfg(target_os = "macos")]
fn start_below_the_core(shares: Shares) -> Command {
    let shared = match shares {
        Shares::Unset => false,
        Shares::Cpu | Shares::CpuAndMemory { .. } => true,
    };
    if shared && is_executable_on_path(std::ffi::OsStr::new(TASKPOLICY), None) {
        let mut both = child::command(TASKPOLICY);
        both.args(["-c", "utility", "nice"]);
        both.args(["-n", NICENESS_BELOW_THE_CORE, "--"]);
        return both;
    }
    nice_below_the_core()
}

#[cfg(all(unix, not(any(target_os = "linux", target_os = "macos"))))]
fn start_below_the_core(_shares: Shares) -> Command {
    nice_below_the_core()
}

#[cfg(unix)]
fn nice_below_the_core() -> Command {
    let mut nice = child::command("nice");
    nice.args(["-n", NICENESS_BELOW_THE_CORE, "--"]);
    nice
}

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
