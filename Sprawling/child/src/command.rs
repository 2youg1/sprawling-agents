// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::ffi::OsStr;
use std::process::Command;

use crate::environment::scrubbed;

#[cfg(windows)]
pub const NO_WINDOW: u32 = 0x0800_0000;

#[must_use]
#[expect(
    clippy::disallowed_methods,
    reason = "the one constructor every other spawn point is sent to"
)]
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
