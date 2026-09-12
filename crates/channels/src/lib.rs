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
mod reading;
#[cfg(feature = "server")]
mod reception;
#[cfg(feature = "server")]
mod server;
mod wire;

pub use aggregate::{Aggregate, CityLabel, Forwarded, Sighting, Upstream};
pub use answer::PlanRow;
pub use answer::PursuitLine;
pub use answer::RunSummary;
pub use answer::{Answer, ApprovalsAnswer, ArchiveLine, BlockedLine, BuildingProgress};
pub use answer::{ArchiveAnswer, ArchiveHit, DiscardAnswer, DiscardLine};
pub use answer::{BuildingAnswer, BuildingDoc};
pub use answer::{Call, Closing, Note, Opening, Outcome, Output, RoundsAnswer, Turn, Used};
pub use answer::{ChangesAnswer, CommitAnswer, CommitsAnswer, HISTORY_MAX, HistoryAnswer};
pub use answer::{ChosenSummary, CityAnswer, CostAnswer, EndpointSummary, EndpointsAnswer};
pub use answer::{ContentAnswer, Drift, GitStatusAnswer, PrefixAnswer, PrefixSegment};
pub use answer::{CostOfAnswer, EvidenceAnswer, EvidenceItem, EvidenceKind, Picture};
pub use answer::{Decision, GovernanceAnswer};
pub use answer::{DoctorAbsence, DoctorAnswer, DoctorFault, DoctorInstall, DoctorItem};
pub use answer::{DoctorNeed, DoctorState, DoctorTier, DoctorVerdict, DoctorVersion};
pub use answer::{DocumentAnswer, Entry, EntryKind, ListingAnswer};
pub use answer::{HunksAnswer, PatchLine, Withheld};
pub use answer::{InboxAnswer, MetricsAnswer, RegistryAnswer, RegistryLine, SignalLine};
pub use answer::{McpHealthAnswer, McpServerHealth, McpState, McpToolLine};
pub use answer::{PrefixSlot, PrefixSource, SkillLine, SkillShelf, SkillsAnswer};
#[cfg(feature = "server")]
pub use assets::{AssetReply, ClientAssets, EmbeddedFile};
pub use auth::{PairingToken, verify};
pub use carried_name::{ModeTag, ProviderName, TemplateName, UploadId};
pub use command::COMMAND_NAMES;
pub use command::{BodyOverride, EndpointTuning, HeaderPair};
pub use command::{Command, WireCommand};
pub use command::{GovernedDocument, HaltScope, LoginStep, NoSecret, PursuitStep};
pub use control::{ControlVerdict, Intervention, classify};
pub use kernel::{FileChange, How, Lines};
pub use kernel::{Span, Token, markdown};
pub use reading::{OUTPUT_LINES, note_of, output_in, said_in, subject_of, text, used_in};
#[cfg(feature = "server")]
pub use reception::{BindFace, BindVerdict, HandshakeVerdict};
#[cfg(feature = "server")]
pub use reception::{SessionState, SessionStep, decide_frame};
#[cfg(feature = "server")]
pub use reception::{decide_bind, decide_handshake};
#[cfg(feature = "server")]
pub use server::{AcpBody, AcpProgress, AcpSink, TranscribeSink};
#[cfg(feature = "server")]
pub use server::{Delivered, Reply, ServeConfig, router, serve};
#[cfg(feature = "schema")]
pub use wire::wire_schema;
pub use wire::{ClientFrame, Delta, ServerFrame};
pub use wire::{Hello, Query, Welcome};
pub use wire::{LogLevel, LogLine};
pub use wire::{QUERY_NAMES, WIRE_V, schema_hash};

pub use kernel::{Address, ApprovalId, Autonomy, AxCode, AxError, B3Hash};
pub use kernel::{ApprovalClass, ApprovalItem, ApprovalSource, ClusterKey, Restoration};
pub use kernel::{BudgetUse, Locator, PlannedProgress, Progress, UnplannedProgress};
pub use kernel::{DialectKind, Effort, ModelTag};
pub use kernel::{EventDraft, EventKind, EventRecord, GitOid, IdemKey, RunId};
pub use kernel::{McpServer, McpTransport, SandboxLimits, ServerLabel};
pub use kernel::{NodeId, PursuitState, RoadmapStatus, WHOLE_PPB};
pub use kernel::{Payload, Sealed, Seq, TimeMs, Tokens, UsdMicros};
pub use kernel::{PolicyVerdict, SessionName, WriteDomain};
