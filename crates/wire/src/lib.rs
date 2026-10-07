// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Process boundary: Command/Query/Event wire, WebSocket server, auth,
//! read-only multi-city aggregation.
//!
//! The kernel types that appear on this crate's own signatures are
//! re-exported below. The client reads the wire and nothing else
//! (ARCHITECTURE section 8), so a client that cannot name `EventRecord`
//! cannot read the frames it is sent; a boundary crate that hands out
//! frames owes the vocabulary to read them.

mod aggregate;
mod answer;
#[cfg(feature = "server")]
mod assets;
mod auth;
mod carried_name;
mod command;
mod control;
mod frames;
mod guide;
mod named_frames;
mod preference;
mod reading;
#[cfg(feature = "server")]
mod reception;
mod reply;
#[cfg(feature = "server")]
mod server;

pub use aggregate::{Aggregate, CityLabel, Forwarded, Sighting, Upstream};
pub use answer::HistoryAnswer;
pub use answer::HistoryRangeAnswer;
pub use answer::PlanRow;
pub use answer::PursuitLine;
pub use answer::TuningDefaults;
pub use answer::Used;
pub use answer::VersionAuthor;
pub use answer::{AccountStatus, EndpointSummary, EndpointsAnswer, KeyState, UnpricedCalls};
pub use answer::{Answer, ApprovalsAnswer, ArchiveLine, BlockedLine, BuildingProgress};
pub use answer::{ArchiveAnswer, ArchiveHit, DiscardAnswer, DiscardLine};
pub use answer::{AutomationAnswer, Cadence, ScheduledJob, WatchedSource};
pub use answer::{BYTES_WINDOW_MAX, BytesAnswer, DocumentVersion, ExportAnswer};
pub use answer::{BuildingAnswer, BuildingDoc};
pub use answer::{Call, Closing, FrozenNames, Note, Opening, Outcome, Output};
pub use answer::{ChangesAnswer, CommitAnswer, CommitAt, CommitsAnswer, HISTORY_MAX};
pub use answer::{ChosenSummary, CityAnswer, CostAnswer};
pub use answer::{ConfigAnswer, ConfigLayer, ModelFactsSummary};
pub use answer::{ContentAnswer, Drift, GitStatusAnswer, PrefixAnswer, PrefixSegment};
pub use answer::{CostOfAnswer, EvidenceAnswer, RUN_COSTS_MAX, RunCostsAnswer};
pub use answer::{Coverage, DocumentAnswer, DocumentBody, DocumentState, HeldDocument};
pub use answer::{DayCount, ExportFormat, HeldSkill, McpServerUsage, McpToolUsage, McpUse};
pub use answer::{Decision, GovernanceAnswer};
pub use answer::{DoctorAbsence, DoctorAnswer, DoctorCore, DoctorFault, DoctorInstall, DoctorItem};
pub use answer::{DoctorCoverage, DoctorCustody, DoctorCustodyLifetime, DoctorCustodyStore};
pub use answer::{DoctorDrive, DoctorExclusion, DoctorScanning, DoctorUntold};
pub use answer::{DoctorGuarantee, DoctorGuaranteeAxis, DoctorSandbox, DoctorSandboxArm};
pub use answer::{DoctorNeed, DoctorState, DoctorTier, DoctorVerdict, DoctorVersion};
pub use answer::{DoctorNewest, DoctorPack, DoctorUnread, DoctorUpstream};
pub use answer::{DoctorSandboxMissing, SandboxArm};
pub use answer::{Entry, EntryKind, ListingAnswer, PreviewAnswer, RangeAnswer};
pub use answer::{EvidenceItem, EvidenceKind, Picture};
pub use answer::{FIND_MAX, FIND_WALK_MAX, FindAnswer, Walked};
pub use answer::{GithubLoginAnswer, GithubReading};
pub use answer::{HandbackNote, ReplyEnd, ReplyEnded, Speaker};
pub use answer::{HarnessLine, HarnessState, HarnessesAnswer};
pub use answer::{HunksAnswer, PatchLine, Withheld};
pub use answer::{IdentityAnswer, StatedIdentity};
pub use answer::{InboxAnswer, MetricsAnswer, RegistryAnswer, RegistryLine, SignalLine};
pub use answer::{InstallChannel, Registry, RegistryNewest, RegistryReading};
pub use answer::{KnownFace, KnownHost, KnownHostsAnswer};
pub use answer::{McpHealthAnswer, McpServerHealth, McpState, McpToolLine, McpUsageAnswer};
pub use answer::{OfferedCard, OpenProposalsAnswer, ProposalCard, ProposalsAnswer};
pub use answer::{PrefixSlot, PrefixSource, SkillLine, SkillShelf, SkillsAnswer};
pub use answer::{ReleaseAnswer, ReleaseLine, UpdateHint};
pub use answer::{RoundsAnswer, Timing, Turn};
pub use answer::{RunSummary, Waiting};
pub use answer::{SESSION_PREVIEW_MAX, SESSIONS_MAX, SessionLine, SessionStart, SessionsAnswer};
pub use answer::{SecondDomain, SettledEffort, SettledSearch, SettledSecond, SupplierAccounts};
pub use answer::{ShellCalls, ShellsAnswer, UsageExportAnswer, UsageKind, UseOutcome};
pub use answer::{SkillAudit, SkillUsageAnswer, SkillUsageLine, SkillUse, SkillVersion};
pub use answer::{Standing, ToolkitLine, ToolkitsAnswer};
pub use answer::{VERSIONS_MAX, VersionSource, VersionsAnswer};
#[cfg(feature = "server")]
pub use assets::{AssetReply, ClientAssets, EmbeddedFile};
pub use auth::{Pairing, PairingToken, verify};
pub use carried_name::{ProviderName, TemplateName, ToolkitSlug};
pub use command::COMMAND_NAMES;
pub use command::{BodyOverride, EndpointTuning, HeaderPair};
pub use command::{Carry, Command, WireCommand};
pub use command::{CitySettings, GovernedDocument, HaltScope, IdentityCard, NoSecret, RulesWrite};
pub use command::{DoorAnswer, DoorOpening, DoorStep, SecretForgetting};
pub use command::{PolicyChange, ProposalDecision, ProposalDecisions, RangeWrite, SessionNaming};
pub use command::{PursuitStep, Shelf, SpineDocument};
pub use control::{ControlVerdict, Intervention, classify};
#[cfg(feature = "schema")]
pub use frames::wire_schema;
pub use frames::{Answered, Ask, AskId, AskOutcome};
pub use frames::{BEAT_MAX_MS, BEAT_MIN_MS, BeatMs};
pub use frames::{ClientFrame, Delta, LiveOutput, OutputStream, ServerFrame};
pub use frames::{Hello, Query, Welcome};
pub use frames::{Lagged, LogLevel, LogLine, Monitoring, Sample, Watched};
pub use frames::{QUERY_NAMES, WIRE_V, schema_hash};
pub use guide::{GuideMark, GuideProgress, GuideState, GuideStep};
pub use kernel::{FileChange, How, Lines};
pub use preference::{Appearance, Chord, Chroma, Density, Face, Lang, Lighting, Motion};
pub use preference::{BODY_PX_MAX, BODY_PX_MIN, PreferencePatch, PreferencesAnswer};
pub use preference::{CorePlacement, CorePreferences, CorePriority};
pub use preference::{Glass, ThemeOverride, Tier};
pub use preference::{SessionTags, TAG_MAX, Tag};
pub use reading::{OUTPUT_LINES, arguments_in, note_of, output_in, said_in};
pub use reading::{text, thought_in, used_in};
#[cfg(feature = "server")]
pub use reception::{Admission, BindFace, BindVerdict, Door, HandshakeVerdict};
#[cfg(feature = "server")]
pub use reception::{SessionState, SessionStep, WelcomeFacts, decide_frame};
#[cfg(feature = "server")]
pub use reception::{decide_admission, offered_pairing};
#[cfg(feature = "server")]
pub use reception::{decide_bind, decide_handshake};
pub use reply::{AcpProgress, Delivered, Reply};
#[cfg(feature = "server")]
pub use server::{AcpSink, Answering, MonitorFeed, TranscribeSink};
#[cfg(feature = "server")]
pub use server::{Bound, Committed, LedgerHead, ServeConfig};
#[cfg(feature = "server")]
pub use server::{DROP_BYTES_MAX, DropSink};
#[cfg(feature = "server")]
pub use server::{bind, bundle_routes, router, serve};

pub use kernel::WriteLimit;
pub use kernel::model::{AdmissionRequirement, LandingPolicy, Mode, RunPolicy, Window};
pub use kernel::{Address, ApprovalId, Autonomy, AxCode, AxError, B3Hash};
pub use kernel::{ApprovalClass, ApprovalItem, ClusterKey, Restoration};
pub use kernel::{BudgetUse, Locator, PlannedProgress, Progress, UnplannedProgress};
pub use kernel::{DialectKind, Effort, KeepWarm, ModelTag};
pub use kernel::{EventDraft, EventKind, EventRecord, GitOid, IdemKey, RunId};
pub use kernel::{McpServer, McpTransport, SandboxLimits, ServerLabel};
pub use kernel::{NodeId, PursuitState, RoadmapStatus, WHOLE_PPB};
pub use kernel::{Payload, Sealed, Seq, TimeMs, Tokens, UsdMicros};
pub use kernel::{Ruling, SessionName, WriteDomain};
