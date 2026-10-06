// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The production privacy host (`crates/sprawling/spec/Privacy/Cli.lean`):
//! the account's identity and its owner binding, and each control's read
//! and write routed to its adapter, machine scope through elevation.
//!
//! The adapters know nothing of controls and decide nothing; this module
//! knows which adapter a control's target needs and maps what the adapter
//! reports onto the faults the coordinator reads.

use kernel::{AxCode, AxError, SecretRef, TimeMs};
use winreg::HKEY;
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
use wire::PrivacyControl;
use zeroize::Zeroizing;

use super::super::controls::definition;
use super::super::coordinator::Host;
use super::super::elevation::{self, ElevationFault, MachineWrite};
use super::super::fault::{ReadFault, WriteFault};
use super::super::target::{Hive, Reading, Snapshot, Target};
use super::{environment, registry, task};

/// The realm every privacy owner reference lives in.
const OWNER_REALM: &str = "privacy";

/// This machine as a privacy operation sees it, with the clock the
/// caller hands in (the one sampling point stays in `bin::assembly`).
pub(crate) struct WindowsHost<C> {
    clock: C,
}

impl<C: accounting::Clock> WindowsHost<C> {
    pub(crate) const fn new(clock: C) -> Self {
        Self { clock }
    }
}

impl<C: accounting::Clock> accounting::Clock for WindowsHost<C> {
    fn now(&self) -> Result<TimeMs, AxError> {
        self.clock.now()
    }
}

impl<C: accounting::Clock> Host for WindowsHost<C> {
    type Identity = Zeroizing<String>;

    fn identity(&mut self) -> Result<Self::Identity, AxError> {
        super::super::identity::read()
    }

    /// A recorded reference is verified and kept; an empty history gets a
    /// fresh reference with the identity bound under it (gateway D30).
    fn owner(
        &mut self,
        recorded: Option<&SecretRef>,
        identity: &Self::Identity,
    ) -> Result<SecretRef, AxError> {
        match recorded {
            Some(owner) => {
                gateway::verify_platform_identity(owner, identity).map(|()| owner.clone())
            }
            None => {
                let mut drawn = [0_u8; 8];
                getrandom::fill(&mut drawn).map_err(|source| {
                    AxError::failure(
                        AxCode::ToolUnavailable,
                        "bind privacy owner",
                        format!("no random reference name: {source}"),
                    )
                    .with_recovery("nothing was written; ask again")
                })?;
                let owner = SecretRef::new(
                    OWNER_REALM,
                    &format!("owner-{:016x}", u64::from_le_bytes(drawn)),
                )?;
                gateway::bind_platform_identity(&owner, identity).map(|()| owner)
            }
        }
    }

    fn read(&mut self, control: PrivacyControl) -> Result<Reading, ReadFault> {
        match definition(control).target {
            Target::Registry { hive, path, name } => {
                registry_reading(registry::read(hkey(hive), path, name))
            }
            Target::UserEnvironment { name } => registry_reading(environment::read(name)),
            Target::ScheduledTask { path, name } => task::read(path, name)
                .map(|state| Reading {
                    value: Snapshot::Task(state),
                    key_existed: true,
                })
                .map_err(|fault| ReadFault::Failed(task_failure("read a scheduled task", fault))),
        }
    }

    fn write(&mut self, control: PrivacyControl, value: &Snapshot) -> Result<(), WriteFault> {
        match (definition(control).target, value) {
            (
                Target::Registry {
                    hive: Hive::CurrentUser,
                    path,
                    name,
                },
                Snapshot::Registry(raw),
            ) => registry::write(HKEY_CURRENT_USER, path, name, raw).map_err(registry_fault),
            (Target::UserEnvironment { name }, Snapshot::Registry(raw)) => {
                match environment::write(name, raw).map_err(registry_fault)? {
                    environment::Written::Announced => Ok(()),
                    // The value is in place; the readback concludes, and
                    // the failed announcement is reported as the write's.
                    environment::Written::Unannounced(failure) => Err(WriteFault::Failed(failure)),
                }
            }
            (
                Target::Registry {
                    hive: Hive::LocalMachine,
                    ..
                }
                | Target::ScheduledTask { .. },
                _,
            ) => elevation::write(&MachineWrite {
                control,
                value: value.clone(),
            })
            .map_err(|fault| match fault {
                ElevationFault::Declined => WriteFault::Declined,
                ElevationFault::Exited(code) => WriteFault::Failed(
                    AxError::failure(
                        AxCode::ToolUnavailable,
                        "write a machine privacy value",
                        format!("the elevated write exited with {code:?}"),
                    )
                    .with_recovery("the value is read back; nothing else is assumed"),
                ),
                ElevationFault::Unavailable(error) => WriteFault::Failed(error),
            }),
            (
                Target::Registry {
                    hive: Hive::CurrentUser,
                    ..
                }
                | Target::UserEnvironment { .. },
                Snapshot::Task(_),
            ) => Err(WriteFault::Failed(
                AxError::failure(
                    AxCode::InvalidArgs,
                    "write a user privacy value",
                    "a task state was asked of a registry value",
                )
                .with_recovery("nothing was written"),
            )),
        }
    }
}

const fn hkey(hive: Hive) -> HKEY {
    match hive {
        Hive::CurrentUser => HKEY_CURRENT_USER,
        Hive::LocalMachine => HKEY_LOCAL_MACHINE,
    }
}

fn registry_reading(reading: registry::Reading) -> Result<Reading, ReadFault> {
    match reading {
        registry::Reading::Value { value, key_existed } => Ok(Reading {
            value: Snapshot::Registry(value),
            key_existed,
        }),
        registry::Reading::AccessDenied => Err(ReadFault::AccessDenied),
        registry::Reading::Failed(source) => Err(ReadFault::Failed(
            AxError::failure(
                AxCode::StorageFatal,
                "read a privacy value",
                source.to_string(),
            )
            .with_recovery("nothing was written; read the page again"),
        )),
    }
}

fn registry_fault(fault: registry::WriteFault) -> WriteFault {
    match fault {
        registry::WriteFault::AccessDenied => WriteFault::AccessDenied,
        registry::WriteFault::NotRewritable => WriteFault::Failed(
            AxError::failure(
                AxCode::InvalidArgs,
                "write a user privacy value",
                "the value has no exact raw encoding",
            )
            .with_recovery("nothing was written"),
        ),
        registry::WriteFault::Failed(source) => WriteFault::Failed(
            AxError::failure(
                AxCode::StorageFatal,
                "write a user privacy value",
                source.to_string(),
            )
            .with_recovery("the value is read back; nothing else is assumed"),
        ),
    }
}

fn task_failure(action: &'static str, fault: task::TaskFault) -> AxError {
    match fault {
        task::TaskFault::Unavailable(error) => error,
        task::TaskFault::Refused(code) => AxError::failure(
            AxCode::ToolUnavailable,
            action,
            format!("the task script exited with {code:?}"),
        )
        .with_recovery("check that this account may query scheduled tasks, then ask again"),
        task::TaskFault::Malformed => AxError::failure(
            AxCode::ToolUnavailable,
            action,
            "the task script answered out of shape",
        )
        .with_recovery("nothing was written; report the Windows build this happened on"),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, reason = "test code")]
mod tests {
    use super::*;

    /// Read only, through the production adapters: one control of each
    /// operation kind reads as a snapshot of its kind.
    #[test]
    fn one_control_of_each_kind_reads_as_its_kind() {
        let mut host = WindowsHost::new(crate::assembly::SystemClock);
        for control in [
            PrivacyControl::FeedbackNotifications,
            PrivacyControl::StartLaunchTracking,
            PrivacyControl::PowershellTelemetryOptout,
            PrivacyControl::DeviceCensusTask,
        ] {
            let reading = host.read(control).unwrap();
            match (definition(control).target, reading.value) {
                (
                    Target::Registry { .. } | Target::UserEnvironment { .. },
                    Snapshot::Registry(_),
                )
                | (Target::ScheduledTask { .. }, Snapshot::Task(_)) => {}
                (target, value) => panic!("{control:?}: {target:?} read as {value:?}"),
            }
        }
    }
}
