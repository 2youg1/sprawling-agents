// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! UTF16 launch packet; seven terminated strings followed by an explicit
//! double-terminated environment. No process environment is inherited.
//! Argument preservation follows `Runtime.NativeWindows.Argv` in
//! `crates/runtime/spec/Tools/Exec/NativeWindows.lean`; this is the sole encoder.

use super::{Action, Failure, Launch};
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;

const BACKSLASH: u16 = 92;
const INVALID_PARAMETER: u32 = 87;

pub(super) fn encode(launch: &Launch) -> Result<Vec<u16>, Failure> {
    if launch.declared_roots > launch.toolchain_roots.len()
        || launch.cpu_rate.get() > 10_000
        || launch.profile.is_empty()
        || launch.profile.len() > 64
        || !launch
            .profile
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-')
    {
        return Err(invalid());
    }
    let mut command = quoted(launch.program.as_os_str())?;
    if launch
        .program
        .file_name()
        .and_then(OsStr::to_str)
        .is_some_and(|name| {
            name.eq_ignore_ascii_case("cmd.exe") || name.eq_ignore_ascii_case("cmd")
        })
    {
        let boundary = launch.args.iter().position(|arg| {
            arg.to_str()
                .is_some_and(|arg| arg.eq_ignore_ascii_case("/c") || arg.eq_ignore_ascii_case("/k"))
        });
        if boundary.is_some() {
            command.extend(" /D /S".encode_utf16());
        }
        for (index, part) in launch.args.iter().enumerate() {
            command.push(u16::from(b' '));
            if boundary.is_some_and(|boundary| index == boundary.saturating_add(1)) {
                command.push(u16::from(b'"'));
            }
            command.extend(terminated(part)?.into_iter().take_while(|unit| *unit != 0));
        }
        if let Some(boundary) = boundary {
            if launch.args.len() == boundary.saturating_add(1) {
                command.extend(" \"".encode_utf16());
            }
            command.push(u16::from(b'"'));
        }
    } else {
        for part in &launch.args {
            command.push(u16::from(b' '));
            command.extend(quoted(part)?);
        }
    }
    if command.len() >= 32_767 {
        return Err(invalid());
    }
    command.push(0);
    let mut packet = terminated(launch.program.as_os_str())?;
    packet.extend(command);
    for part in [
        launch.directory.as_os_str(),
        OsStr::new(&launch.profile),
        launch.stdout.as_os_str(),
        launch.stderr.as_os_str(),
    ] {
        packet.extend(terminated(part)?);
    }
    for (index, root) in launch.toolchain_roots.iter().enumerate() {
        if index != 0 {
            packet.push(10);
        }
        for unit in root.as_os_str().encode_wide() {
            if unit == 0 || unit == 10 {
                return Err(invalid());
            }
            packet.push(unit);
        }
    }
    packet.push(0);
    let mut environment = Vec::new();
    for (name, value) in &launch.environment {
        let key: Vec<u16> = name.encode_wide().collect();
        if key.is_empty() || key.iter().any(|u| *u == 0 || *u == u16::from(b'=')) {
            return Err(invalid());
        }
        environment.extend(key);
        environment.push(u16::from(b'='));
        environment.extend(terminated(value)?);
    }
    if environment.is_empty() {
        environment.push(0);
    }
    environment.push(0);
    if environment.len() > 32_767 {
        return Err(invalid());
    }
    packet.extend(environment);
    Ok(packet)
}

pub(super) fn terminated(value: &OsStr) -> Result<Vec<u16>, Failure> {
    let mut units: Vec<u16> = value.encode_wide().collect();
    if units.contains(&0) {
        return Err(invalid());
    }
    units.push(0);
    Ok(units)
}

fn quoted(value: &OsStr) -> Result<Vec<u16>, Failure> {
    let mut result = vec![u16::from(b'"')];
    let mut slashes = 0_usize;
    for unit in value.encode_wide() {
        if unit == 0 {
            return Err(invalid());
        }
        if unit == BACKSLASH {
            slashes = slashes.checked_add(1).ok_or_else(invalid)?;
            continue;
        }
        let count = if unit == u16::from(b'"') {
            slashes
                .checked_mul(2)
                .and_then(|n| n.checked_add(1))
                .ok_or_else(invalid)?
        } else {
            slashes
        };
        result.extend(std::iter::repeat_n(BACKSLASH, count));
        result.push(unit);
        slashes = 0;
    }
    result.extend(std::iter::repeat_n(
        BACKSLASH,
        slashes.checked_mul(2).ok_or_else(invalid)?,
    ));
    result.push(u16::from(b'"'));
    Ok(result)
}

fn invalid() -> Failure {
    Failure {
        action: Action::Encode,
        code: INVALID_PARAMETER,
        phase: String::new(),
        cleanup_code: None,
        resources: None,
    }
}

#[cfg(test)]
pub(super) fn valid(mut units: &[u16]) -> bool {
    for index in 0..7 {
        let Some(end) = units.iter().position(|unit| *unit == 0) else {
            return false;
        };
        if end == 0 && index != 6 {
            return false;
        }
        let Some(next) = end.checked_add(1).and_then(|n| units.get(n..)) else {
            return false;
        };
        units = next;
    }
    if units.len() < 2 || !units.ends_with(&[0, 0]) {
        return false;
    }
    let Some(environment) = units.get(..units.len().saturating_sub(1)) else {
        return false;
    };
    if environment == [0] {
        return true;
    }
    let mut remaining = environment;
    while !remaining.is_empty() {
        let Some(end) = remaining.iter().position(|unit| *unit == 0) else {
            return false;
        };
        let Some(entry) = remaining.get(..end) else {
            return false;
        };
        if entry.is_empty()
            || !entry
                .iter()
                .position(|unit| *unit == u16::from(b'='))
                .is_some_and(|equal| equal > 0)
        {
            return false;
        }
        let Some(next) = end.checked_add(1).and_then(|n| remaining.get(n..)) else {
            return false;
        };
        remaining = next;
    }
    true
}
