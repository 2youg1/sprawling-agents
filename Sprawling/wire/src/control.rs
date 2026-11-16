// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxCode, AxError, RunId};

use crate::command::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intervention {
    Steer,
    Cancel,
    Halt,
    Release,
}

impl Intervention {

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Steer => "Steer",
            Self::Cancel => "Cancel",
            Self::Halt => "Halt",
            Self::Release => "Release",
        }
    }
}

#[derive(Debug)]
pub enum ControlVerdict {
    Intervene {
        verb: Intervention,

        run: Option<RunId>,

        must_write_handoff: bool,
    },
    NotAnIntervention,
    Refuse(AxError),
}

#[must_use]
pub fn classify(command: &Command) -> ControlVerdict {
    match *command {
        Command::Steer { run, ref text, .. } => {
            if text.trim().is_empty() {
                return ControlVerdict::Refuse(
                    AxError::failure(
                        AxCode::WireMismatch,
                        "steer a Run",
                        "the steer carries no text",
                    )
                    .with_recovery("say what should change, or use Cancel to stop the Run"),
                );
            }
            intervene(Intervention::Steer, Some(run))
        }
        Command::Cancel { run, .. } => intervene(Intervention::Cancel, Some(run)),
        Command::Halt { .. } => scope_intervention(Intervention::Halt),
        Command::Release { .. } => scope_intervention(Intervention::Release),
        Command::Dispatch { .. }
        | Command::Wake { .. }
        | Command::ProbeEndpoint { .. }
        | Command::ConfigureBuilding(_)
        | Command::AttachEndpoint { .. }
        | Command::SelectModel { .. }

        | Command::OpenSession { .. }
        | Command::CreateBuilding { .. }

        | Command::RemoveBuilding { .. }
        | Command::PutSecret { .. }

        | Command::Reveal { .. }

        | Command::RestoreDiscard { .. }

        | Command::DoctorInstall { .. }
        | Command::DoctorRefresh { .. }
        | Command::BatchByBuilding { .. }
        | Command::Approve { .. }

        | Command::HandOff { .. }
        | Command::SetAutonomy { .. }

        | Command::Pursue { .. }

        | Command::PutDocument { .. }
        | Command::PutIdentity { .. }

        | Command::PutRules(_)
        | Command::SetWorkspaceChoice(_)

        | Command::RestoreFile { .. }

        | Command::PutGuide { .. }
        | Command::ConfigureCity(_)

        | Command::PutSpine { .. }

        | Command::PutRange(_)
        | Command::PutPrompt(_)
        | Command::DecideProposals(_)

        | Command::PutPreferences { .. }
        | Command::PutShelved { .. }

        | Command::ConnectToolkit { .. }

        | Command::NameSession(_)
        | Command::ChangeRunPolicy(_)

        | Command::OpenRemoteDoor { .. }
        | Command::ReplaceCityKey { .. }
        | Command::ConfirmRemoteDoor { .. }
        | Command::CloseRemoteDoor { .. }

        | Command::PrivacyOperation { .. }

        | Command::ForgetSecret(_)

        | Command::CloseCity(_)
        | Command::AddAgent(_)
        | Command::AgentLogin(_)
        | Command::ForgetDevice(_)
        | Command::Auth { .. } => ControlVerdict::NotAnIntervention,
    }
}

fn intervene(verb: Intervention, run: Option<RunId>) -> ControlVerdict {
    ControlVerdict::Intervene {
        verb,
        run,
        must_write_handoff: true,
    }
}

fn scope_intervention(verb: Intervention) -> ControlVerdict {
    ControlVerdict::Intervene {
        verb,
        run: None,
        must_write_handoff: false,
    }
}
