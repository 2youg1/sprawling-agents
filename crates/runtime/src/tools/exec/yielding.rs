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
//! file needs no foreign call (`crates/runtime/spec/Tools/Exec.lean` §8-13-3).

use std::process::Command;

#[cfg(unix)]
use kernel::AxCode;
use kernel::AxError;

use crate::backlog::Shares;

/// `BELOW_NORMAL_PRIORITY_CLASS` from `WinBase.h`: an absolute class,
/// one step under the class a process gets when nobody chose one.
#[cfg(windows)]
const BELOW_NORMAL_PRIORITY_CLASS: u32 = 0x0000_4000;

/// How many niceness units a dispatched command sits under the core.
/// The increment is relative, so it holds whatever the core runs at, and
/// lowering a priority never needs a privilege.
#[cfg(unix)]
const NICENESS_BELOW_THE_CORE: &str = "10";

/// The IO level a dispatched command gets on Linux: the lowest of the
/// best-effort class, which queues behind the core without the idle
/// class's risk of reading nothing while the disk is busy.
#[cfg(target_os = "linux")]
const IO_BELOW_THE_CORE: [&str; 4] = ["-c", "2", "-n", "7"];

/// The command, set to start below the core's priority.
#[cfg(windows)]
///
/// # Errors
///
/// None on Windows: a missing program still fails at spawn, where the
/// backlog reports it; the `Result` is the Unix arm's.
pub(super) fn one_level_down(mut command: Command, _shares: Shares) -> Result<Command, AxError> {
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
        .with_recovery("check the program name, or use the shell arm"));
    }
    Ok(())
}

/// The wrapper a dispatched program starts under: `nice`, and on Linux
/// `ionice` around it when the core's `PATH` has it. The IO half is the
/// lighter of the two, so a host without util-linux keeps the CPU half
/// instead of failing every command at spawn.
#[cfg(target_os = "linux")]
fn start_below_the_core(_shares: Shares) -> Command {
    if is_executable_on_path(std::ffi::OsStr::new("ionice"), None) {
        let mut both = Command::new("ionice");
        both.args(IO_BELOW_THE_CORE).arg("nice");
        both.args(["-n", NICENESS_BELOW_THE_CORE, "--"]);
        return both;
    }
    nice_below_the_core()
}

/// Where macOS keeps `taskpolicy`, which starts a program with its QoS
/// clamped: utility work is placed on the efficiency cores first, on
/// Apple silicon and Intel Macs alike (`crates/runtime/spec/Tools/Exec.lean` D29).
#[cfg(target_os = "macos")]
const TASKPOLICY: &str = "/usr/sbin/taskpolicy";

/// The wrapper a dispatched program starts under on macOS: `nice`, and
/// `taskpolicy -c utility` around it when the run is to share the
/// processors (macOS's CPU share, D29) and the system has it; without it
/// the command keeps the CPU half rather than failing at spawn.
#[cfg(target_os = "macos")]
fn start_below_the_core(shares: Shares) -> Command {
    let shared = match shares {
        Shares::Unset => false,
        Shares::Cpu | Shares::CpuAndMemory { .. } => true,
    };
    if shared && is_executable_on_path(std::ffi::OsStr::new(TASKPOLICY), None) {
        let mut both = Command::new(TASKPOLICY);
        both.args(["-c", "utility", "nice"]);
        both.args(["-n", NICENESS_BELOW_THE_CORE, "--"]);
        return both;
    }
    nice_below_the_core()
}

/// The wrapper a dispatched program starts under: `nice`.
#[cfg(all(unix, not(any(target_os = "linux", target_os = "macos"))))]
fn start_below_the_core(_shares: Shares) -> Command {
    nice_below_the_core()
}

/// `--` ends `nice`'s options, so a program whose name starts with a
/// dash is started rather than read as an option of `nice`.
#[cfg(unix)]
fn nice_below_the_core() -> Command {
    let mut nice = Command::new("nice");
    nice.args(["-n", NICENESS_BELOW_THE_CORE, "--"]);
    nice
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
