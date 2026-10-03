// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a Query comes back as: one shape per view, and the closed set
//! of them.
//!
//! These are read shapes and nothing else. Every one is derived from the
//! city's own state by a projection above this crate, so nothing here
//! decides anything; what this module owns is that a page and a server
//! agree on the field names, and that `Unavailable` is a real answer —
//! a view this build does not evaluate yet says so by name rather than
//! returning an empty result a reader would mistake for an empty city.

use kernel::{
    Address, ApprovalItem, Autonomy, ClusterKey, FileChange, GitOid, Restoration, Ruling, Seq,
    TimeMs,
};
use serde::{Deserialize, Serialize};

use crate::command::HaltScope;
use crate::guide::GuideProgress;
use crate::preference::PreferencesAnswer;

mod automation;
mod building;
mod commits;
mod config;
mod cost;
mod cost_of;
mod doctor;
mod document;
mod document_bytes;
mod document_versions;
mod endpoints;
mod evidence;
mod find;
mod git_status;
mod github;
mod harnesses;
mod history;
mod hunks;
mod identity;
mod known_hosts;
mod listing;
mod mcp_health;
mod metrics;
mod model_facts;
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
mod toolkits;
mod usage;

pub use automation::{AutomationAnswer, Cadence, ScheduledJob, WatchedSource};
pub use building::{ArchiveLine, BlockedLine, BuildingAnswer, BuildingDoc};
pub use building::{BuildingProgress, PlanRow, PursuitLine};
pub use commits::{CommitAnswer, CommitAt, CommitsAnswer};
pub use config::{
    ConfigAnswer, ConfigLayer, SecondDomain, SettledEffort, SettledSecond, TuningDefaults,
};
pub use cost::{CostAnswer, UnpricedCalls};
pub use cost_of::{CostOfAnswer, RUN_COSTS_MAX, RunCostsAnswer};
pub use doctor::{DoctorAbsence, DoctorAnswer, DoctorCore, DoctorFault, DoctorInstall, DoctorItem};
pub use doctor::{DoctorCoverage, DoctorCustody, DoctorCustodyLifetime, DoctorCustodyStore};
pub use doctor::{DoctorGuarantee, DoctorGuaranteeAxis, DoctorSandbox, DoctorSandboxArm};
pub use doctor::{DoctorNeed, DoctorState, DoctorTier, DoctorVerdict, DoctorVersion};
pub use doctor::{DoctorNewest, DoctorPack, DoctorUnread, DoctorUpstream};
pub use doctor::{DoctorSandboxMissing, SandboxArm};
pub use document::{Coverage, DocumentAnswer, DocumentBody, DocumentState, HeldDocument};
pub use document_bytes::{BYTES_WINDOW_MAX, BytesAnswer, ExportAnswer};
pub use document_versions::{DocumentVersion, VERSIONS_MAX, VersionSource, VersionsAnswer};
pub use endpoints::{ChosenSummary, EndpointSummary, EndpointsAnswer};
pub use evidence::{EvidenceAnswer, EvidenceItem, EvidenceKind, Picture};
pub use find::{FIND_MAX, FIND_WALK_MAX, FindAnswer, Walked};
pub use git_status::{Drift, GitStatusAnswer};
pub use github::{GithubLoginAnswer, GithubReading};
pub use harnesses::{HarnessLine, HarnessState, HarnessesAnswer};
pub use history::{HistoryAnswer, HistoryRangeAnswer};
pub use hunks::{HunksAnswer, PatchLine, Withheld};
pub use identity::{IdentityAnswer, StatedIdentity};
pub use known_hosts::{KnownFace, KnownHost, KnownHostsAnswer};
pub use listing::{Entry, EntryKind, ListingAnswer};
pub use mcp_health::{McpHealthAnswer, McpServerHealth, McpState, McpToolLine};
pub use metrics::MetricsAnswer;
pub use model_facts::ModelFactsSummary;
pub use note::{HandbackNote, Note, ReplyEnd, ReplyEnded, Speaker};
pub use prefix::{ContentAnswer, PrefixAnswer, PrefixSegment, PrefixSlot, PrefixSource};
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
pub use sessions::{SESSION_PREVIEW_MAX, SESSIONS_MAX, SessionLine, SessionStart, SessionsAnswer};
pub use skills::{SkillLine, SkillShelf, SkillsAnswer};
pub use toolkits::{Standing, ToolkitLine, ToolkitsAnswer};
pub use usage::{
    DayCount, ExportFormat, HeldSkill, McpServerUsage, McpToolUsage, McpUsageAnswer, McpUse,
};
pub use usage::{ShellCalls, ShellsAnswer, UsageExportAnswer, UsageKind, UseOutcome};
pub use usage::{SkillAudit, SkillUsageAnswer, SkillUsageLine, SkillUse, SkillVersion};

/// What moved between two checkpoints, one row per file, path order.
///
/// Path order rather than size order: somebody looking for one file finds
/// it in the same place every time, and a list that reorders itself as
/// the numbers change cannot be scanned twice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ChangesAnswer {
    pub base: GitOid,
    /// Absent when the comparison ran against the working tree.
    pub head: Option<GitOid>,
    pub files: Vec<FileChange>,
}

/// The most records one `History` answer may carry. A page asking for
/// more gets this many; the whole ledger is not a thing to put on a
/// socket, and a limit the caller cannot exceed is one fewer way for a
/// client to make the server do unbounded work.
pub const HISTORY_MAX: u32 = 500;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CityAnswer {
    /// Every active run and the latest frozen few, in RunId order;
    /// `frozen` counts every frozen run, listed or not.
    pub runs: Vec<RunSummary>,
    pub active: u64,
    pub frozen: u64,
    /// One entry per building whose roadmap the city could read.
    pub buildings: Vec<BuildingProgress>,
    /// The standing goals this city is working towards, if any.
    pub pursuits: Vec<PursuitLine>,
    /// The scopes shut by `halt` and not yet released, in the shape a
    /// `halt` frame names them. A page that has just opened has no
    /// other way to learn that the city is stopped.
    ///
    /// The same type the command carries, not the ledger's string form:
    /// a reader that had to take `building:<addr>` apart would be
    /// writing a second copy of a grammar that already has one, and the
    /// first client to get it wrong reported every shut building as
    /// running.
    pub halted: Vec<HaltScope>,
    /// The last record of a history proved whole, when the proof has
    /// found it so; absent while a served city is still proving it, or
    /// after the proof found it broken (`crates/wire/spec/Answer.lean` §8-63).
    pub proved: Option<Seq>,
}

/// What is waiting for a person, as the Ledger recorded it.
///
/// The whole item travels rather than a summary of it. An interface that
/// groups identical questions needs the cluster key, and one that leads
/// with the longest wait needs the arrival time; a summary type that
/// dropped both forced the page to render every item as its own group
/// under a sentence nobody wrote. `tainted` needs no special carriage
/// here for the same reason - it is a field of the item itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ApprovalsAnswer {
    pub items: Vec<ApprovalItem>,
}

/// What a query returns. `Unavailable` is a real answer: a view this
/// build does not evaluate yet says so by name, rather than returning an
/// empty result a reader would mistake for an empty city.
///
/// `Eq` went when `History` arrived: a record's payload is arbitrary
/// JSON, and JSON has no total equality. Nothing compared two answers
/// for equality outside a test.
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
    Harnesses(HarnessesAnswer),
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
    /// A reply's text laid out (`crates/wire/spec/Answer/Preview.lean` §8-75): its closed blocks, and
    /// the bytes they cover.
    Reply(Box<documents::Laid>),
    Commits(CommitsAnswer),
    Doctor(Box<DoctorAnswer>),
    Upstream(Box<DoctorUpstream>),
    Prefix(Box<PrefixAnswer>),
    Content(Box<ContentAnswer>),
    Skills(Box<SkillsAnswer>),
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
    /// `reason` is the text of what stopped a view that tried to look
    /// (`crates/wire/spec/Server.lean` D47); absent when the view names
    /// the query alone.
    Unavailable {
        query: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
}

/// Who answers for this city, and what was answered on the person's
/// behalf.
///
/// One shape for both halves: a list of decisions with nobody named
/// beside it does not say whether the person delegated them, and a
/// delegation with nothing under it does not say whether it was ever
/// used.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct GovernanceAnswer {
    pub autonomy: Autonomy,
    /// Every approval this city has answered, oldest first — the order
    /// the ledger wrote them and the order a fold expects.
    ///
    /// The person's own answers are here too. A list holding only what
    /// somebody else decided would make "I answered this" and "nobody
    /// ever answered this" look the same, and who may answer is the
    /// other field of this same answer.
    pub decided: Vec<Decision>,
}

/// One approval, as it was answered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Decision {
    /// The `ApprovalId` that was answered.
    pub item: String,
    pub verdict: Ruling,
    /// What the person was shown when they answered: the answer covers
    /// the group, not the one row.
    pub cluster: ClusterKey,
    pub at: TimeMs,
}

/// What waits in one room, without taking it.
///
/// Folded from the ledger rather than read off the queue: a queue that
/// is read by being consumed cannot also be looked at, and a view that
/// consumed what it showed would be a view that changes the thing it
/// reports on.
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
    /// The first line of the signal's `text`; `None` when its payload
    /// carries no text (`crates/wire/spec/Reading.lean` D39).
    #[serde(default)]
    pub first_line: Option<String>,
}

/// The Recycle Bin: what was discarded and how each row gets back.
///
/// A row without a way back cannot be constructed upstream, so every
/// row here states one; `restored` says whether somebody already took
/// it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DiscardAnswer {
    pub rows: Vec<DiscardLine>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DiscardLine {
    pub path: String,
    /// The way back, as the record kept it - the plan itself, not a
    /// sentence about it.
    ///
    /// Composing the sentence is the interface's job and it already has
    /// one authority for it (`client/src/views/record/bin.svelte`); a server that
    /// also rendered the plan into words would be the second. `None`
    /// means the record names a scheme this build cannot read, which the
    /// interface shows as a row it will not invent an action for - the
    /// row is never dropped, because hiding a discarded thing is worse
    /// than admitting the plan is unreadable.
    pub restoration: Option<Restoration>,
    pub at: TimeMs,
    pub restored: bool,
}

/// What this city has decided is worth keeping.
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

/// Archive hits across every building, read from the shelves at the
/// moment of asking. The files are the authority; an index kept beside
/// them would be a second copy of what the disk says.
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
