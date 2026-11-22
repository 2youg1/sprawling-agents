// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Schema } from "effect";

export const AccountDisposition = Schema.Literals(["keep", "advance"]).annotate({ identifier: "AccountDisposition" });
export type AccountDisposition = typeof AccountDisposition.Type;

export const AccountRetries = Schema.Literals(["one", "two"]).annotate({ identifier: "AccountRetries" });
export type AccountRetries = typeof AccountRetries.Type;

export const KeyState = Schema.Literals(["anonymous", "stored", "missing", "environment", "environment_unusable", "unread"]).annotate({ identifier: "KeyState" });
export type KeyState = typeof KeyState.Type;

export const ServerLabel = Schema.String.check(Schema.isPattern(new RegExp("^[a-z0-9]+$", "u"))).pipe(Schema.brand("ServerLabel"));
export type ServerLabel = typeof ServerLabel.Type;

export const AccountStatus = Schema.Struct({
  id: ServerLabel,
  key: KeyState,
}).annotate({ identifier: "AccountStatus" });
export type AccountStatus = typeof AccountStatus.Type;

export const Address = Schema.String.check(Schema.isPattern(new RegExp("^(?:[^/\\\\:\\p{Cc}]*[^/\\\\:\\p{Cc}.\\p{White_Space}])(?:/[^/\\\\:\\p{Cc}]*[^/\\\\:\\p{Cc}.\\p{White_Space}])*$", "u"))).pipe(Schema.brand("Address"));
export type Address = typeof Address.Type;

export const AdmissionRequirement = Schema.Literals(["standing", "tested", "contract_kept", "double_validated"]).annotate({ identifier: "AdmissionRequirement" });
export type AdmissionRequirement = typeof AdmissionRequirement.Type;

export const AgentSource = Schema.Literals(["registry", "detected", "pasted"]).annotate({ identifier: "AgentSource" });
export type AgentSource = typeof AgentSource.Type;

export const B3Hash = Schema.String.check(Schema.isPattern(new RegExp("^[0-9a-f]{64}$", "u"))).pipe(Schema.brand("B3Hash"));
export type B3Hash = typeof B3Hash.Type;

export const IdemKey = Schema.String.check(Schema.isPattern(new RegExp("^idem1-[0-9a-f]{32}$", "u"))).pipe(Schema.brand("IdemKey"));
export type IdemKey = typeof IdemKey.Type;

export const AgentAdding = Schema.Struct({
  idem: IdemKey,
  seat_here: Schema.optional(Schema.NullOr(Address)),
  source: AgentSource,
  spec_digest: B3Hash,
}).annotate({ identifier: "AgentAdding" });
export type AgentAdding = typeof AgentAdding.Type;

export const LoginKind = Schema.Literals(["agent", "terminal"]).annotate({ identifier: "LoginKind" });
export type LoginKind = typeof LoginKind.Type;

export const AuthMethod = Schema.Struct({
  id: Schema.String,
  kind: LoginKind,
  name: Schema.String,
}).annotate({ identifier: "AuthMethod" });
export type AuthMethod = typeof AuthMethod.Type;

export const LoginState = Schema.Literals(["unasked", "ready", "required"]).annotate({ identifier: "LoginState" });
export type LoginState = typeof LoginState.Type;

export const PinState = Schema.Literals(["exact", "floating", "unknown"]).annotate({ identifier: "PinState" });
export type PinState = typeof PinState.Type;

export const AgentLine = Schema.Struct({
  auth_methods: Schema.Array(AuthMethod),
  id: Schema.String,
  login_state: LoginState,
  name: Schema.String,
  pinned: PinState,
  seated_in: Schema.Array(Address),
  source: AgentSource,
  version: Schema.optional(Schema.NullOr(Schema.String)),
}).annotate({ identifier: "AgentLine" });
export type AgentLine = typeof AgentLine.Type;

export const AgentOffer = Schema.Struct({
  env_names: Schema.Array(Schema.String),
  id: Schema.String,
  launch_preview: Schema.String,
  licence: Schema.optional(Schema.NullOr(Schema.String)),
  login: Schema.Array(LoginKind),
  name: Schema.String,
  pinned: PinState,
  source: AgentSource,
  spec_digest: B3Hash,
  version: Schema.optional(Schema.NullOr(Schema.String)),
}).annotate({ identifier: "AgentOffer" });
export type AgentOffer = typeof AgentOffer.Type;

export const CatalogSnapshot = Schema.Struct({
  date: Schema.String,
  etag: Schema.String,
}).annotate({ identifier: "CatalogSnapshot" });
export type CatalogSnapshot = typeof CatalogSnapshot.Type;

export const AgentCatalogAnswer = Schema.Struct({
  added: Schema.Array(AgentLine),
  catalog: Schema.Array(AgentOffer),
  detected: Schema.Array(AgentOffer),
  snapshot: CatalogSnapshot,
}).annotate({ identifier: "AgentCatalogAnswer" });
export type AgentCatalogAnswer = typeof AgentCatalogAnswer.Type;

export const AgentLoginStart = Schema.Struct({
  agent: Schema.String,
  idem: IdemKey,
  method: Schema.String,
}).annotate({ identifier: "AgentLoginStart" });
export type AgentLoginStart = typeof AgentLoginStart.Type;

export const Align = Schema.Literals(["none", "left", "center", "right"]).annotate({ identifier: "Align" });
export type Align = typeof Align.Type;

export const ApprovalId = Schema.String.pipe(Schema.brand("ApprovalId"));
export type ApprovalId = typeof ApprovalId.Type;

export const ApprovalClass = Schema.Literal("question").annotate({ identifier: "ApprovalClass" });
export type ApprovalClass = typeof ApprovalClass.Type;

export const ClusterKey = Schema.Struct({
  class: ApprovalClass,
  detail: Schema.String,
}).annotate({ identifier: "ClusterKey" });
export type ClusterKey = typeof ClusterKey.Type;

export const Locator = Schema.String.pipe(Schema.brand("Locator"));
export type Locator = typeof Locator.Type;

export const TimeMs = Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)).pipe(Schema.brand("TimeMs"));
export type TimeMs = typeof TimeMs.Type;

export const ApprovalItem = Schema.Struct({
  action_desc: Schema.String,
  actor: Schema.String,
  artifact: Locator,
  cluster_key: ClusterKey,
  created: TimeMs,
  id: ApprovalId,
  tainted: Schema.Boolean,
}).annotate({ identifier: "ApprovalItem" });
export type ApprovalItem = typeof ApprovalItem.Type;

export const ApprovalsAnswer = Schema.Struct({
  items: Schema.Array(ApprovalItem),
}).annotate({ identifier: "ApprovalsAnswer" });
export type ApprovalsAnswer = typeof ApprovalsAnswer.Type;

export const ArchiveHit = Schema.Struct({
  building: Address,
  day: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  kind: Schema.String,
  subject: Schema.String,
}).annotate({ identifier: "ArchiveHit" });
export type ArchiveHit = typeof ArchiveHit.Type;

export const ArchiveAnswer = Schema.Struct({
  hits: Schema.Array(ArchiveHit),
  needle: Schema.String,
}).annotate({ identifier: "ArchiveAnswer" });
export type ArchiveAnswer = typeof ArchiveAnswer.Type;

export const Cadence = Schema.Union([
  Schema.Struct({
    every_minutes: Schema.Struct({
      minutes: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
    }),
  }),
  Schema.Struct({
    daily_at: Schema.Struct({
      minute: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
    }),
  }),
  Schema.Struct({
    weekly_at: Schema.Struct({
      minute: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
    }),
  }),
]).annotate({ identifier: "Cadence" });
export type Cadence = typeof Cadence.Type;

export const ScheduledJob = Schema.Struct({
  addr: Address,
  cadence: Cadence,
  goal: Schema.String,
  name: Schema.String,
  task: Schema.String,
}).annotate({ identifier: "ScheduledJob" });
export type ScheduledJob = typeof ScheduledJob.Type;

export const WatchedSource = Schema.Struct({
  addr: Address,
  matches: Schema.String,
  name: Schema.String,
  starts_work: Schema.Boolean,
}).annotate({ identifier: "WatchedSource" });
export type WatchedSource = typeof WatchedSource.Type;

export const AutomationAnswer = Schema.Struct({
  jobs: Schema.Array(ScheduledJob),
  sources: Schema.Array(WatchedSource),
  unreadable: Schema.Array(Schema.String),
}).annotate({ identifier: "AutomationAnswer" });
export type AutomationAnswer = typeof AutomationAnswer.Type;

export const ArchiveLine = Schema.Struct({
  day: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  kind: Schema.String,
  subject: Schema.String,
}).annotate({ identifier: "ArchiveLine" });
export type ArchiveLine = typeof ArchiveLine.Type;

export const NodeId = Schema.String.pipe(Schema.brand("NodeId"));
export type NodeId = typeof NodeId.Type;

export const BlockedLine = Schema.Struct({
  line: Schema.String,
  source: NodeId,
  waiting: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
}).annotate({ identifier: "BlockedLine" });
export type BlockedLine = typeof BlockedLine.Type;

export const BuildingDoc = Schema.Struct({
  bytes: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  name: Schema.String,
  text: Schema.String,
  truncated: Schema.Boolean,
}).annotate({ identifier: "BuildingDoc" });
export type BuildingDoc = typeof BuildingDoc.Type;

export const McpTransport = Schema.Union([
  Schema.Struct({
    stdio: Schema.Struct({
      args: Schema.Array(Schema.String),
      command: Schema.String,
      env: Schema.Array(Schema.Tuple([Schema.String, Schema.String])),
    }),
  }),
  Schema.Struct({
    http: Schema.Struct({
      headers: Schema.Array(Schema.Tuple([Schema.String, Schema.String])),
      url: Schema.String,
    }),
  }),
  Schema.Struct({
    sse: Schema.Struct({
      headers: Schema.Array(Schema.Tuple([Schema.String, Schema.String])),
      url: Schema.String,
    }),
  }),
]).annotate({ identifier: "McpTransport" });
export type McpTransport = typeof McpTransport.Type;

export const McpServer = Schema.Struct({
  label: ServerLabel,
  transport: McpTransport,
}).annotate({ identifier: "McpServer" });
export type McpServer = typeof McpServer.Type;

export const RoadmapStatus = Schema.Literals(["not_started", "in_progress", "done", "blocked", "awaiting_approval"]).annotate({ identifier: "RoadmapStatus" });
export type RoadmapStatus = typeof RoadmapStatus.Type;

export const PlanRow = Schema.Struct({
  evidence: Schema.optional(Schema.NullOr(Schema.String)),
  item: Schema.String,
  leaf: Schema.Boolean,
  needs: Schema.Array(NodeId),
  node: NodeId,
  ready: Schema.Boolean,
  share_ppb: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  status: RoadmapStatus,
}).annotate({ identifier: "PlanRow" });
export type PlanRow = typeof PlanRow.Type;

export const PlannedProgress = Schema.Struct({
  blocked: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  blocked_ppb: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  done: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  done_ppb: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  total: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
}).annotate({ identifier: "PlannedProgress" });
export type PlannedProgress = typeof PlannedProgress.Type;

export const Tokens = Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)).pipe(Schema.brand("Tokens"));
export type Tokens = typeof Tokens.Type;

export const UsdMicros = Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)).pipe(Schema.brand("UsdMicros"));
export type UsdMicros = typeof UsdMicros.Type;

export const BudgetUse = Schema.Struct({
  tokens: Tokens,
  usd: UsdMicros,
}).annotate({ identifier: "BudgetUse" });
export type BudgetUse = typeof BudgetUse.Type;

export const UnplannedProgress = Schema.Struct({
  budget: BudgetUse,
  steps: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
}).annotate({ identifier: "UnplannedProgress" });
export type UnplannedProgress = typeof UnplannedProgress.Type;

export const Progress = Schema.Union([
  Schema.Struct({
    planned: PlannedProgress,
  }),
  Schema.Struct({
    unplanned: UnplannedProgress,
  }),
]).annotate({ identifier: "Progress" });
export type Progress = typeof Progress.Type;

export const ContainerImage = Schema.String.check(Schema.isPattern(new RegExp("^sha256:[0123456789abcdef]{64}(?![\\s\\S])", "u"))).pipe(Schema.brand("ContainerImage"));
export type ContainerImage = typeof ContainerImage.Type;

export const ContainerLimits = Schema.Struct({
  cpu_millis: Schema.Int.check(Schema.isGreaterThanOrEqualTo(1)),
  image: ContainerImage,
  memory_bytes: Schema.Int.check(Schema.isGreaterThanOrEqualTo(1)),
  pids: Schema.Int.check(Schema.isGreaterThanOrEqualTo(1)),
  user: Schema.Int.check(Schema.isGreaterThanOrEqualTo(1)),
}).annotate({ identifier: "ContainerLimits" });
export type ContainerLimits = typeof ContainerLimits.Type;

export const EnvVarName = Schema.String.pipe(Schema.brand("EnvVarName"));
export type EnvVarName = typeof EnvVarName.Type;

export const Interpreter = Schema.Literals(["system", "pwsh"]).annotate({ identifier: "Interpreter" });
export type Interpreter = typeof Interpreter.Type;

export const SandboxArm = Schema.Literals(["none", "copied_tree", "native", "container", "python"]).annotate({ identifier: "SandboxArm" });
export type SandboxArm = typeof SandboxArm.Type;

export const SandboxLimits = Schema.Struct({
  arm: Schema.optional(Schema.NullOr(SandboxArm)),
  container: Schema.optional(Schema.NullOr(ContainerLimits)),
  env_passthrough: Schema.optional(Schema.Array(EnvVarName)),
  interpreter: Schema.optional(Interpreter),
  shell: Schema.Boolean,
  trusted: Schema.optional(Schema.Array(ServerLabel)),
}).annotate({ identifier: "SandboxLimits" });
export type SandboxLimits = typeof SandboxLimits.Type;

export const BuildingAnswer = Schema.Struct({
  addr: Address,
  archive: Schema.Array(ArchiveLine),
  blocked: Schema.Array(BlockedLine),
  docs: Schema.Array(BuildingDoc),
  mcp: Schema.Array(McpServer),
  plan: Schema.Array(PlanRow),
  problems: Schema.Array(Schema.String),
  progress: Progress,
  rooms: Schema.Array(Schema.String),
  sandbox: Schema.optional(Schema.NullOr(SandboxLimits)),
}).annotate({ identifier: "BuildingAnswer" });
export type BuildingAnswer = typeof BuildingAnswer.Type;

export const Span = Schema.Struct({
  end: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  start: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
}).annotate({ identifier: "Span" });
export type Span = typeof Span.Type;

export const BytesAnswer = Schema.Struct({
  base64: Schema.String,
  size: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  span: Span,
  version: B3Hash,
}).annotate({ identifier: "BytesAnswer" });
export type BytesAnswer = typeof BytesAnswer.Type;

export const AuditVerdict = Schema.Literals(["pass", "warn", "fail", "unreachable"]).annotate({ identifier: "AuditVerdict" });
export type AuditVerdict = typeof AuditVerdict.Type;

export const CatalogChange = Schema.Literals(["added", "changed", "removed"]).annotate({ identifier: "CatalogChange" });
export type CatalogChange = typeof CatalogChange.Type;

export const Exposure = Schema.Literals(["core", "dormant", "off"]).annotate({ identifier: "Exposure" });
export type Exposure = typeof Exposure.Type;

export const ExposureFrom = Schema.Literals(["fixed", "default", "city", "building"]).annotate({ identifier: "ExposureFrom" });
export type ExposureFrom = typeof ExposureFrom.Type;

export const CatalogSkill = Schema.Struct({
  audit: Schema.optional(Schema.NullOr(AuditVerdict)),
  audit_stale: Schema.Boolean,
  candidate: Schema.optional(Schema.NullOr(B3Hash)),
  change: Schema.optional(Schema.NullOr(CatalogChange)),
  current_document: Schema.optional(Schema.NullOr(Schema.String)),
  disclosure: Schema.String,
  exposure: Exposure,
  from: ExposureFrom,
  last_used: Schema.optional(Schema.NullOr(TimeMs)),
  name: Schema.String,
  pinned: Schema.optional(Schema.NullOr(B3Hash)),
  pinned_document: Schema.optional(Schema.NullOr(Schema.String)),
  runs_used: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  section: Schema.String,
}).annotate({ identifier: "CatalogSkill" });
export type CatalogSkill = typeof CatalogSkill.Type;

export const Payload = Schema.Record(Schema.String, Schema.Unknown).annotate({ identifier: "Payload" });
export type Payload = typeof Payload.Type;

