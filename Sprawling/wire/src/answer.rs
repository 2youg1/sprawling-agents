// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{
    Address, ApprovalItem, Autonomy, ClusterKey, FileChange, GitOid, Restoration, Ruling, Seq,
    TimeMs,
};
use serde::{Deserialize, Serialize};

use crate::command::HaltScope;
use crate::guide::GuideProgress;
use crate::preference::PreferencesAnswer;

mod agents;
mod automation;
mod building;
mod catalog;
mod commits;
mod config;
mod cost;
mod cost_of;
mod devices;
mod doctor;
mod document;
mod document_bytes;
mod document_versions;
mod endpoints;
mod evidence;
mod find;
mod git_status;
mod github;
mod history;
mod hunks;
mod identity;
mod known_hosts;
mod layers;
mod listing;
mod mcp_health;
mod metrics;
mod model_facts;
mod model_figures;
mod note;
mod prefix;
mod preview;
mod proposals;
mod range;
mod release;
mod rounds;
mod run_summary;
mod scanning;
mod sessions;
mod skills;
mod thinking;
mod toolkits;
mod usage;

pub use agents::{
    AgentCatalogAnswer, AgentLine, AgentOffer, AgentSource, AuthMethod, CatalogSnapshot, LoginKind,
    LoginState, PinState,
};
pub use automation::{AutomationAnswer, Cadence, ScheduledJob, WatchedSource};
pub use building::{ArchiveLine, BlockedLine, BuildingAnswer, BuildingDoc};
pub use building::{BuildingProgress, PlanRow, PursuitLine, TemplateAnswer};
pub use catalog::{CatalogAnswer, CatalogChange, CatalogSkill, CatalogTool, CatalogVersion};
pub use commits::{CommitAnswer, CommitAt, CommitsAnswer};
pub use config::{
    ConfigAnswer, ConfigLayer, SecondDomain, SettledEffort, SettledSearch, SettledSecond,
    SupplierAccounts, TuningDefaults,
};
pub use cost::{CostAnswer, Renewals, UnpricedCalls};
pub use cost_of::{CostOfAnswer, RUN_COSTS_MAX, RunCostsAnswer};
pub use devices::{DeviceId, DeviceLine, DevicesAnswer};
pub use doctor::{DoctorAbsence, DoctorAnswer, DoctorCore, DoctorFault, DoctorInstall, DoctorItem};
pub use doctor::{DoctorCoverage, DoctorCustody, DoctorCustodyLifetime, DoctorCustodyStore};
pub use doctor::{DoctorGuarantee, DoctorGuaranteeAxis, DoctorSandbox, DoctorSandboxArm};
pub use doctor::{DoctorNeed, DoctorState, DoctorTier, DoctorVerdict, DoctorVersion};
pub use doctor::{DoctorNewest, DoctorPack, DoctorUnread, DoctorUpstream};
pub use doctor::{DoctorSandboxMissing, SandboxArm};
pub use document::{Coverage, DocumentAnswer, DocumentBody, DocumentState, HeldDocument};
pub use document_bytes::{BYTES_WINDOW_MAX, BytesAnswer, ExportAnswer};
pub use document_versions::{DocumentVersion, VERSIONS_MAX, VersionSource, VersionsAnswer};
pub use endpoints::{AccountStatus, ChosenSummary, EndpointSummary, EndpointsAnswer, KeyState};
pub use evidence::{EvidenceAnswer, EvidenceItem, EvidenceKind, Picture};
pub use find::{FIND_MAX, FIND_WALK_MAX, FindAnswer, Walked};
pub use git_status::{Drift, GitStatusAnswer};
pub use github::{GithubLoginAnswer, GithubReading};
pub use history::{HistoryAnswer, HistoryRangeAnswer};
pub use hunks::{HunksAnswer, PatchLine, Withheld};
pub use identity::{IdentityAnswer, StatedIdentity};
pub use known_hosts::{KnownFace, KnownHost, KnownHostsAnswer};
pub use layers::{
    DefaultStanding, LayerDocument, LayerState, LayersAnswer, PromptKind, PromptLayer,
    ShippedDefault,
};
pub use listing::{Entry, EntryKind, ListingAnswer};
pub use mcp_health::{McpHealthAnswer, McpServerHealth, McpState, McpToolLine};
pub use metrics::MetricsAnswer;
pub use model_facts::{GenerationAnswer, ModelFactsSummary, ParameterAnswer};
pub use model_figures::{
    CeilingLine, FigureFrom, FiguresAsked, FiguresLine, ModelFiguresAnswer, WindowLine,
};
pub use note::{HandbackNote, Note, ReplyEnd, ReplyEnded, Speaker};
pub use prefix::{
    ContentAnswer, PrefixAnswer, PrefixSegment, PrefixSlot, PrefixSource, PromptShapeAnswer,
};
pub use preview::PreviewAnswer;
pub use proposals::{OfferedCard, OpenProposalsAnswer, ProposalCard, ProposalsAnswer};
pub use range::RangeAnswer;
pub use release::{InstallChannel, Registry, RegistryNewest, RegistryReading};
pub use release::{ReleaseAnswer, ReleaseLine, UpdateHint};
pub use rounds::{
    Call, Closing, FrozenNames, Opening, Outcome, Output, RoundsAnswer, Timing, Turn, Used,
};
pub use run_summary::{RunSummary, Waiting};
pub use scanning::{DoctorDrive, DoctorExclusion, DoctorScanning, DoctorUntold};
pub use sessions::{
    SESSION_PREVIEW_MAX, SESSIONS_MAX, SessionLine, SessionRef, SessionStart, SessionsAnswer,
};
pub use skills::{SkillLine, SkillShelf, SkillsAnswer};
pub use thinking::{EffortWord, OfferSource, Switch, ThinkingOffer};
pub use toolkits::{Standing, ToolkitLine, ToolkitsAnswer};
pub use usage::VersionAuthor;
pub use usage::{
    DayCount, ExportFormat, HeldSkill, McpServerUsage, McpToolUsage, McpUsageAnswer, McpUse,
};
pub use usage::{ShellCalls, ShellsAnswer, UsageExportAnswer, UsageKind, UseOutcome};
pub use usage::{SkillAudit, SkillUsageAnswer, SkillUsageLine, SkillUse, SkillVersion};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ChangesAnswer {
    pub base: GitOid,

    pub head: Option<GitOid>,
    pub files: Vec<FileChange>,
}

pub const HISTORY_MAX: u32 = 500;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CityAnswer {

    pub runs: Vec<RunSummary>,
    pub active: u64,
    pub frozen: u64,

    pub buildings: Vec<BuildingProgress>,

    pub pursuits: Vec<PursuitLine>,

    pub halted: Vec<HaltScope>,

    pub proved: Option<Seq>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ApprovalsAnswer {
    pub items: Vec<ApprovalItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Answer {
    History(Box<HistoryAnswer>),
    HistoryRange(Box<HistoryRangeAnswer>),
    Sessions(SessionsAnswer),
    Changes(ChangesAnswer),
    Commit(Box<CommitAnswer>),
    City(CityAnswer),
    Run(Option<Box<RunSummary>>),
    Approvals(ApprovalsAnswer),
    Cost(Box<CostAnswer>),
    Endpoints(EndpointsAnswer),
    KnownHosts(KnownHostsAnswer),
    AgentCatalog(Box<AgentCatalogAnswer>),

    AgentSpec(Box<AgentOffer>),
    Devices(Box<DevicesAnswer>),
    Building(Box<BuildingAnswer>),
    Inbox(InboxAnswer),
    Discards(DiscardAnswer),
    Registry(RegistryAnswer),
    Archive(ArchiveAnswer),
    Metrics(Box<MetricsAnswer>),
    Governance(GovernanceAnswer),
    Hunks(Box<HunksAnswer>),
    Rounds(Box<RoundsAnswer>),
    Evidence(EvidenceAnswer),
    CostOf(CostOfAnswer),
    RunCosts(RunCostsAnswer),
    Listing(ListingAnswer),
    Find(FindAnswer),
    Document(Box<DocumentAnswer>),
    Proposals(Box<ProposalsAnswer>),
    OpenProposals(OpenProposalsAnswer),
    Range(Box<RangeAnswer>),
    Versions(Box<VersionsAnswer>),
    Bytes(Box<BytesAnswer>),
    Export(Box<ExportAnswer>),
    Preview(Box<PreviewAnswer>),

    Reply(Box<documents::Laid>),
    Commits(CommitsAnswer),
    Doctor(Box<DoctorAnswer>),
    Upstream(Box<DoctorUpstream>),
    Prefix(Box<PrefixAnswer>),

    PromptShape(Box<PromptShapeAnswer>),
    Content(Box<ContentAnswer>),
    Skills(Box<SkillsAnswer>),

    Template(Box<TemplateAnswer>),

    Layers(Box<LayersAnswer>),

    Catalog(Box<CatalogAnswer>),
    GitStatus(Box<GitStatusAnswer>),
    McpHealth(Box<McpHealthAnswer>),
    SkillUsage(Box<SkillUsageAnswer>),
    McpUsage(Box<McpUsageAnswer>),
    UsageExport(Box<UsageExportAnswer>),
    Shells(Box<ShellsAnswer>),
    Toolkits(Box<ToolkitsAnswer>),
    Release(Box<ReleaseAnswer>),
    Preferences(Box<PreferencesAnswer>),
    Config(Box<ConfigAnswer>),
    Identity(Box<IdentityAnswer>),
    Automation(Box<AutomationAnswer>),
    GithubLogin(GithubLoginAnswer),
    Guide(GuideProgress),
    Privacy(Box<crate::PrivacyAnswer>),

    ModelFigures(ModelFiguresAnswer),

    Unavailable {
        query: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct GovernanceAnswer {
    pub autonomy: Autonomy,

    pub decided: Vec<Decision>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Decision {

    pub item: String,
    pub verdict: Ruling,

    pub cluster: ClusterKey,
    pub at: TimeMs,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct InboxAnswer {
    pub addr: Address,
    pub waiting: Vec<SignalLine>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SignalLine {
    pub id: String,
    pub kind: String,
    pub from: String,
    pub at: TimeMs,

    #[serde(default)]
    pub first_line: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DiscardAnswer {
    pub rows: Vec<DiscardLine>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DiscardLine {
    pub path: String,

    pub restoration: Option<Restoration>,
    pub at: TimeMs,
    pub restored: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RegistryAnswer {
    pub assets: Vec<RegistryLine>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RegistryLine {
    pub addr: Address,
    pub kind: String,
    pub subject: String,
    pub at: TimeMs,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ArchiveAnswer {
    pub needle: String,
    pub hits: Vec<ArchiveHit>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ArchiveHit {
    pub building: Address,
    pub kind: String,
    pub day: u64,
    pub subject: String,
}
