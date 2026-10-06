// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The production host the privacy page and its operations run through
//! (`crates/sprawling/spec/Privacy/Service.lean` D68): this process's
//! clock, the identity of the account running it, the owner binding in the
//! platform vault, and the targets of the controls.
//!
//! Only the clock, the identity and the check of a recorded owner are real
//! today. Target reads and writes, the version record and the first owner
//! binding are refused with a recovery, so the page shows every control as
//! unread rather than as holding no value.

use kernel::{AxCode, AxError, SecretRef, TimeMs};
use wire::{PrivacyControl, PrivacyHost};
use zeroize::Zeroizing;

use super::coordinator::Host;
use super::fault::{ReadFault, WriteFault};
use super::service::Machine;
use super::target::{Reading, Snapshot};

/// The machine this process runs on.
pub(crate) struct System;

impl accounting::Clock for System {
    fn now(&self) -> Result<TimeMs, AxError> {
        accounting::Clock::now(&crate::assembly::SystemClock)
    }
}

impl Host for System {
    type Identity = Zeroizing<String>;

    fn identity(&mut self) -> Result<Zeroizing<String>, AxError> {
        super::identity::read()
    }

    fn owner(
        &mut self,
        recorded: Option<&SecretRef>,
        identity: &Zeroizing<String>,
    ) -> Result<SecretRef, AxError> {
        let recorded = recorded.ok_or_else(|| {
            unbuilt(
                "bind this account as the owner of the privacy history",
                "the platform vault has no writer for the owner binding yet",
            )
        })?;
        gateway::verify_platform_identity(recorded, identity).map(|()| recorded.clone())
    }

    fn read(&mut self, _control: PrivacyControl) -> Result<Reading, ReadFault> {
        Err(ReadFault::Failed(targets("read a privacy control")))
    }

    fn write(&mut self, _control: PrivacyControl, _value: &Snapshot) -> Result<(), WriteFault> {
        Err(WriteFault::Failed(targets("write a privacy control")))
    }
}

impl Machine for System {
    fn facts(&mut self) -> PrivacyHost {
        if cfg!(windows) {
            PrivacyHost::Unreadable {
                error: targets("read the Windows version record"),
            }
        } else {
            PrivacyHost::NotWindows
        }
    }
}

/// Why a target or the version record is not read: on Windows the
/// adapters are not routed here yet, elsewhere there is nothing to read.
fn targets(action: &'static str) -> AxError {
    if cfg!(windows) {
        unbuilt(
            action,
            "the Windows privacy adapters are not part of this build",
        )
    } else {
        unbuilt(action, "privacy controls exist only on Windows")
    }
}

fn unbuilt(action: &'static str, subject: &str) -> AxError {
    AxError::failure(AxCode::ToolUnavailable, action, subject)
        .with_recovery("nothing was read or written; this build cannot change this setting")
}
