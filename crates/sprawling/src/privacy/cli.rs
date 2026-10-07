// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The privacy command verbs (`crates/sprawling/spec/Privacy/Cli.lean`):
//! status and inspect read; apply, restore, restore-all and reconcile run
//! through the one coordinator; elevated-write is the elevated child's.

use accounting::home::Home;
use kernel::{AxCode, AxError, SecretRef};
use serde::Serialize;
use wire::PrivacyControl;
use zeroize::Zeroizing;

use super::coordinator::Command;
use super::target::Snapshot;

/// Reads local operation receipts without creating or settling history,
/// and shows them only to the identity their owner reference is bound to.
///
/// # Errors
/// Identity sampling, owner binding, home detection, busy or malformed history,
/// IO, and JSON encoding failures.
pub fn status() -> Result<String, AxError> {
    status_at(
        &Home::detect()?.privacy_history(),
        super::identity::read,
        gateway::verify_platform_identity,
    )
}

/// One JSON line per control (only `control` when one is named): what its
/// target reads now, and what this app owns of it.
///
/// # Errors
/// An unknown control name, identity sampling, home detection, owner
/// verification and history faults; `ToolUnavailable` off Windows.
pub fn inspect(clock: impl accounting::Clock, control: Option<&str>) -> Result<String, AxError> {
    let only = control.map(named).transpose()?;
    #[cfg(windows)]
    {
        inspected(
            &mut super::windows::host::WindowsHost::new(clock),
            &Home::detect()?.privacy_history(),
            only,
        )
    }
    #[cfg(not(windows))]
    {
        drop((clock, only));
        Err(super::system::windows_only("inspect privacy controls"))
    }
}

/// Writes the value the controls table holds for `control`, provided its
/// target still reads `expected`, the reading `inspect` printed.
///
/// # Errors
/// Malformed arguments, and every privacy fault, worded by its stable code.
pub fn apply(
    clock: impl accounting::Clock,
    control: Option<&str>,
    expected: Option<&str>,
) -> Result<String, AxError> {
    run(
        clock,
        &Command::Apply {
            control: named(required(control, "control")?)?,
            expected: reading(required(expected, "expected")?)?,
        },
    )
}

/// Writes back the original of the latest change this app owns of
/// `control`, provided its target still reads `expected`.
///
/// # Errors
/// As [`apply`].
pub fn restore(
    clock: impl accounting::Clock,
    control: Option<&str>,
    expected: Option<&str>,
) -> Result<String, AxError> {
    run(
        clock,
        &Command::Restore {
            control: named(required(control, "control")?)?,
            expected: reading(required(expected, "expected")?)?,
        },
    )
}

/// Records the person's check of the unresolved operation; writes nothing
/// to the host.
///
/// # Errors
/// As [`apply`].
pub fn reconcile(clock: impl accounting::Clock, expected: Option<&str>) -> Result<String, AxError> {
    run(
        clock,
        &Command::Reconcile {
            expected: reading(required(expected, "expected")?)?,
        },
    )
}

/// Restores, control by control, every change this app still owns, each
/// against what its target reads just before (Privacy D63). `emit`
/// receives one line per control as it ends; the first failure stops the
/// rest.
///
/// # Errors
/// The first failure, after the lines of the controls before it.
pub fn restore_all(clock: impl accounting::Clock, emit: impl FnMut(String)) -> Result<(), AxError> {
    #[cfg(windows)]
    {
        restored_all(clock, &Home::detect()?.privacy_history(), emit)
    }
    #[cfg(not(windows))]
    {
        drop((clock, emit));
        Err(super::system::windows_only(RESTORE_ALL))
    }
}

/// Carries out one machine-scope write as the elevated child the privacy
/// page starts (`crates/sprawling/spec/Privacy/Windows.lean` D60): checks
/// it against the control table and writes, reading nothing back and
/// recording nothing.
///
/// # Errors
/// `InvalidArgs` when `write` is missing or is not one machine-scope write
/// the control table allows; `StorageFatal` when the write fails;
/// `ToolUnavailable` off Windows.
pub fn elevated_write(write: Option<&str>) -> Result<(), AxError> {
    let refused = |code, subject: &str, recovery: &str| {
        AxError::failure(code, "check an elevated privacy write", subject).with_recovery(recovery)
    };
    let write = write.ok_or_else(|| {
        refused(
            AxCode::InvalidArgs,
            "no write was given",
            "nothing was written; this verb is started only by the privacy page's elevation",
        )
    })?;
    #[cfg(windows)]
    {
        super::elevation::carry_out(write)
    }
    #[cfg(not(windows))]
    {
        Err(refused(
            AxCode::ToolUnavailable,
            &format!("a {}-character write names a Windows control", write.len()),
            "nothing was written; privacy controls are written on Windows only",
        ))
    }
}

const RESTORE_ALL: &str = "restore all privacy controls";

/// What one control reads now and what this app owns of it.
#[cfg(windows)]
#[derive(Serialize)]
struct Inspected<'h> {
    control: PrivacyControl,
    #[serde(flatten)]
    read: Read,
    owned: Option<Owned<'h>>,
    unresolved: bool,
}

#[cfg(windows)]
#[derive(Serialize)]
#[serde(untagged)]
enum Read {
    Reading {
        reading: Snapshot,
        key_existed: bool,
    },
    Unreadable {
        unreadable: String,
    },
}

#[cfg(windows)]
#[derive(Serialize)]
struct Owned<'h> {
    operation: std::num::NonZeroU64,
    original: &'h Snapshot,
    modified: &'h Snapshot,
}

/// One control's line of `restore-all`.
#[cfg(windows)]
#[derive(Serialize)]
struct Ended {
    control: PrivacyControl,
    #[serde(flatten)]
    done: super::coordinator::Done,
}

#[cfg(windows)]
fn inspected<C: accounting::Clock>(
    host: &mut super::windows::host::WindowsHost<C>,
    path: &std::path::Path,
    only: Option<PrivacyControl>,
) -> Result<String, AxError> {
    use super::coordinator::Host;

    let identity = host.identity()?;
    let history =
        super::journal::read(path).map_err(|fault| fault.into_ax("inspect privacy controls"))?;
    let holdings = reader(&history, &identity)?;
    let controls = match only {
        Some(control) => vec![control],
        None => PrivacyControl::ALL.to_vec(),
    };
    let mut lines = Vec::with_capacity(controls.len());
    for control in controls {
        let read = match host.read(control) {
            Ok(fresh) => Read::Reading {
                reading: fresh.value,
                key_existed: fresh.key_existed,
            },
            Err(fault) => Read::Unreadable {
                unreadable: fault.to_string(),
            },
        };
        lines.push(encoded(&Inspected {
            control,
            read,
            owned: holdings.latest_owned(control).map(|intent| Owned {
                operation: intent.operation,
                original: &intent.original,
                modified: &intent.modified,
            }),
            unresolved: holdings
                .unresolved()
                .is_some_and(|intent| intent.control == control),
        })?);
    }
    Ok(lines.join("\n"))
}

#[cfg(windows)]
fn restored_all(
    clock: impl accounting::Clock,
    path: &std::path::Path,
    mut emit: impl FnMut(String),
) -> Result<(), AxError> {
    use super::coordinator::Host;

    let mut host = super::windows::host::WindowsHost::new(clock);
    let identity = host.identity()?;
    let history = super::journal::read(path).map_err(|fault| fault.into_ax(RESTORE_ALL))?;
    let owned: Vec<PrivacyControl> = {
        let holdings = reader(&history, &identity)?;
        PrivacyControl::ALL
            .into_iter()
            .filter(|control| holdings.latest_owned(*control).is_some())
            .collect()
    };
    for control in owned {
        let expected = host
            .read(control)
            .map_err(|fault| {
                super::fault::PrivacyFault::Unreadable { control, fault }.into_ax(RESTORE_ALL)
            })?
            .value;
        let done = carried_out(&mut host, path, &Command::Restore { control, expected })?;
        emit(encoded(&Ended { control, done })?);
    }
    Ok(())
}

/// The holdings a reader that will not write may see: a recorded owner
/// is verified against `identity`, and an empty history asks nothing.
#[cfg(windows)]
fn reader<'h>(
    history: &'h super::state::History,
    identity: &str,
) -> Result<super::state::Holdings<'h, ()>, AxError> {
    history.holdings(|recorded| match recorded {
        None => Ok(()),
        Some(owner) => gateway::verify_platform_identity(owner, identity),
    })
}

fn run(clock: impl accounting::Clock, command: &Command) -> Result<String, AxError> {
    #[cfg(windows)]
    {
        encoded(&carried_out(
            &mut super::windows::host::WindowsHost::new(clock),
            &Home::detect()?.privacy_history(),
            command,
        )?)
    }
    #[cfg(not(windows))]
    {
        drop(clock);
        Err(super::system::windows_only(command.action()))
    }
}

#[cfg(windows)]
fn carried_out<C: accounting::Clock>(
    host: &mut super::windows::host::WindowsHost<C>,
    path: &std::path::Path,
    command: &Command,
) -> Result<super::coordinator::Done, AxError> {
    super::coordinator::run(host, || super::journal::LockedJournal::open(path), command)
        .map_err(|fault| fault.into_ax(command.action()))
}

fn required<'a>(value: Option<&'a str>, what: &str) -> Result<&'a str, AxError> {
    value.ok_or_else(|| {
        AxError::failure(
            AxCode::InvalidArgs,
            "read a privacy command",
            format!("no {what} was given"),
        )
        .with_recovery("nothing was written; run privacy inspect and pass what it prints")
    })
}

fn named(control: &str) -> Result<PrivacyControl, AxError> {
    serde_json::from_value(serde_json::Value::String(control.to_owned())).map_err(|_| {
        AxError::failure(
            AxCode::InvalidArgs,
            "read a privacy command",
            format!("{control} is not a privacy control"),
        )
        .with_recovery("nothing was written; run privacy inspect for the control names")
    })
}

fn reading(expected: &str) -> Result<Snapshot, AxError> {
    serde_json::from_str(expected).map_err(|_| {
        AxError::failure(
            AxCode::InvalidArgs,
            "read a privacy command",
            "the expected value is not one reading",
        )
        .with_recovery("nothing was written; pass the reading privacy inspect printed, unchanged")
    })
}

fn encoded(value: &impl Serialize) -> Result<String, AxError> {
    serde_json::to_string(value).map_err(|source| {
        AxError::failure(
            AxCode::InvalidArgs,
            "encode privacy answer",
            source.to_string(),
        )
        .with_recovery("keep the history unchanged and report the encoding failure")
    })
}

/// The identity is sampled before the history is opened, so a failed
/// sample says nothing about the history, not even whether it is intact.
fn status_at(
    path: &std::path::Path,
    identity: impl FnOnce() -> Result<Zeroizing<String>, AxError>,
    verify: impl FnOnce(&SecretRef, &str) -> Result<(), AxError>,
) -> Result<String, AxError> {
    let observed = identity()?;
    let history =
        super::journal::read(path).map_err(|fault| fault.into_ax("read local privacy history"))?;
    encoded(&history.disclose(|owner| verify(owner, &observed))?)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, reason = "test code")]
mod tests {
    use super::*;
    use std::path::Path;

    fn refused() -> AxError {
        AxError::failure(
            AxCode::ConfigInvalid,
            "verify privacy owner",
            "identity unavailable",
        )
        .with_recovery("leave history unchanged")
    }

    fn fixture(path: &Path) -> Vec<u8> {
        use super::super::state::fixtures::{apply, bytes, dword, prepared};
        use super::super::target::{RawValue, Snapshot};
        let bytes = bytes(&[prepared(&apply(
            1,
            wire::PrivacyControl::PowershellTelemetryOptout,
            Snapshot::Registry(RawValue::Absent),
            dword(1),
        ))]);
        std::fs::write(path, &bytes).unwrap();
        bytes
    }

    #[test]
    fn identity_failure_precedes_even_a_malformed_history_read() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.jsonl");
        std::fs::write(&path, b"not JSON").unwrap();
        assert_eq!(
            status_at(
                &path,
                || Err(refused()),
                |_, _| panic!("vault must not be queried")
            ),
            Err(refused())
        );
    }

    #[test]
    fn authorized_summary_omits_values_and_empty_history_skips_vault() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.jsonl");
        assert_eq!(
            status_at(
                &path,
                || Ok(Zeroizing::new("fixture".to_owned())),
                |_, _| panic!("empty history must not query vault")
            )
            .unwrap(),
            "[]"
        );
        let before = fixture(&path);
        let mut calls = 0;
        let summary = status_at(
            &path,
            || Ok(Zeroizing::new("fixture".to_owned())),
            |reference, observed| {
                assert_eq!(
                    reference,
                    &SecretRef::new("privacy", "fixture-owner").unwrap()
                );
                assert_eq!(observed, "fixture");
                calls += 1;
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(calls, 1);
        assert_eq!(summary, r#"[{"operation":1,"outcome":"unresolved"}]"#);
        assert_eq!(std::fs::read(path).unwrap(), before);
    }

    fn snapshot(steps: Vec<(u32, Vec<u8>, u8, bool)>) -> (Vec<u8>, usize) {
        use super::super::state::fixtures::{apply, finished, prepared, restore};
        use super::super::state::{Intent, Outcome};
        use super::super::target::{RawValue, Snapshot};
        let mut lines = Vec::new();
        let mut owned: Vec<Intent> = Vec::new();
        let mut count = 0usize;
        for (index, (kind, raw, outcome, restoring)) in steps.into_iter().enumerate() {
            let id = u64::try_from(index.checked_add(1).unwrap()).unwrap();
            let intent = match owned.last() {
                Some(previous) if restoring => restore(id, previous),
                Some(_) | None => apply(
                    id,
                    wire::PrivacyControl::PowershellTelemetryOptout,
                    owned
                        .last()
                        .map_or(Snapshot::Registry(RawValue::Absent), |previous| {
                            previous.modified.clone()
                        }),
                    Snapshot::Registry(RawValue::Present { kind, bytes: raw }),
                ),
            };
            if intent.original == intent.modified {
                break;
            }
            lines.push(prepared(&intent));
            count = count.checked_add(1).unwrap();
            if outcome == 3 {
                break;
            }
            let result = match outcome {
                0 if intent.restore_of.is_some() => Outcome::Restored,
                0 => Outcome::Applied,
                1 => Outcome::NotApplied,
                2 => Outcome::Unknown,
                _ => panic!("generator outcome outside its strategy"),
            };
            lines.push(finished(&intent, result));
            match result {
                Outcome::Applied => owned.push(intent),
                Outcome::Restored => {
                    owned.pop().unwrap();
                }
                Outcome::NotApplied | Outcome::RolledBack => (),
                Outcome::Unknown => break,
            }
        }
        (super::super::state::fixtures::bytes(&lines), count)
    }

    fn histories() -> impl proptest::strategy::Strategy<Value = (Vec<u8>, usize)> {
        use proptest::prelude::*;
        proptest::collection::vec(
            (
                any::<u32>(),
                proptest::collection::vec(any::<u8>(), 0..64),
                0u8..4,
                any::<bool>(),
            ),
            0..24,
        )
        .prop_map(snapshot)
    }

    proptest::proptest! {
        #[test]
        fn only_the_bound_identity_receives_a_summary(
            (bytes, count) in histories(), owner in proptest::num::u64::ANY,
            offset in 1u64..=u64::MAX,
        ) {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("history.jsonl");
            std::fs::write(&path, &bytes).unwrap();
            let reference = SecretRef::new("privacy", "fixture-owner").unwrap();
            let stored = owner.to_string();
            let foreign = owner.wrapping_add(offset).to_string();
            for observed in [&stored, &foreign] {
                let mut reads = 0usize;
                let result = status_at(&path, || Ok(Zeroizing::new(observed.clone())),
                    |requested, observed| gateway::verify_identity_binding(requested, observed, |requested| {
                        reads = reads.checked_add(1).unwrap();
                        assert_eq!(requested, &reference);
                        Ok(Some(Zeroizing::new(stored.clone())))
                    }));
                if count == 0 || observed == &stored {
                    let values: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
                    proptest::prop_assert_eq!(values.as_array().unwrap().len(), count);
                } else {
                    proptest::prop_assert_eq!(*result.unwrap_err().code(), AxCode::ConfigInvalid);
                }
                proptest::prop_assert_eq!(reads, usize::from(count != 0));
                proptest::prop_assert_eq!(std::fs::read(&path).unwrap(), bytes.clone());
            }
        }
    }
}
