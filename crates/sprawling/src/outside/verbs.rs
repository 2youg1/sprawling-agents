// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which class of verb a frame from a remote device carries
//! (sprawling-SPEC.md 8-139; wire-SPEC.md section 19-2).
//!
//! The table in wire-SPEC.md section 19-2 states each Command's class,
//! and `xtask wiring` reads the arms of [`command_class`] against it, so
//! the table and this match are one decision written twice and checked
//! as one. The match is exhaustive: a Command added to the wire does
//! not compile here until somebody decides its class, and the decision
//! the table records for every new Command is `LocalOnly`.
//!
//! Pure: no door, no clock. Whether a session may carry the class is
//! `remote_access::door::permits`, asked by the conduit.

use remote_access::door::VerbClass;

/// What the relay does with one frame a device sent.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Passage {
    /// The door judges it: forwarded when the session's authority
    /// permits the class, refused otherwise.
    Judged(VerbClass),
    /// The device's greeting, which the relay sends on as the city's
    /// own: the device does not know this city's pairing token, and
    /// never has to.
    Greeting(wire::Hello),
}

pub(super) fn passage(frame: wire::ClientFrame) -> Passage {
    match frame {
        wire::ClientFrame::Hello(said) => Passage::Greeting(said),
        // Asking and watching read the city and change nothing.
        wire::ClientFrame::Ask(_) | wire::ClientFrame::Monitor(_) => {
            Passage::Judged(VerbClass::Read)
        }
        wire::ClientFrame::Command(command) => Passage::Judged(command_class(&command)),
    }
}

/// The class of one Command, as wire-SPEC.md section 19-2 states it.
///
/// `Act` is the work a person away from the machine still does: send
/// work, change its course, stop it, answer what it asks. Everything
/// that widens access, reaches a credential or the machine, or changes
/// what governs the city is `LocalOnly`.
pub(super) fn command_class(command: &wire::WireCommand) -> VerbClass {
    match command {
        wire::Command::Dispatch { .. }
        | wire::Command::Steer { .. }
        | wire::Command::Cancel { .. }
        | wire::Command::Halt { .. }
        | wire::Command::Release { .. }
        | wire::Command::Approve { .. }
        | wire::Command::HandOff { .. }
        | wire::Command::BatchByBuilding { .. }
        | wire::Command::Pursue { .. }
        | wire::Command::OpenSession { .. }
        | wire::Command::PutSpine { .. } => VerbClass::Act,
        wire::Command::ProbeEndpoint { .. }
        | wire::Command::ConfigureBuilding { .. }
        | wire::Command::AttachEndpoint { .. }
        | wire::Command::SelectModel { .. }
        | wire::Command::CreateBuilding { .. }
        | wire::Command::RemoveBuilding { .. }
        | wire::Command::PutSecret { .. }
        | wire::Command::Reveal { .. }
        | wire::Command::RestoreDiscard { .. }
        | wire::Command::DoctorInstall { .. }
        | wire::Command::DoctorRefresh { .. }
        | wire::Command::SetAutonomy { .. }
        | wire::Command::Wake { .. }
        | wire::Command::PutDocument { .. }
        | wire::Command::PutIdentity { .. }
        | wire::Command::PutRules(_)
        | wire::Command::RestoreFile { .. }
        | wire::Command::ConfigureCity(_)
        | wire::Command::ConnectToolkit { .. }
        | wire::Command::PutPreferences { .. }
        | wire::Command::PutShelved { .. }
        | wire::Command::PutGuide { .. }
        | wire::Command::Auth { .. } => VerbClass::LocalOnly,
    }
}
