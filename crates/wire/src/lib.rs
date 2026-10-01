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
mod named_frames;
mod preference;
mod reading;
#[cfg(feature = "server")]
mod reception;
mod reply;
#[cfg(feature = "server")]
mod server;

pub use aggregate::{Aggregate, CityLabel, Forwarded, Sighting, Upstream};
pub use answer::DoctorSandboxMissing;
pub use answer::HistoryAnswer;
pub use answer::HistoryRangeAnswer;
pub use answer::PlanRow;
pub use answer::PursuitLine;
pub use answer::RunSummary;
pub use answer::Used;
pub use answer::{Answer, ApprovalsAnswer, ArchiveLine, BlockedLine, BuildingProgress};
pub use answer::{ArchiveAnswer, ArchiveHit, DiscardAnswer, DiscardLine};
pub use answer::{BuildingAnswer, BuildingDoc};
pub use answer::{Call, Closing, Note, Opening, Outcome, Output, RoundsAnswer, Timing, Turn};
pub use answer::{ChangesAnswer, CommitAnswer, CommitAt, CommitsAnswer, HISTORY_MAX};
pub use answer::{ChosenSummary, CityAnswer, CostAnswer};
pub use answer::{ConfigAnswer, ConfigLayer, ModelFactsSummary};
pub use answer::{ContentAnswer, Drift, GitStatusAnswer, PrefixAnswer, PrefixSegment};
pub use answer::{CostOfAnswer, EvidenceAnswer, RUN_COSTS_MAX, RunCostsAnswer};
pub use answer::{Decision, GovernanceAnswer};
pub use answer::{DoctorAbsence, DoctorAnswer, DoctorCore, DoctorFault, DoctorInstall, DoctorItem};
pub use answer::{DoctorCoverage, DoctorCustody, DoctorCustodyLifetime, DoctorCustodyStore};
pub use answer::{DoctorGuarantee, DoctorGuaranteeAxis, DoctorSandbox, DoctorSandboxArm};
pub use answer::{DoctorNeed, DoctorState, DoctorTier, DoctorVerdict, DoctorVersion};
pub use answer::{DoctorNewest, DoctorPack, DoctorUnread, DoctorUpstream};
pub use answer::{DocumentAnswer, Entry, EntryKind, ListingAnswer};
pub use answer::{EndpointSummary, EndpointsAnswer, UnpricedCalls};
pub use answer::{EvidenceItem, EvidenceKind, Picture};
pub use answer::{HarnessLine, HarnessesAnswer, KnownFace, KnownHost, KnownHostsAnswer};
pub use answer::{HunksAnswer, PatchLine, Withheld};
pub use answer::{InboxAnswer, MetricsAnswer, RegistryAnswer, RegistryLine, SignalLine};
pub use answer::{McpHealthAnswer, McpServerHealth, McpState, McpToolLine};
pub use answer::{PrefixSlot, PrefixSource, SkillLine, SkillShelf, SkillsAnswer};
pub use answer::{ReleaseAnswer, ReleaseLine};
pub use answer::{SecondDomain, SettledEffort, SettledSecond, TuningDefaults};
pub use answer::{Standing, ToolkitLine, ToolkitsAnswer};
#[cfg(feature = "server")]
pub use assets::{AssetReply, ClientAssets, EmbeddedFile};
pub use auth::{Pairing, PairingToken, verify};
pub use carried_name::{ProviderName, TemplateName, ToolkitSlug};
pub use command::COMMAND_NAMES;
pub use command::{BodyOverride, EndpointTuning, HeaderPair};
pub use command::{Carry, Command, WireCommand};
pub use command::{GovernedDocument, HaltScope, NoSecret};
pub use command::{PursuitStep, Shelf, SpineDocument};
pub use control::{ControlVerdict, Intervention, classify};
#[cfg(feature = "schema")]
pub use frames::wire_schema;
pub use frames::{Answered, Ask, AskId, AskOutcome};
pub use frames::{ClientFrame, Delta, LiveOutput, OutputStream, ServerFrame};
pub use frames::{Hello, Query, Welcome};
pub use frames::{Lagged, LogLevel, LogLine, Monitoring, Sample, Watched};
pub use frames::{QUERY_NAMES, WIRE_V, schema_hash};
pub use kernel::{FileChange, How, Lines};
pub use preference::{Appearance, Chord, Chroma, Density, Face, Lang, Lighting, Motion};
pub use preference::{BODY_PX_MAX, BODY_PX_MIN, PreferencePatch, PreferencesAnswer};
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
pub use server::{bind, router, serve};

pub use kernel::WriteLimit;
pub use kernel::model::{AdmissionRequirement, LandingPolicy, Mode, RunPolicy, Window};
pub use kernel::{Address, ApprovalId, Autonomy, AxCode, AxError, B3Hash};
pub use kernel::{ApprovalClass, ApprovalItem, ClusterKey, Restoration};
pub use kernel::{BudgetUse, Locator, PlannedProgress, Progress, UnplannedProgress};
pub use kernel::{DialectKind, Effort, ModelTag};
pub use kernel::{EventDraft, EventKind, EventRecord, GitOid, IdemKey, RunId};
pub use kernel::{McpServer, McpTransport, SandboxLimits, ServerLabel};
pub use kernel::{NodeId, PursuitState, RoadmapStatus, WHOLE_PPB};
pub use kernel::{Payload, Sealed, Seq, TimeMs, Tokens, UsdMicros};
pub use kernel::{Ruling, SessionName, WriteDomain};
