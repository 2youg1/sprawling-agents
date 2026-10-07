// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The user persistent environment as raw `HKCU\Environment` values,
//! announced after each write (`crates/sprawling/spec/Privacy/Windows.lean`).
//!
//! The persistent value and a process environment are different things:
//! a program already running, and the cleaned environment sprawling's
//! own exec tool starts programs in, keep what they had. Only programs
//! started after the announcement read the new value.

use winreg::enums::HKEY_CURRENT_USER;

use super::super::target::RawValue;
use super::registry::{self, Reading, WriteFault};

/// The key Windows keeps the user persistent environment under.
const USER_ENVIRONMENT: &str = "Environment";

/// What a write of a user environment variable did.
pub(in crate::privacy) enum Written {
    /// Written, and every top-level window was told the environment moved.
    Announced,
    /// Written, but the announcement failed, so only programs started
    /// after the next sign-in are sure to read it. The value itself is
    /// in place; the readback says so.
    Unannounced(kernel::AxError),
}

/// Reads the user persistent variable `name` without expanding it.
pub(in crate::privacy) fn read(name: &str) -> Reading {
    registry::read(HKEY_CURRENT_USER, USER_ENVIRONMENT, name)
}

/// Makes the user persistent variable `name` equal `value`, then
/// announces the change.
///
/// # Errors
/// [`WriteFault`] when the registry write fails; nothing is announced
/// then.
pub(in crate::privacy) fn write(name: &str, value: &RawValue) -> Result<Written, WriteFault> {
    registry::write(HKEY_CURRENT_USER, USER_ENVIRONMENT, name, value)?;
    Ok(match crate::environment_broadcast::announce() {
        Ok(()) => Written::Announced,
        Err(failure) => Written::Unannounced(failure),
    })
}
