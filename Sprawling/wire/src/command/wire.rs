// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::IdemKey;

use super::kind::Command;
use super::no_secret::NoSecret;
use super::step::{
    AgentAdding, AgentLoginStart, BuildingSettings, CityClosing, CitySettings, DeviceForgetting,
    DoorAnswer, DoorOpening, DoorStep, PolicyChange, PromptWrite, ProposalDecisions, RangeWrite,
    RulesWrite, SecretForgetting, SessionNaming, WorkspaceChoiceWrite,
};

pub type WireCommand = Command<NoSecret>;

impl<Secret> Command<Secret> {

    pub fn idem(&self) -> Option<&IdemKey> {
        match *self {
            Self::Dispatch { ref idem, .. }
            | Self::Wake { ref idem, .. }
            | Self::OpenSession { ref idem, .. }
            | Self::ProbeEndpoint { ref idem, .. }
            | Self::ConfigureBuilding(BuildingSettings { ref idem, .. })
            | Self::CreateBuilding { ref idem, .. }
            | Self::RemoveBuilding { ref idem, .. }
            | Self::Steer { ref idem, .. }
            | Self::Cancel { ref idem, .. }
            | Self::Halt { ref idem, .. }
            | Self::Release { ref idem, .. }
            | Self::BatchByBuilding { ref idem, .. }
            | Self::Approve { ref idem, .. }
            | Self::HandOff { ref idem, .. }
            | Self::SetAutonomy { ref idem, .. }
            | Self::Pursue { ref idem, .. }
            | Self::PutDocument { ref idem, .. }
            | Self::PutIdentity { ref idem, .. }
            | Self::PutRules(RulesWrite { ref idem, .. })
            | Self::SetWorkspaceChoice(WorkspaceChoiceWrite { ref idem, .. })
            | Self::ConfigureCity(CitySettings { ref idem, .. })
            | Self::RestoreFile { ref idem, .. }
            | Self::PutGuide { ref idem, .. }
            | Self::PutSpine { ref idem, .. }
            | Self::PutRange(RangeWrite { ref idem, .. })
            | Self::PutPrompt(PromptWrite { ref idem, .. })
            | Self::DecideProposals(ProposalDecisions { ref idem, .. })
            | Self::PutPreferences { ref idem, .. }
            | Self::PutShelved { ref idem, .. }
            | Self::AttachEndpoint { ref idem, .. }
            | Self::SelectModel { ref idem, .. }
            | Self::Reveal { ref idem, .. }
            | Self::RestoreDiscard { ref idem, .. }
            | Self::DoctorInstall { ref idem, .. }
            | Self::DoctorRefresh { ref idem, .. }
            | Self::ConnectToolkit { ref idem, .. }
            | Self::NameSession(SessionNaming { ref idem, .. })
            | Self::ChangeRunPolicy(PolicyChange { ref idem, .. })
            | Self::OpenRemoteDoor(DoorOpening { ref idem, .. })
            | Self::ConfirmRemoteDoor(DoorAnswer { ref idem, .. })
            | Self::ReplaceCityKey(DoorStep { ref idem })
            | Self::CloseRemoteDoor(DoorStep { ref idem })
            | Self::PrivacyOperation(crate::PrivacyRequest { ref idem, .. })
            | Self::ForgetSecret(SecretForgetting { ref idem, .. })
            | Self::CloseCity(CityClosing { ref idem, .. })
            | Self::AddAgent(AgentAdding { ref idem, .. })
            | Self::AgentLogin(AgentLoginStart { ref idem, .. })
            | Self::ForgetDevice(DeviceForgetting { ref idem, .. }) => Some(idem),
            Self::PutSecret { .. } | Self::Auth { .. } => None,
        }
    }
}

impl From<WireCommand> for Command {

    fn from(wire: WireCommand) -> Self {
        match wire {

            #[rustfmt::skip]
            Command::Wake { source, subject, body, idem } => Self::Wake { source, subject, body, idem },
            Command::Dispatch {
                addr,
                task,
                goal,
                policy,
                idem,
                session,
                effort,
                model,
            } => Self::Dispatch {
                addr,
                task,
                goal,
                policy,
                idem,
                session,
                effort,
                model,
            },
            Command::ConfigureBuilding(settings) => Self::ConfigureBuilding(settings),
            Command::ProbeEndpoint {
                name,
                base_url,
                dialect,
                secret,
                auth_header,
                tuning,
                idem,
            } => Self::ProbeEndpoint {
                name,
                base_url,
                dialect,
                secret,
                auth_header,
                tuning,
                idem,
            },
            Command::AttachEndpoint {
                name,
                base_url,
                dialect,
                secret,
                auth_header,
                admit,
                named,
                tuning,
                idem,
            } => Self::AttachEndpoint {
                name,
                base_url,
                dialect,
                secret,
                auth_header,
                admit,
                named,
                tuning,
                idem,
            },
            Command::SelectModel {
                endpoint,
                model,
                tag,
                context_tokens,
                max_output_tokens,
                input,
                idem,
            } => Self::SelectModel {
                endpoint,
                model,
                tag,
                context_tokens,
                max_output_tokens,
                input,
                idem,
            },
            Command::OpenSession {
                addr,
                carry,
                from,
                idem,
            } => Self::OpenSession {
                addr,
                carry,
                from,
                idem,
            },

            #[rustfmt::skip]
            Command::CreateBuilding { addr, template, rules, source, idem } =>
                Self::CreateBuilding { addr, template, rules, source, idem },
            Command::PutSecret { value, .. } => match value {},
            Command::Steer { run, text, idem } => Self::Steer { run, text, idem },
            Command::Cancel { run, idem } => Self::Cancel { run, idem },
            Command::Halt { scope, idem } => Self::Halt { scope, idem },
            Command::Reveal { at, idem } => Self::Reveal { at, idem },
            Command::RestoreDiscard { restoration, idem } => {
                Self::RestoreDiscard { restoration, idem }
            }
            Command::RemoveBuilding { addr, idem } => Self::RemoveBuilding { addr, idem },
            Command::DoctorInstall { item, idem } => Self::DoctorInstall { item, idem },
            Command::DoctorRefresh { idem } => Self::DoctorRefresh { idem },
            Command::ConnectToolkit { toolkit, idem } => Self::ConnectToolkit { toolkit, idem },
            Command::Release { scope, idem } => Self::Release { scope, idem },
            Command::BatchByBuilding { addr, idem } => Self::BatchByBuilding { addr, idem },
            Command::Approve {
                item,
                verdict,
                idem,
            } => Self::Approve {
                item,
                verdict,
                idem,
            },
            Command::HandOff { item, to, idem } => Self::HandOff { item, to, idem },
            Command::Pursue { addr, step, idem } => Self::Pursue { addr, step, idem },
            Command::SetAutonomy {
                scope,
                autonomy,
                idem,
            } => Self::SetAutonomy {
                scope,
                autonomy,
                idem,
            },
            Command::PutDocument {
                which,
                base,
                body,
                idem,
            } => Self::PutDocument {
                which,
                base,
                body,
                idem,
            },
            Command::PutIdentity { card, base, idem } => Self::PutIdentity { card, base, idem },
            Command::PutRules(write) => Self::PutRules(write),
            Command::SetWorkspaceChoice(write) => Self::SetWorkspaceChoice(write),
            Command::RestoreFile { at, point, idem } => Self::RestoreFile { at, point, idem },
            Command::PutGuide { progress, idem } => Self::PutGuide { progress, idem },
            Command::ConfigureCity(settings) => Self::ConfigureCity(settings),
            Command::PutRange(write) => Self::PutRange(write),
            Command::PutPrompt(write) => Self::PutPrompt(write),
            Command::DecideProposals(decisions) => Self::DecideProposals(decisions),
            Command::PutSpine {
                building,
                which,
                base,
                body,
                idem,
            } => Self::PutSpine {
                building,
                which,
                base,
                body,
                idem,
            },
            Command::PutPreferences { patch, idem } => Self::PutPreferences { patch, idem },
            Command::NameSession(naming) => Self::NameSession(naming),
            Command::ChangeRunPolicy(change) => Self::ChangeRunPolicy(change),
            Command::OpenRemoteDoor(opening) => Self::OpenRemoteDoor(opening),
            Command::ReplaceCityKey(step) => Self::ReplaceCityKey(step),
            Command::ConfirmRemoteDoor(answer) => Self::ConfirmRemoteDoor(answer),
            Command::CloseRemoteDoor(step) => Self::CloseRemoteDoor(step),
            Command::PrivacyOperation(request) => Self::PrivacyOperation(request),
            Command::ForgetSecret(forgetting) => Self::ForgetSecret(forgetting),
            Command::CloseCity(closing) => Self::CloseCity(closing),
            Command::AddAgent(adding) => Self::AddAgent(adding),
            Command::AgentLogin(login) => Self::AgentLogin(login),
            Command::ForgetDevice(forgetting) => Self::ForgetDevice(forgetting),
            #[rustfmt::skip]
            Command::PutShelved { shelf, name, text, idem } => Self::PutShelved { shelf, name, text, idem },
            Command::Auth { token } => Self::Auth { token },
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::float_arithmetic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests {
    use super::*;

    #[test]
    fn a_wire_frame_carrying_put_secret_fails_to_decode() {
        let json = r#"{"put_secret":{"realm":"anthropic","name":"api","value":"sk-nope"}}"#;
        let decoded: Result<WireCommand, _> = serde_json::from_str(json);
        let err = decoded.expect_err("PutSecret has no wire form");
        assert!(
            !err.to_string().contains("sk-nope"),
            "the refusal must not echo the bytes it protects"
        );
    }
}
