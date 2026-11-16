// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxCode, AxError};

const ANNOUNCE: &str = r#"
$ErrorActionPreference='Stop'
Add-Type -Namespace SprawlingNative -Name Env -MemberDefinition @'
[DllImport("user32.dll", SetLastError=true, CharSet=CharSet.Auto)]
public static extern IntPtr SendMessageTimeout(IntPtr hWnd, uint Msg, UIntPtr wParam, string lParam, uint fuFlags, uint uTimeout, out UIntPtr lpdwResult);
'@
$r = [UIntPtr]::Zero
[void][SprawlingNative.Env]::SendMessageTimeout([IntPtr]0xffff, 0x1A, [UIntPtr]::Zero, 'Environment', 2, 5000, [ref]$r)
"#;

pub fn announce() -> Result<(), AxError> {
    use crate::doctor::asking::{self, Ended};

    let mut command = child::command(crate::privacy::windows::powershell()?);
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

#[cfg(test)]
mod tests {
    #[test]
    fn the_announcing_script_parses() {
        assert!(crate::privacy::windows::parses(super::ANNOUNCE));
    }
}
