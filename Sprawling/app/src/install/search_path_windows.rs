// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::{PathEdit, PathOutcome, PathRemoval, plan_append, plan_remove};
use kernel::{AxCode, AxError};

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

const WRITE: &str = r#"
$ErrorActionPreference='Stop'
$v = [IO.File]::ReadAllText($env:SPRAWLING_PATH_FILE, (New-Object Text.UTF8Encoding $false))
Set-ItemProperty -LiteralPath 'HKCU:\Environment' -Name 'Path' -Value $v -Type $env:SPRAWLING_PATH_KIND
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
    let mut command = child::command("powershell");
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

    drop(std::fs::remove_file(&raw.carrier));
    outcome?;

    Ok(match sprawling::environment_broadcast::announce() {
        Ok(()) => None,
        Err(unannounced) => Some(format!(
            "the search path is written, but {}; windows opened after you sign out and back in will see it",
            unannounced.subject()
        )),
    })
}

pub(super) fn extend(dir: &str) -> Result<(PathOutcome, Option<String>), AxError> {
    let raw = read()?;
    match plan_append(&raw.value, dir) {
        PathEdit::AlreadyPresent => {
            drop(std::fs::remove_file(&raw.carrier));
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
            drop(std::fs::remove_file(&raw.carrier));
            Ok(PathOutcome::Unchanged)
        }
        PathRemoval::Rewrite(next) => {
            write(&raw, &next)?;
            Ok(PathOutcome::Rewritten)
        }
    }
}
