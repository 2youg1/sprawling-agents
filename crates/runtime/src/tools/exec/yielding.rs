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
pub(super) fn one_level_down(mut command: Command) -> Command {
    use std::os::windows::process::CommandExt;
    command.creation_flags(BELOW_NORMAL_PRIORITY_CLASS);
    command
}

/// The command, run under `nice` so that it starts below the core's
/// priority. The working directory and every variable move to the outer
/// command, which is the one that is spawned.
#[cfg(unix)]
pub(super) fn one_level_down(command: Command) -> Command {
    let mut lowered = Command::new("nice");
    lowered
        .arg("-n")
        .arg(NICENESS_BELOW_THE_CORE)
        .arg(command.get_program())
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
    lowered
}
