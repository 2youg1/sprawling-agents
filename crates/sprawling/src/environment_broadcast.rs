// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The one announcement that the user environment changed
//! (`crates/sprawling/spec/Install.lean` §8-9, `crates/sprawling/spec/Privacy/Windows.lean`).
//!
//! Explorer caches the environment block, and a console started from it
//! inherits the cache, so a registry write alone reaches no new window.
//! `WM_SETTINGCHANGE` with `"Environment"` tells every top-level window to
//! reload it. `unsafe_code` is forbidden in this crate, which puts
//! `SendMessageTimeout` out of Rust's reach, so Windows PowerShell under
//! the protected installation path sends it through `Add-Type` P/Invoke.
//! The search path install and the privacy environment adapter both
//! call this one function.

use kernel::{AxCode, AxError};

/// `HWND_BROADCAST`, `WM_SETTINGCHANGE`, `SMTO_ABORTIFHUNG` and a five
/// second cap, so a hung window cannot hold the announcement.
const ANNOUNCE: &str = r#"
$ErrorActionPreference='Stop'
Add-Type -Namespace SprawlingNative -Name Env -MemberDefinition @'
[DllImport("user32.dll", SetLastError=true, CharSet=CharSet.Auto)]
public static extern IntPtr SendMessageTimeout(IntPtr hWnd, uint Msg, UIntPtr wParam, string lParam, uint fuFlags, uint uTimeout, out UIntPtr lpdwResult);
'@
$r = [UIntPtr]::Zero
[void][SprawlingNative.Env]::SendMessageTimeout([IntPtr]0xffff, 0x1A, [UIntPtr]::Zero, 'Environment', 2, 5000, [ref]$r)
"#;

/// Tells every top-level window that the user environment changed.
///
/// # Errors
/// `ToolUnavailable` when Windows PowerShell cannot be named, started,
/// or does not finish within its counted wait. The registry value the
/// caller wrote is in place either way; only programs started before the
/// next sign-in may miss it.
pub fn announce() -> Result<(), AxError> {
    use crate::doctor::asking::{self, Ended};

    let mut command = std::process::Command::new(crate::privacy::windows::powershell()?);
    command.args([
        "-NoLogo",
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        ANNOUNCE,
    ]);
    match asking::ask(&mut command, crate::privacy::windows::PATIENCE, |_| false) {
        Ended::Exited { code: Some(0), .. } => Ok(()),
        Ended::Exited {
            code: Some(_) | None,
            ..
        }
        | Ended::Unstarted
        | Ended::Unanswered { .. } => Err(AxError::failure(
            AxCode::ToolUnavailable,
            "announce the environment change",
            "Windows PowerShell did not send WM_SETTINGCHANGE",
        )
        .with_recovery("the value is written; programs started after the next sign-in read it")),
    }
}
