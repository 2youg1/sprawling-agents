// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Machine-scope writes through a short-lived UAC-elevated child that
//! writes and never judges (`crates/sprawling/spec/Privacy/Windows.lean` D57).
//!
//! The parent starts this same executable as `privacy elevated-write
//! <write>` under the `runas` verb and waits for it; the child checks the
//! write against the control table, writes, and exits. The parent then
//! reads the target back itself, so neither a declined prompt nor a
//! child's exit code is taken as the outcome.

use kernel::{AxCode, AxError};
use serde::{Deserialize, Serialize};
use winreg::enums::HKEY_LOCAL_MACHINE;
use wire::PrivacyControl;

use super::controls::definition;
use super::target::{Hive, Snapshot, Target, TaskState};
use super::windows::{registry, task};

/// The most bytes a registry value may carry through the command line;
/// every value the control table writes is four bytes, so only an
/// original can be longer (Privacy.Windows D57).
const VALUE_BYTES_MAX: usize = 1024;

/// `ERROR_CANCELLED`: the person declined the UAC prompt.
const DECLINED: i32 = 1223;

/// Starts the elevated child and waits for it. The child's arguments
/// reach this script through the environment, so it carries no quoting
/// rule; a declined prompt surfaces as a `Win32Exception` somewhere in
/// the exception chain and exits with its code.
const ELEVATE: &str = r"
$ErrorActionPreference='Stop'
$start = New-Object System.Diagnostics.ProcessStartInfo
$start.FileName = $env:SPRAWLING_SELF
$start.Arguments = 'privacy elevated-write ' + $env:SPRAWLING_WRITE
$start.Verb = 'runas'
$start.UseShellExecute = $true
$start.WindowStyle = [System.Diagnostics.ProcessWindowStyle]::Hidden
try { $child = [System.Diagnostics.Process]::Start($start) }
catch {
  $cause = $_.Exception
  while ($null -ne $cause) {
    if ($cause -is [System.ComponentModel.Win32Exception] -and $cause.NativeErrorCode -eq 1223) { exit 1223 }
    $cause = $cause.InnerException
  }
  throw
}
$child.WaitForExit()
exit $child.ExitCode
";

/// One machine-scope write: the control and the snapshot its target is
/// to become. The whole argument the elevated child receives.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::privacy) struct MachineWrite {
    pub(in crate::privacy) control: PrivacyControl,
    pub(in crate::privacy) value: Snapshot,
}

/// Why the elevated write did not report success. None of these is an
/// outcome: the parent reads the target back either way.
#[derive(Debug)]
pub(in crate::privacy) enum ElevationFault {
    /// The person declined the UAC prompt.
    Declined,
    /// The child, or the script that starts it, exited with this code.
    Exited(Option<i32>),
    /// Windows PowerShell or this executable could not be named or
    /// started.
    Unavailable(AxError),
}

/// Writes `write` through an elevated child and waits for it to exit.
///
/// The wait has no counted bound: the UAC prompt waits on a person, and a
/// readback taken while the child might still write would judge a late
/// write as not applied.
///
/// # Errors
/// [`ElevationFault`].
pub(in crate::privacy) fn write(write: &MachineWrite) -> Result<(), ElevationFault> {
    let encoded = serde_json::to_vec(write)
        .map_err(|source| ElevationFault::Unavailable(refused(&source.to_string())))?;
    let this = std::env::current_exe()
        .map_err(|source| ElevationFault::Unavailable(refused(&source.to_string())))?;
    let status = std::process::Command::new(
        super::windows::powershell().map_err(ElevationFault::Unavailable)?,
    )
    .args([
        "-NoLogo",
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        ELEVATE,
    ])
    .env("SPRAWLING_SELF", this)
    .env("SPRAWLING_WRITE", hex(&encoded))
    .stdin(std::process::Stdio::null())
    .stdout(std::process::Stdio::null())
    .stderr(std::process::Stdio::null())
    .status()
    .map_err(|source| ElevationFault::Unavailable(refused(&source.to_string())))?;
    judged(status.code())
}

/// The elevated child's side: decodes `argument`, checks it against the
/// control table, and writes. It reads nothing back and records nothing.
///
/// # Errors
/// `InvalidArgs` when the argument is not one machine-scope write the
/// control table allows; `StorageFatal` when the write fails.
pub(in crate::privacy) fn carry_out(argument: &str) -> Result<(), AxError> {
    let json = super::windows::bytes_of_hex(argument)
        .ok_or_else(|| invalid("the argument is not lowercase hexadecimal"))?;
    let write: MachineWrite = serde_json::from_slice(&json)
        .map_err(|_| invalid("the argument is not one machine-scope write"))?;
    match planned(&write)? {
        Planned::Registry { path, name, value } => {
            registry::write(HKEY_LOCAL_MACHINE, path, name, value).map_err(|fault| {
                AxError::failure(
                    AxCode::StorageFatal,
                    "write a machine privacy value",
                    match fault {
                        registry::WriteFault::AccessDenied => "access denied".to_owned(),
                        registry::WriteFault::NotRewritable => {
                            "the value has no exact raw encoding".to_owned()
                        }
                        registry::WriteFault::Failed(source) => source.to_string(),
                    },
                )
                .with_recovery("the parent reads the value back and reports what it found")
            })
        }
        Planned::Task { path, name, switch } => task::write(path, name, switch).map_err(|fault| {
            AxError::failure(
                AxCode::StorageFatal,
                "switch a scheduled task",
                match fault {
                    task::TaskFault::Unavailable(cause) => cause.subject().to_owned(),
                    task::TaskFault::Refused(code) => {
                        format!("the task script exited with {code:?}")
                    }
                    task::TaskFault::Malformed => {
                        "the task script answered out of shape".to_owned()
                    }
                },
            )
            .with_recovery("the parent reads the task back and reports what it found")
        }),
    }
}

/// A write the control table allows, resolved to its target.
#[derive(Debug, PartialEq, Eq)]
enum Planned<'write> {
    Registry {
        path: &'static str,
        name: &'static str,
        value: &'write super::target::RawValue,
    },
    Task {
        path: &'static str,
        name: &'static str,
        switch: task::Switch,
    },
}

/// Checks `write` against its control's row: a machine-scope target, a
/// snapshot of that target's kind, a task switched rather than created
/// or removed, and a value inside [`VALUE_BYTES_MAX`].
fn planned(write: &MachineWrite) -> Result<Planned<'_>, AxError> {
    match (definition(write.control).target, &write.value) {
        (
            Target::Registry {
                hive: Hive::LocalMachine,
                path,
                name,
            },
            Snapshot::Registry(value),
        ) => match value {
            super::target::RawValue::Present { bytes, .. } if bytes.len() > VALUE_BYTES_MAX => Err(
                invalid("the value is longer than one elevated write carries"),
            ),
            super::target::RawValue::Absent | super::target::RawValue::Present { .. } => {
                Ok(Planned::Registry { path, name, value })
            }
        },
        (Target::ScheduledTask { path, name }, Snapshot::Task(state)) => match state {
            TaskState::Enabled { .. } => Ok(Planned::Task {
                path,
                name,
                switch: task::Switch::Enable,
            }),
            TaskState::Disabled { .. } => Ok(Planned::Task {
                path,
                name,
                switch: task::Switch::Disable,
            }),
            TaskState::Absent => Err(invalid("a task is switched, never created or removed")),
        },
        (
            Target::Registry {
                hive: Hive::CurrentUser,
                ..
            }
            | Target::UserEnvironment { .. },
            _,
        ) => Err(invalid("a user-scope control is never written elevated")),
        (Target::Registry { .. } | Target::ScheduledTask { .. }, _) => {
            Err(invalid("the snapshot is not of the control's kind"))
        }
    }
}

/// What the starting script's exit code says.
fn judged(code: Option<i32>) -> Result<(), ElevationFault> {
    match code {
        Some(0) => Ok(()),
        Some(DECLINED) => Err(ElevationFault::Declined),
        Some(code) => Err(ElevationFault::Exited(Some(code))),
        None => Err(ElevationFault::Exited(None)),
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn invalid(subject: &str) -> AxError {
    AxError::failure(
        AxCode::InvalidArgs,
        "check an elevated privacy write",
        subject,
    )
    .with_recovery("nothing was written; this verb is started only by the privacy page's elevation")
}

fn refused(subject: &str) -> AxError {
    AxError::failure(
        AxCode::ToolUnavailable,
        "start the elevated privacy write",
        subject,
    )
    .with_recovery(
        "nothing was written; check that Windows PowerShell and this executable can start",
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, reason = "test code")]
mod tests {
    use super::*;
    use crate::privacy::target::RawValue;

    fn dword(value: u32) -> Snapshot {
        Snapshot::Registry(RawValue::Present {
            kind: 4,
            bytes: value.to_le_bytes().to_vec(),
        })
    }

    fn checked(control: PrivacyControl, value: Snapshot) -> Result<(), AxCode> {
        planned(&MachineWrite { control, value })
            .map(drop)
            .map_err(|refusal| *refusal.code())
    }

    /// A declined prompt and every other exit are faults the caller reads
    /// back after; only a clean exit is success.
    #[test]
    fn the_exit_code_tells_a_declined_prompt_from_a_failed_child() {
        assert!(judged(Some(0)).is_ok());
        assert!(matches!(judged(Some(1223)), Err(ElevationFault::Declined)));
        assert!(matches!(
            judged(Some(1)),
            Err(ElevationFault::Exited(Some(1)))
        ));
        assert!(matches!(judged(None), Err(ElevationFault::Exited(None))));
    }

    /// The child writes only what the control table makes a machine-scope
    /// write: HKLM values and task switches, of the control's own kind,
    /// within the size bound.
    #[test]
    fn only_a_machine_scope_write_of_the_controls_kind_is_carried_out() {
        let digest = [7_u8; 32];
        assert_eq!(checked(PrivacyControl::DiagnosticData, dword(0)), Ok(()));
        assert_eq!(
            checked(
                PrivacyControl::DiagnosticData,
                Snapshot::Registry(RawValue::Absent)
            ),
            Ok(())
        );
        assert_eq!(
            checked(
                PrivacyControl::DeviceCensusTask,
                Snapshot::Task(TaskState::Disabled {
                    definition_sha256: digest
                })
            ),
            Ok(())
        );
        assert_eq!(
            checked(
                PrivacyControl::DeviceCensusTask,
                Snapshot::Task(TaskState::Enabled {
                    definition_sha256: digest
                })
            ),
            Ok(())
        );
        for (control, value) in [
            (PrivacyControl::StartLaunchTracking, dword(0)),
            (PrivacyControl::TailoredExperiences, dword(1)),
            (
                PrivacyControl::PowershellTelemetryOptout,
                Snapshot::Registry(RawValue::Absent),
            ),
            (
                PrivacyControl::DiagnosticData,
                Snapshot::Task(TaskState::Disabled {
                    definition_sha256: digest,
                }),
            ),
            (PrivacyControl::DeviceCensusTask, dword(0)),
            (
                PrivacyControl::DeviceCensusTask,
                Snapshot::Task(TaskState::Absent),
            ),
            (
                PrivacyControl::DiagnosticData,
                Snapshot::Registry(RawValue::Present {
                    kind: 3,
                    bytes: vec![0; VALUE_BYTES_MAX.checked_add(1).unwrap()],
                }),
            ),
        ] {
            assert_eq!(checked(control, value), Err(AxCode::InvalidArgs));
        }
    }

    #[test]
    fn the_elevating_script_parses() {
        assert!(crate::privacy::windows::parses(ELEVATE));
    }

    /// An argument that is not exactly one encoded write is refused
    /// before anything is written.
    #[test]
    fn a_malformed_argument_is_refused_before_any_write() {
        let encoded = |json: &str| hex(json.as_bytes());
        for argument in [
            String::new(),
            "zz".to_owned(),
            "ABCD".to_owned(),
            encoded("{}"),
            encoded(r#"{"control":"diagnostic_data"}"#),
            encoded(
                r#"{"control":"diagnostic_data","value":{"registry":{"state":"absent"}},"extra":1}"#,
            ),
            encoded(r#"{"control":"no_such_control","value":{"registry":{"state":"absent"}}}"#),
            encoded(
                r#"{"control":"start_launch_tracking","value":{"registry":{"state":"absent"}}}"#,
            ),
        ] {
            assert_eq!(
                *carry_out(&argument).unwrap_err().code(),
                AxCode::InvalidArgs,
                "{argument}"
            );
        }
    }
}
