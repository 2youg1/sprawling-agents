// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use remote_access::door::VerbClass;

#[derive(Debug, Clone, PartialEq)]
pub(super) enum Passage {
    Judged(VerbClass),

    Greeting(wire::Hello),
}

pub(super) fn passage(frame: wire::ClientFrame) -> Passage {
    match frame {
        wire::ClientFrame::Hello(said) => Passage::Greeting(said),

        wire::ClientFrame::Ask(wire::Ask {
            query: wire::Query::Privacy,
            ..
        }) => Passage::Judged(VerbClass::LocalOnly),

        wire::ClientFrame::Ask(_) | wire::ClientFrame::Monitor(_) => {
            Passage::Judged(VerbClass::Read)
        }
        wire::ClientFrame::Command(command) => Passage::Judged(command_class(&command)),
    }
}

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
        | wire::Command::NameSession(_)
        | wire::Command::ChangeRunPolicy(_)
        | wire::Command::PutSpine { .. } => VerbClass::Act,
        wire::Command::ProbeEndpoint { .. }
        | wire::Command::ConfigureBuilding(_)
        | wire::Command::AttachEndpoint { .. }
        | wire::Command::SelectModel { .. }
        | wire::Command::CreateBuilding { .. }
        | wire::Command::RemoveBuilding { .. }
        | wire::Command::PutSecret { .. }
        | wire::Command::ForgetSecret(_)
        | wire::Command::Reveal { .. }
        | wire::Command::RestoreDiscard { .. }
        | wire::Command::DoctorInstall { .. }
        | wire::Command::DoctorRefresh { .. }
        | wire::Command::SetAutonomy { .. }
        | wire::Command::Wake { .. }
        | wire::Command::PutDocument { .. }
        | wire::Command::PutIdentity { .. }
        | wire::Command::PutRules(_)
        | wire::Command::SetWorkspaceChoice(_)
        | wire::Command::RestoreFile { .. }
        | wire::Command::ConfigureCity(_)
        | wire::Command::ConnectToolkit { .. }
        | wire::Command::PutPreferences { .. }
        | wire::Command::PutShelved { .. }
        | wire::Command::PutGuide { .. }
        | wire::Command::PutRange(_)
        | wire::Command::PutPrompt(_)
        | wire::Command::DecideProposals(_)
        | wire::Command::OpenRemoteDoor { .. }
        | wire::Command::ReplaceCityKey { .. }
        | wire::Command::ConfirmRemoteDoor { .. }
        | wire::Command::CloseRemoteDoor { .. }
        | wire::Command::PrivacyOperation { .. }
        | wire::Command::CloseCity(_)
        | wire::Command::AddAgent(_)
        | wire::Command::AgentLogin(_)
        | wire::Command::ForgetDevice(_)
        | wire::Command::Auth { .. } => VerbClass::LocalOnly,
    }
}
