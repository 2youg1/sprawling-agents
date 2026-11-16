// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::model::{RunPolicy, Window};
use kernel::{
    Address, ApprovalId, Autonomy, Ceiling, DialectKind, Effort, GitOid, IdemKey, InputKinds,
    ModelTag, Origin, ResidentId, Restoration, Ruling, RunId, Sealed, SessionName,
};
use serde::{Deserialize, Serialize};

use crate::carried_name::{ProviderName, TemplateName, ToolkitSlug};
use crate::command::shelf::Shelf;
use crate::command::step::{
    BuildingSettings, Carry, CitySettings, DoorAnswer, DoorOpening, DoorStep, GovernedDocument,
    HaltScope, IdentityCard, PolicyChange, PromptWrite, ProposalDecisions, PursuitStep, RangeWrite,
    RulesWrite, SecretForgetting, SessionNaming, SpineDocument, WorkspaceChoiceWrite,
};
use crate::command::tuning::EndpointTuning;
use crate::guide::GuideProgress;
use crate::named_frames::named_frames;
use crate::preference::PreferencePatch;

named_frames! {

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "schema", schemars(rename = "Command"))]
pub enum Command<Secret = Sealed<String>> {
    Dispatch {
        addr: Address,
        task: String,
        goal: String,

        policy: RunPolicy,
        idem: IdemKey,

        session: Option<SessionName>,

        effort: Option<Effort>,

        model: Option<String>,
    },

    ProbeEndpoint {
        name: ProviderName,
        base_url: String,
        dialect: DialectKind,
        secret: Option<String>,
        auth_header: Option<String>,
        tuning: EndpointTuning,
        idem: IdemKey,
    },

    ConfigureBuilding(BuildingSettings),
    AttachEndpoint {
        name: ProviderName,

        base_url: String,
        dialect: DialectKind,

        secret: Option<String>,

        auth_header: Option<String>,

        admit: Vec<String>,

        named: Vec<String>,

        tuning: EndpointTuning,
        idem: IdemKey,
    },

    SelectModel {
        endpoint: ProviderName,
        model: String,
        tag: ModelTag,

        context_tokens: Option<Window>,

        max_output_tokens: Option<Ceiling>,

        input: Option<InputKinds>,
        idem: IdemKey,
    },

    OpenSession {
        addr: Address,
        carry: Carry,

        from: Option<Origin>,
        idem: IdemKey,
    },

    CreateBuilding {
        addr: Address,
        template: TemplateName,

        rules: Option<String>,

        #[serde(default)]
        source: Option<String>,
        idem: IdemKey,
    },

    RemoveBuilding {
        addr: Address,
        idem: IdemKey,
    },

    PutSecret {
        realm: String,
        name: String,
        value: Secret,
    },
    Steer {
        run: RunId,
        text: String,
        idem: IdemKey,
    },
    Cancel {
        run: RunId,
        idem: IdemKey,
    },
    Halt {
        scope: HaltScope,
        idem: IdemKey,
    },

    Reveal {
        at: Address,
        idem: IdemKey,
    },

    RestoreDiscard {
        restoration: Restoration,
        idem: IdemKey,
    },

    DoctorInstall {
        item: String,
        idem: IdemKey,
    },

    DoctorRefresh {
        idem: IdemKey,
    },
    Release {
        scope: HaltScope,
        idem: IdemKey,
    },
    BatchByBuilding {
        addr: Address,
        idem: IdemKey,
    },
    Approve {
        item: ApprovalId,
        verdict: Ruling,
        idem: IdemKey,
    },

    HandOff {
        item: ApprovalId,
        to: ResidentId,
        idem: IdemKey,
    },
    SetAutonomy {
        scope: HaltScope,
        autonomy: Autonomy,
        idem: IdemKey,
    },

    Pursue {
        addr: Address,
        step: PursuitStep,
        idem: IdemKey,
    },

    Wake {
        source: String,
        subject: String,
        body: String,
        idem: IdemKey,
    },

    PutDocument {
        which: GovernedDocument,
        base: String,
        body: String,
        idem: IdemKey,
    },

    PutIdentity {
        card: IdentityCard,
        base: String,
        idem: IdemKey,
    },

    PutRules(RulesWrite),

    SetWorkspaceChoice(WorkspaceChoiceWrite),

    RestoreFile {
        at: Address,
        point: GitOid,
        idem: IdemKey,
    },

    PutGuide {
        progress: GuideProgress,
        idem: IdemKey,
    },

    ConfigureCity(CitySettings),

    PutSpine {
        building: Address,
        which: SpineDocument,
        base: String,
        body: String,
        idem: IdemKey,
    },

    PutRange(RangeWrite),

    DecideProposals(ProposalDecisions),

    ConnectToolkit {
        toolkit: ToolkitSlug,
        idem: IdemKey,
    },

    PutPreferences {
        patch: PreferencePatch,
        idem: IdemKey,
    },

    PutShelved {
        shelf: Shelf,
        name: String,
        text: String,
        idem: IdemKey,
    },

    NameSession(SessionNaming),

    ChangeRunPolicy(PolicyChange),
    OpenRemoteDoor(DoorOpening),
    ReplaceCityKey(DoorStep),
    ConfirmRemoteDoor(DoorAnswer),
    CloseRemoteDoor(DoorStep),
    PrivacyOperation(crate::PrivacyRequest),

    ForgetSecret(SecretForgetting),

    CloseCity(crate::CityClosing),
    AddAgent(crate::AgentAdding),
    AgentLogin(crate::AgentLoginStart),
    ForgetDevice(crate::DeviceForgetting),

    PutPrompt(PromptWrite),

    Auth {
        token: String,
    },
}

pub const COMMAND_NAMES;
}
