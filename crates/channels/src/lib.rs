// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Process boundary: Command/Query/Event wire, WebSocket server, auth,
//! read-only multi-city aggregation.
//!
//! The kernel types that appear on this crate's own signatures are
//! re-exported below. `web` depends on `channels` and on nothing else
//! (ARCHITECTURE section 2), so a client that cannot name `EventRecord`
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
mod named_frames;
mod preference;
mod reading;
#[cfg(feature = "server")]
mod reception;
#[cfg(feature = "server")]
mod server;
mod wire;

pub use aggregate::{Aggregate, CityLabel, Forwarded, Sighting, Upstream};
pub use answer::DoctorSandboxMissing;
pub use answer::HistoryRangeAnswer;
pub use answer::PlanRow;
pub use answer::PursuitLine;
pub use answer::RunSummary;
pub use answer::{Answer, ApprovalsAnswer, ArchiveLine, BlockedLine, BuildingProgress};
pub use answer::{ArchiveAnswer, ArchiveHit, DiscardAnswer, DiscardLine};
pub use answer::{BuildingAnswer, BuildingDoc};
pub use answer::{Call, Closing, Note, Opening, Outcome, Output, RoundsAnswer, Turn, Used};
pub use answer::{ChangesAnswer, CommitAnswer, CommitsAnswer, HISTORY_MAX, HistoryAnswer};
pub use answer::{ChosenSummary, CityAnswer, CostAnswer, EndpointSummary, EndpointsAnswer};
pub use answer::{ConfigAnswer, ConfigLayer, ModelFactsSummary, SettledEffort, TuningDefaults};
pub use answer::{ContentAnswer, Drift, GitStatusAnswer, PrefixAnswer, PrefixSegment};
pub use answer::{CostOfAnswer, EvidenceAnswer, EvidenceItem, EvidenceKind, Picture};
pub use answer::{Decision, GovernanceAnswer};
pub use answer::{DoctorAbsence, DoctorAnswer, DoctorFault, DoctorInstall, DoctorItem};
pub use answer::{DoctorCoverage, DoctorCustody, DoctorCustodyLifetime, DoctorCustodyStore};
pub use answer::{DoctorGuarantee, DoctorGuaranteeAxis, DoctorSandbox, DoctorSandboxArm};
pub use answer::{DoctorNeed, DoctorState, DoctorTier, DoctorVerdict, DoctorVersion};
pub use answer::{DocumentAnswer, Entry, EntryKind, ListingAnswer};
pub use answer::{HunksAnswer, PatchLine, Withheld};
pub use answer::{InboxAnswer, MetricsAnswer, RegistryAnswer, RegistryLine, SignalLine};
pub use answer::{McpHealthAnswer, McpServerHealth, McpState, McpToolLine};
pub use answer::{PrefixSlot, PrefixSource, SkillLine, SkillShelf, SkillsAnswer};
pub use answer::{ReleaseAnswer, ReleaseLine};
pub use answer::{Standing, ToolkitLine, ToolkitsAnswer};
#[cfg(feature = "server")]
pub use assets::{AssetReply, ClientAssets, EmbeddedFile};
pub use auth::{PairingToken, verify};
pub use carried_name::{ProviderName, TemplateName, ToolkitSlug};
pub use command::COMMAND_NAMES;
pub use command::{BodyOverride, EndpointTuning, HeaderPair};
pub use command::{Command, WireCommand};
pub use command::{GovernedDocument, HaltScope, LoginStep, NoSecret, PursuitStep, Shelf};
pub use control::{ControlVerdict, Intervention, classify};
pub use kernel::highlight::markdown;
pub use kernel::{FileChange, How, Lines};
pub use kernel::{Span, Token};
pub use preference::{Appearance, Chord, Chroma, Density, Face, Lang, Lighting, Motion};
pub use preference::{BODY_PX_MAX, BODY_PX_MIN, PreferencePatch, PreferencesAnswer};
pub use reading::{OUTPUT_LINES, arguments_in, note_of, output_in, said_in, subject_of};
pub use reading::{text, thought_in, used_in};
#[cfg(feature = "server")]
pub use reception::{Admission, BindFace, BindVerdict, Door, HandshakeVerdict, Pairing};
#[cfg(feature = "server")]
pub use reception::{SessionState, SessionStep, decide_frame};
#[cfg(feature = "server")]
pub use reception::{decide_admission, offered_pairing};
#[cfg(feature = "server")]
pub use reception::{decide_bind, decide_handshake};
#[cfg(feature = "server")]
pub use server::{AcpProgress, AcpSink, TranscribeSink};
#[cfg(feature = "server")]
pub use server::{Delivered, Reply, ServeConfig, router, serve};
#[cfg(feature = "schema")]
pub use wire::wire_schema;
pub use wire::{ClientFrame, Delta, ServerFrame};
pub use wire::{Hello, Query, Welcome};
pub use wire::{Lagged, LogLevel, LogLine};
pub use wire::{QUERY_NAMES, WIRE_V, schema_hash};

pub use kernel::model::{Mode, Window};
pub use kernel::{Address, ApprovalId, Autonomy, AxCode, AxError, B3Hash};
pub use kernel::{ApprovalClass, ApprovalItem, ClusterKey, Restoration};
pub use kernel::{BudgetUse, Locator, PlannedProgress, Progress, UnplannedProgress};
pub use kernel::{DialectKind, Effort, ModelTag};
pub use kernel::{EventDraft, EventKind, EventRecord, GitOid, IdemKey, RunId};
pub use kernel::{McpServer, McpTransport, SandboxLimits, ServerLabel};
pub use kernel::{NodeId, PursuitState, RoadmapStatus, WHOLE_PPB};
pub use kernel::{Payload, Sealed, Seq, TimeMs, Tokens, UsdMicros};
pub use kernel::{Ruling, SessionName, WriteDomain};
