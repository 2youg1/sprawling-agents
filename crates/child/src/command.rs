// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The constructor every child process of the city starts from
//! (`crates/child/Spec.lean` §8, D1 and D2).

use std::ffi::OsStr;
use std::process::Command;

use crate::environment::scrubbed;

/// Windows' `CREATE_NO_WINDOW`: the child gets a console of its own with
/// no window, instead of the city's, so it cannot read the city's screen
/// buffer or keyboard input.
///
/// `creation_flags` replaces the flags rather than adding to them, so a
/// caller that sets its own (a priority class) ORs this in.
#[cfg(windows)]
pub const NO_WINDOW: u32 = 0x0800_0000;

/// A [`Command`] for `program`, ready for arguments, stdio and `spawn`.
///
/// Off the city's console on Windows, in a process group of its own on
/// Unix, and without any environment key that holds one of the city's
/// secrets. A key the caller sets with `env` afterwards is passed on: that
/// is a value the caller chose to hand over.
#[must_use]
pub fn command(program: impl AsRef<OsStr>) -> Command {
    let mut built = Command::new(program);
    detach(&mut built);
    scrubbed(built, std::env::vars_os().map(|(key, _)| key))
}

#[cfg(windows)]
fn detach(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    command.creation_flags(NO_WINDOW);
}

#[cfg(unix)]
fn detach(command: &mut Command) {
    use std::os::unix::process::CommandExt;
    command.process_group(0);
}
