// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Scheduled task presence, enabled state and definition digest, and
//! enabling or disabling one task (`crates/sprawling/spec/Privacy/Windows.lean`).
//!
//! The `ScheduledTasks` module has no safe Rust binding, so Windows
//! PowerShell under the protected installation path asks it. A task is
//! named by the control table alone and reaches the script through two
//! environment variables, never through the script text, so a task name
//! with spaces or quotes needs no quoting rule. This adapter never
//! registers, deletes or edits a task: it only flips the enabled flag.

use serde::Deserialize;

use super::super::target::TaskState;

/// Reads the task `name` in folder `path`: absent, or its enabled state
/// and the SHA-256 of its exported definition with the settings'
/// `Enabled` element removed, so disabling a task leaves the digest as it
/// was and any other edit changes it. One JSON line answers.
const READ: &str = r"
$ErrorActionPreference='Stop'
[Console]::OutputEncoding=[Text.UTF8Encoding]::new($false)
$path = $env:SPRAWLING_TASK_PATH; $name = $env:SPRAWLING_TASK_NAME
$task = Get-ScheduledTask -TaskPath $path -TaskName $name -ErrorAction SilentlyContinue -ErrorVariable failure
if ($null -eq $task) {
  if ($failure.Count -eq 1 -and $failure[0].CategoryInfo.Category -eq 'ObjectNotFound') {
    [ordered]@{ state = 'absent' } | ConvertTo-Json -Compress
    exit 0
  }
  exit 3
}
$definition = [xml](Export-ScheduledTask -TaskPath $path -TaskName $name)
$flag = $definition.Task.Settings['Enabled']
if ($null -ne $flag) { [void]$definition.Task.Settings.RemoveChild($flag) }
$digest = [Security.Cryptography.SHA256]::Create().ComputeHash([Text.Encoding]::UTF8.GetBytes($definition.OuterXml))
$state = if ($task.Settings.Enabled) { 'enabled' } else { 'disabled' }
[ordered]@{ state = $state; definition_sha256 = [BitConverter]::ToString($digest).Replace('-', '').ToLowerInvariant() } | ConvertTo-Json -Compress
";

/// Enables or disables the task named as [`READ`] names it.
const SWITCH: &str = r"
$ErrorActionPreference='Stop'
$path = $env:SPRAWLING_TASK_PATH; $name = $env:SPRAWLING_TASK_NAME
if ($env:SPRAWLING_TASK_SWITCH -eq 'enable') {
  [void](Enable-ScheduledTask -TaskPath $path -TaskName $name)
} elseif ($env:SPRAWLING_TASK_SWITCH -eq 'disable') {
  [void](Disable-ScheduledTask -TaskPath $path -TaskName $name)
} else { exit 4 }
";

/// Which way a write flips a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::privacy) enum Switch {
    Enable,
    Disable,
}

/// Why a task could not be read or switched.
#[derive(Debug)]
pub(in crate::privacy) enum TaskFault {
    /// Windows PowerShell could not be named or started, or did not
    /// answer in its counted wait.
    Unavailable(kernel::AxError),
    /// The script ran and exited with this code: the task exists but
    /// could not be queried or switched, access denied among the causes.
    Refused(Option<i32>),
    /// The script answered with a line that is not one task state.
    Malformed,
}

/// Reads the task `name` in folder `path`.
///
/// # Errors
/// [`TaskFault`]; a fault is never read as an absent task.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the coordinator reads tasks back; until it exists only tests read"
    )
)]
pub(in crate::privacy) fn read(path: &str, name: &str) -> Result<TaskState, TaskFault> {
    decoded(&asked(READ, path, name, None)?)
}

/// Flips the task `name` in folder `path` to `switch`. Machine scope: the
/// elevated child is the caller.
///
/// # Errors
/// [`TaskFault`]; whether the flag moved is told by the readback.
pub(in crate::privacy) fn write(path: &str, name: &str, switch: Switch) -> Result<(), TaskFault> {
    let word = match switch {
        Switch::Enable => "enable",
        Switch::Disable => "disable",
    };
    asked(SWITCH, path, name, Some(word)).map(drop)
}

/// The one answer line `script` printed, after a clean exit.
fn asked(script: &str, path: &str, name: &str, switch: Option<&str>) -> Result<String, TaskFault> {
    use crate::doctor::asking::{self, Ended};

    let mut command =
        std::process::Command::new(super::powershell().map_err(TaskFault::Unavailable)?);
    command
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            script,
        ])
        .env("SPRAWLING_TASK_PATH", path)
        .env("SPRAWLING_TASK_NAME", name);
    if let Some(word) = switch {
        command.env("SPRAWLING_TASK_SWITCH", word);
    }
    let mut first = true;
    match asking::ask(&mut command, super::PATIENCE, move |_| {
        std::mem::take(&mut first)
    }) {
        Ended::Exited {
            code: Some(0),
            kept,
        } => Ok(kept),
        Ended::Exited { code, .. } => Err(TaskFault::Refused(code)),
        Ended::Unstarted | Ended::Unanswered { .. } => Err(TaskFault::Unavailable(
            kernel::AxError::failure(
                kernel::AxCode::ToolUnavailable,
                "ask Windows about a scheduled task",
                "Windows PowerShell did not answer",
            )
            .with_recovery("nothing was written; check that Windows PowerShell starts, then read the page again"),
        )),
    }
}

/// The script's answer: a closed set of states, unknown fields refused.
#[derive(Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
enum Answer {
    Absent {},
    Enabled { definition_sha256: String },
    Disabled { definition_sha256: String },
}

/// Decodes one answer line.
fn decoded(line: &str) -> Result<TaskState, TaskFault> {
    match serde_json::from_str::<Answer>(line).map_err(|_| TaskFault::Malformed)? {
        Answer::Absent {} => Ok(TaskState::Absent),
        Answer::Enabled { definition_sha256 } => Ok(TaskState::Enabled {
            definition_sha256: digest(&definition_sha256)?,
        }),
        Answer::Disabled { definition_sha256 } => Ok(TaskState::Disabled {
            definition_sha256: digest(&definition_sha256)?,
        }),
    }
}

/// A SHA-256 written as 64 lowercase hex digits, the one spelling the
/// script prints.
fn digest(hex: &str) -> Result<[u8; 32], TaskFault> {
    let bytes = super::bytes_of_hex(hex).ok_or(TaskFault::Malformed)?;
    <[u8; 32]>::try_from(bytes).map_err(|_| TaskFault::Malformed)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::string_slice,
    clippy::wildcard_enum_match_arm,
    reason = "test code"
)]
mod tests {
    use super::*;

    const DIGEST: &str = "e677637cdc00fe069aafeb6ee4663de0a21a2ae116b3aa4967ecd21f24d58feb";

    #[test]
    fn each_state_the_script_prints_decodes_to_one_task_state() {
        assert!(decoded(r#"{"state":"absent"}"#).unwrap() == TaskState::Absent);
        let enabled = decoded(&format!(
            r#"{{"state":"enabled","definition_sha256":"{DIGEST}"}}"#
        ))
        .unwrap();
        let disabled = decoded(&format!(
            r#"{{"state":"disabled","definition_sha256":"{DIGEST}"}}"#
        ))
        .unwrap();
        match (enabled, disabled) {
            (
                TaskState::Enabled {
                    definition_sha256: on,
                },
                TaskState::Disabled {
                    definition_sha256: off,
                },
            ) => {
                assert_eq!(on, off);
                assert_eq!(on.first(), Some(&0xe6));
                assert_eq!(on.last(), Some(&0xeb));
            }
            _ => panic!("states decoded out of order"),
        }
    }

    /// An answer this adapter does not know is refused rather than read
    /// as some state: an unknown field, an unknown state, a missing or
    /// malformed digest, a digest on an absent task, or no answer at all.
    #[test]
    fn an_answer_outside_the_closed_set_is_refused() {
        for line in [
            String::new(),
            "not json".to_owned(),
            r#"{"state":"absent","definition_sha256":"00"}"#.to_owned(),
            r#"{"state":"ready"}"#.to_owned(),
            r#"{"state":"enabled"}"#.to_owned(),
            format!(r#"{{"state":"enabled","definition_sha256":"{DIGEST}","path":"x"}}"#),
            format!(r#"{{"state":"disabled","definition_sha256":"{}"}}"#, DIGEST.to_uppercase()),
            format!(r#"{{"state":"disabled","definition_sha256":"{}"}}"#, &DIGEST[..62]),
            format!(r#"{{"state":"disabled","definition_sha256":"{DIGEST}00"}}"#),
            r#"{"state":"disabled","definition_sha256":"zz77637cdc00fe069aafeb6ee4663de0a21a2ae116b3aa4967ecd21f24d58feb"}"#.to_owned(),
        ] {
            assert!(matches!(decoded(&line), Err(TaskFault::Malformed)), "{line}");
        }
    }

    #[test]
    fn both_task_scripts_parse() {
        assert!(super::super::parses(READ));
        assert!(super::super::parses(SWITCH));
        assert!(!super::super::parses("if ("));
    }

    /// Read only, through the production path: a task folder that cannot
    /// exist reads as absent, not as a fault.
    #[test]
    fn a_task_that_does_not_exist_reads_as_absent() {
        assert!(read(r"\sprawling-absent\", "probe").unwrap() == TaskState::Absent);
    }
}
