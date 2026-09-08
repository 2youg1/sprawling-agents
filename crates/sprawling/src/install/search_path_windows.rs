// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Editing the user search path on Windows: the registry value, read
//! and written under its own type, and the broadcast that tells the
//! desktop it moved.

use super::{PathEdit, PathOutcome, PathRemoval, plan_append, plan_remove};
use kernel::{AxCode, AxError};

/// Reads `HKCU\Environment\Path` without expanding it, and says which
/// registry type it has.
///
/// The raw value is what has to be rewritten: expanding `%VAR%` and
/// writing the result back is how a search path silently stops
/// following the variables a person put in it.
const READ: &str = r"
$ErrorActionPreference='Stop'
$k = Get-Item -LiteralPath 'HKCU:\Environment'
if ($k.GetValueNames() -contains 'Path') {
  $kind = $k.GetValueKind('Path')
  $v = $k.GetValue('Path','',[Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
} else { $kind = 'ExpandString'; $v = '' }
[IO.File]::WriteAllText($env:SPRAWLING_PATH_FILE, $v, (New-Object Text.UTF8Encoding $false))
Write-Output $kind
";

/// Writes the value back under the type it was read as, then tells
/// every top-level window that the environment moved.
///
/// `[Environment]::SetEnvironmentVariable` is the obvious call and
/// the wrong one: it always writes REG_SZ, which demotes a
/// REG_EXPAND_SZ search path so its `%VAR%` entries stop expanding
/// (dotnet/runtime#1442). Writing the registry directly means the
/// broadcast is ours to send, and `#![forbid(unsafe_code)]` puts
/// `SendMessageTimeout` out of Rust's reach - so it is sent from
/// here.
const WRITE: &str = r#"
$ErrorActionPreference='Stop'
$v = [IO.File]::ReadAllText($env:SPRAWLING_PATH_FILE, (New-Object Text.UTF8Encoding $false))
Set-ItemProperty -LiteralPath 'HKCU:\Environment' -Name 'Path' -Value $v -Type $env:SPRAWLING_PATH_KIND
Add-Type -Namespace SprawlingNative -Name Env -MemberDefinition @'
[DllImport("user32.dll", SetLastError=true, CharSet=CharSet.Auto)]
public static extern IntPtr SendMessageTimeout(IntPtr hWnd, uint Msg, UIntPtr wParam, string lParam, uint fuFlags, uint uTimeout, out UIntPtr lpdwResult);
'@
$r = [UIntPtr]::Zero
[void][SprawlingNative.Env]::SendMessageTimeout([IntPtr]0xffff, 0x1A, [UIntPtr]::Zero, 'Environment', 2, 5000, [ref]$r)
"#;

struct Raw {
    value: String,
    kind: String,
    carrier: std::path::PathBuf,
}

fn carrier_path() -> std::path::PathBuf {
    std::env::temp_dir().join(format!("sprawling-path-{}.txt", std::process::id()))
}

fn powershell(
    script: &str,
    carrier: &std::path::Path,
    kind: Option<&str>,
) -> Result<String, AxError> {
    let mut command = std::process::Command::new("powershell");
    command
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            script,
        ])
        .env("SPRAWLING_PATH_FILE", carrier);
    if let Some(kind) = kind {
        command.env("SPRAWLING_PATH_KIND", kind);
    }
    let done = command.output().map_err(|err| {
        AxError::failure(AxCode::StorageFatal, "run powershell", err.to_string())
            .with_recovery("powershell is how a user-level PATH is edited on this platform")
    })?;
    if !done.status.success() {
        return Err(AxError::failure(
            AxCode::StorageFatal,
            "edit the user search path",
            String::from_utf8_lossy(&done.stderr).trim().to_owned(),
        )
        .with_recovery("no change was made; edit PATH in System Properties instead"));
    }
    Ok(String::from_utf8_lossy(&done.stdout).trim().to_owned())
}

fn read() -> Result<Raw, AxError> {
    let carrier = carrier_path();
    let kind = powershell(READ, &carrier, None)?;
    let value = std::fs::read_to_string(&carrier).map_err(|err| {
        AxError::failure(
            AxCode::StorageFatal,
            "read the user search path",
            err.to_string(),
        )
        .with_recovery("no change was made")
    })?;
    Ok(Raw {
        value,
        kind,
        carrier,
    })
}

fn write(raw: &Raw, value: &str) -> Result<Option<String>, AxError> {
    std::fs::write(&raw.carrier, value).map_err(|err| {
        AxError::failure(
            AxCode::StorageFatal,
            "stage the new search path",
            err.to_string(),
        )
        .with_recovery("no change was made")
    })?;
    let outcome = powershell(WRITE, &raw.carrier, Some(&raw.kind));
    // The carrier held nothing secret, but it held the whole of this
    // person's search path, so it does not outlive the edit.
    let _ = std::fs::remove_file(&raw.carrier);
    outcome?;
    Ok(None)
}

pub(super) fn extend(dir: &str) -> Result<(PathOutcome, Option<String>), AxError> {
    let raw = read()?;
    match plan_append(&raw.value, dir) {
        PathEdit::AlreadyPresent => {
            let _ = std::fs::remove_file(&raw.carrier);
            Ok((PathOutcome::Unchanged, None))
        }
        PathEdit::Append(next) => {
            let notice = write(&raw, &next)?;
            Ok((PathOutcome::Rewritten, notice))
        }
    }
}

pub(super) fn retract(dir: &str) -> Result<PathOutcome, AxError> {
    let raw = read()?;
    match plan_remove(&raw.value, dir) {
        PathRemoval::Absent => {
            let _ = std::fs::remove_file(&raw.carrier);
            Ok(PathOutcome::Unchanged)
        }
        PathRemoval::Rewrite(next) => {
            write(&raw, &next)?;
            Ok(PathOutcome::Rewritten)
        }
    }
}
