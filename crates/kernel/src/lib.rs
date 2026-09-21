// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Pure decision functions: values in, verdicts out. Zero internal deps.
//! Holds no I/O handles, no clock, no global state (ARCHITECTURE.md paragraph 1).
//!
//! **This file is an index, and the modules are the library.** Every
//! module is public and a caller reaches a decision through the module
//! that owns it — `gate::domain`, `discard::decide`, `approval::may_answer`
//! — so the module name says which authority answered. What the root
//! re-exports is the vocabulary those signatures are written in: the
//! value types a caller names in its own fields and arguments. A
//! function is never re-exported here, and nothing is renamed on the
//! way out, because a second spelling of one name is the defect
//! `xtask lexicon` exists to catch.

pub mod address;
pub mod approval;
pub mod backpressure;
pub mod blockage;
pub mod budget;
pub mod change;
pub mod completion;
pub mod config;
pub mod consts_external;
pub mod consts_policy;
pub mod delegation;
pub mod discard;
pub mod error;
pub mod event;
pub mod gate;
pub mod goal;
pub mod highlight;
pub mod idem;
pub mod layout;
pub mod ledger;
pub mod locator;
pub mod model;
pub mod node_id;
pub mod plan;
pub mod pursuit;
pub mod reach;
pub mod registry;
pub mod release;
pub mod repair;
#[cfg(feature = "schema")]
pub mod schema;
pub mod secret;
pub mod share;
pub mod spine;
pub mod stall;
pub mod taint;
pub mod tool;
pub mod version;
pub mod write_domain;

pub use address::{Address, RESERVED_PREFIX, SessionName};
pub use approval::{AnswerVerdict, Answerer, ApprovalClass, ApprovalId, ApprovalItem};
pub use approval::{Autonomy, ClusterKey, Ruling};
pub use backpressure::{Admission, ItemMeta, QueueStats, ShedReason};
pub use blockage::{Blockage, Notice, RedNode};
pub use budget::{BudgetUse, ByteLen, Tokens, UsdMicros};
pub use change::{FileChange, How, Lines};
pub use completion::{Completion, Evidence, PlannedProgress, Progress, UnplannedProgress};
pub use config::{ClockStampGranularity, ClockZone, FrozenConfig, McpServer, McpTransport};
pub use config::{EnvVarName, LayeredValue, LiveConfig, SandboxLimits};
pub use delegation::{Delegate, DelegateKind, DelegationVerdict, Delegator, Depth};
pub use discard::Restoration;
pub use discard::{DenyReason, Discard, DiscardForecast, DiscardRequest, DiscardVerdict};
pub use error::{AxCode, AxError, Carrier, ErrorDraft, GateRefusal};
pub use event::{EventDraft, EventKind, EventRecord, EventRef, Payload};
pub use event::{RunId, Seq, TimeMs, WindowClass};
pub use gate::GateOutcome;
pub use gate::{ConnectorCall, DOORS, DoorId, EgressAllowlist, EgressOutcome, EgressTarget};
pub use goal::{GoalEntry, GoalId, GoalResource, GoalVerdict};
pub use highlight::{Span, Token};
pub use idem::{Duplicate, IDEM_DERIVE_V, IdemGuard, IdemKey};
pub use ledger::{GENESIS_PREV, Ledger};
pub use locator::{B3Hash, GitOid, Locator, Range};
pub use model::{BuildingPolicy, Ceiling, ChatMessage, ChatRequest, ChatResponse, ContentBlock};
pub use model::{DialectKind, Effort, ImageRef, ImageType, Increment, Increments, Model};
pub use model::{ModelRequest, ModelReturn, ModelTag, ModelUsage, Role, StopReason};
pub use model::{SystemBlock, ToolDef};
pub use node_id::{NODE_DEPTH_MAX, NodeId};
pub use plan::{Held, PLAN_WHOLE_PPB, PlanExit, PlanNode, PlanTree, StopCause};
pub use pursuit::{Pursuit, PursuitState, PursuitVerdict};
pub use reach::{Answered, Connected, Named, Proxying, Reach, Through};
pub use registry::{Artifact, Claim, RegisterVerdict, Registry, ResidentId};
pub use release::{Release, ReleaseVerdict};
pub use repair::RepairVerdict;
pub use secret::{Sealed, SecretRef, SecretSpan};
pub use share::{Share, WHOLE_PPB};
pub use spine::{EvidenceCell, MEMO_OUTLINE_FIELDS, MemoShape, NewChild, ROADMAP_COLUMNS};
pub use spine::{ROADMAP_FILE, ROADMAP_STATUS_SPELLINGS, RoadmapRow, RoadmapShape};
pub use spine::{RoadmapStatus, ScopeChange, WriteMoment};
pub use stall::{ActionFingerprint, StallVerdict};
pub use taint::{TaintSet, TaintSource, Tainted};
pub use tool::{CostTier, Effect, ExecArm, RenderIntent, ServerLabel, Temporal, TimeoutMs};
pub use tool::{Tool, ToolCall, ToolMeta, ToolName, ToolOutcome};
pub use version::{Version, VersionVerdict};
pub use write_domain::{DocumentReason, DomainPrefixes, DomainVerdict, EditSample};
pub use write_domain::{EditWarVerdict, WriteDomain};
