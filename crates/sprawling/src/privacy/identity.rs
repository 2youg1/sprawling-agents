// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Read-only principal sampling (`crates/sprawling/spec/Privacy/Cli.lean`).

use kernel::{AxCode, AxError};
use zeroize::Zeroizing;

#[cfg(windows)]
pub(super) fn read() -> Result<Zeroizing<String>, AxError> {
    use crate::doctor::asking::{self, Ended};
    use winreg::RegKey;
    use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_QUERY_VALUE, KEY_WOW64_64KEY};

    let installation = RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey_with_flags(
            r"SOFTWARE\Microsoft\Windows NT\CurrentVersion",
            KEY_QUERY_VALUE | KEY_WOW64_64KEY,
        )
        .and_then(|key| key.get_value::<std::ffi::OsString, _>("SystemRoot"))
        .map_err(|_| refused("protected Windows installation path unavailable"))?;
    let executable = std::path::PathBuf::from(installation)
        .join(r"System32\WindowsPowerShell\v1.0\powershell.exe");
    if !executable.is_absolute() {
        return Err(refused("Windows installation path is not absolute"));
    }
    let mut command = std::process::Command::new(executable);
    command.args([
        "-NoLogo", "-NoProfile", "-NonInteractive", "-Command",
        "$ErrorActionPreference = 'Stop'; [Console]::OutputEncoding = [Text.Encoding]::UTF8; [Security.Principal.WindowsIdentity]::GetCurrent().User.Value",
    ]);
    let knocks = u32::try_from(
        crate::doctor::PATIENCE
            .as_millis()
            .checked_div(asking::TICK.as_millis())
            .ok_or_else(|| refused("identity query tick is zero"))?,
    )
    .map_err(|_| refused("identity query patience cannot be represented"))?;
    let mut first = true;
    match asking::ask(&mut command, knocks, move |_| std::mem::take(&mut first)) {
        Ended::Exited {
            code: Some(0),
            kept,
        } => {
            let value = Zeroizing::new(kept);
            if !value.starts_with("S-1-")
                || !value
                    .bytes()
                    .all(|byte| byte == b'S' || byte == b'-' || byte.is_ascii_digit())
            {
                return Err(refused(
                    "Windows principal query returned no valid identity",
                ));
            }
            Ok(value)
        }
        Ended::Exited {
            code: Some(_) | None,
            ..
        }
        | Ended::Unstarted
        | Ended::Unanswered { .. } => Err(refused("Windows principal query failed")),
    }
}

#[cfg(not(windows))]
pub(super) fn read() -> Result<Zeroizing<String>, AxError> {
    Err(refused(
        "Windows privacy controls are unavailable on this platform",
    ))
}

fn refused(subject: &str) -> AxError {
    AxError::failure(AxCode::ToolUnavailable, "read privacy principal", subject).with_recovery(
        "run on Windows with a readable system installation; leave privacy history unchanged",
    )
}
