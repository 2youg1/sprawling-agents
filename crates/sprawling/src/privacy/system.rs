// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The machine this process runs on, as the privacy page sees it
//! (`crates/sprawling/spec/Privacy/Service.lean` D71): on Windows the
//! production host (`bin::privacy::windows::host`) with the facts its
//! version record states; anywhere else a host that has no privacy
//! controls, refuses every operation and is never read.

use wire::PrivacyHost;

use super::service::Machine;

/// The machine this process runs on.
#[cfg(windows)]
pub(crate) type System = super::windows::host::WindowsHost<crate::assembly::SystemClock>;
/// The machine this process runs on.
#[cfg(not(windows))]
pub(crate) type System = Elsewhere;

/// A fresh handle on the machine this process runs on.
pub(crate) const fn this_machine() -> System {
    #[cfg(windows)]
    {
        super::windows::host::WindowsHost::new(crate::assembly::SystemClock)
    }
    #[cfg(not(windows))]
    {
        Elsewhere
    }
}

#[cfg(windows)]
impl<C: accounting::Clock + Send> Machine for super::windows::host::WindowsHost<C> {
    fn facts(&mut self) -> PrivacyHost {
        match super::windows::host_facts() {
            Ok(facts) => PrivacyHost::Windows(wire::PrivacyWindows {
                edition_id: facts.edition_id,
                edition: facts.edition,
                build: facts.build,
                display_version: facts.display_version,
            }),
            Err(error) => PrivacyHost::Unreadable { error },
        }
    }
}

/// A host that is not Windows: privacy controls are Windows settings.
#[cfg(not(windows))]
pub(crate) struct Elsewhere;

#[cfg(not(windows))]
impl accounting::Clock for Elsewhere {
    fn now(&self) -> Result<kernel::TimeMs, kernel::AxError> {
        accounting::Clock::now(&crate::assembly::SystemClock)
    }
}

#[cfg(not(windows))]
impl super::coordinator::Host for Elsewhere {
    type Identity = ();

    fn identity(&mut self) -> Result<(), kernel::AxError> {
        Err(windows_only("read privacy principal"))
    }

    fn owner(
        &mut self,
        _recorded: Option<&kernel::SecretRef>,
        (): &(),
    ) -> Result<kernel::SecretRef, kernel::AxError> {
        Err(windows_only("verify privacy owner"))
    }

    fn read(
        &mut self,
        _control: wire::PrivacyControl,
    ) -> Result<super::target::Reading, super::fault::ReadFault> {
        Err(super::fault::ReadFault::Failed(windows_only(
            "read a privacy control",
        )))
    }

    fn write(
        &mut self,
        _control: wire::PrivacyControl,
        _value: &super::target::Snapshot,
    ) -> Result<(), super::fault::WriteFault> {
        Err(super::fault::WriteFault::Failed(windows_only(
            "write a privacy control",
        )))
    }
}

#[cfg(not(windows))]
impl Machine for Elsewhere {
    fn facts(&mut self) -> PrivacyHost {
        PrivacyHost::NotWindows
    }
}

/// Why `action` is refused on a host that is not Windows.
#[cfg(not(windows))]
pub(super) fn windows_only(action: &'static str) -> kernel::AxError {
    kernel::AxError::failure(
        kernel::AxCode::ToolUnavailable,
        action,
        "privacy controls are Windows settings",
    )
    .with_recovery("nothing was written; privacy controls are written on Windows only")
}

#[cfg(all(test, windows))]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use wire::PrivacyCurrent;

    use super::super::service::Service;
    use super::*;

    /// The page's production path end to end on this machine, reading
    /// only: the version record, then every control through the adapters
    /// the CLI write verbs use. A history in a fresh directory keeps the
    /// person's own history out of it, and nothing is written.
    #[test]
    fn the_page_reads_every_control_of_this_machine() {
        let dir = tempfile::tempdir().unwrap();
        let answer = Service::new(this_machine, dir.path().join("changes.jsonl")).answer();
        assert!(
            matches!(answer.host, PrivacyHost::Windows(_)),
            "{:?}",
            answer.host
        );
        assert_eq!(answer.controls.len(), wire::PrivacyControl::ALL.len());
        for entry in &answer.controls {
            assert!(
                matches!(
                    entry.current,
                    PrivacyCurrent::Read { .. } | PrivacyCurrent::AccessDenied
                ),
                "{entry:?}"
            );
        }
        assert!(!dir.path().join("changes.jsonl").exists());
    }
}
