// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Schema } from "effect";

export const WIRE_V = 79 as const;

export const WIRE_HASH = "edccfc623a2c07088052f273da697dfebc14c671240d737ed9992d6060c19902" as const;

export const CITY_RUN = "00000000-0000-0000-0000-000000000000" as const;

export const BODY_PX = { min: 12 } as const;

export const LINE_HEIGHT = { min: 1200 } as const;

export const TRACKING = { min: -50 } as const;

export const DEFAULT_EFFORT = "high" as const;

export const SLASH = [
  { spelling: "/help", takes: "", offered: "both", says: "list these verbs" },
  { spelling: "/room", takes: "<addr>", offered: "both", says: "choose the room plain lines go to" },
  { spelling: "/new", takes: "", offered: "both", says: "start a new session in this room" },
  { spelling: "/stop", takes: "", offered: "both", says: "stop the run working in this room" },
  { spelling: "/halt", takes: "[--all]", offered: "both", says: "hold this room's building, or with --all the whole city" },
  { spelling: "/release", takes: "[--all]", offered: "both", says: "let this room's building work again, or with --all the whole city" },
  { spelling: "/model", takes: "[<id>]", offered: "both", says: "show the model, or choose one the city's endpoints offer" },
  { spelling: "/effort", takes: "[<level>]", offered: "both", says: "show the thinking level, or choose one the model offers" },
  { spelling: "/approve", takes: "[<id>]", offered: "cli", says: "approve the waiting request, or the one with this id" },
  { spelling: "/deny", takes: "[<id>]", offered: "cli", says: "deny the waiting request, or the one with this id" },
  { spelling: "/web", takes: "", offered: "cli", says: "open the WebUI and quiet this terminal; Esc comes back" },
  { spelling: "/quit", takes: "", offered: "both", says: "close the city" },
  { spelling: "/serving", takes: "", offered: "cli", says: "where this city listens, and what is running in it" },
  { spelling: "/remote", takes: "<verb>", offered: "cli", says: "open|pair|close|devices|revoke: the door a device reaches the city through" },
  { spelling: "/acp", takes: "[<text>]", offered: "both", says: "find an ACP agent, or read a pasted launch line" },
  { spelling: "/wire", takes: "<verb> [<json>]", offered: "cli", says: "send any wire command or question, its body as JSON" },
] as const;

export const AccountDisposition = Schema.Union([
  Schema.Literal("keep"),
  Schema.Literal("advance"),
]).annotate({ identifier: "AccountDisposition" });
export type AccountDisposition = typeof AccountDisposition.Type;

export const AccountRetries = Schema.Literals(["one", "two"]).annotate({ identifier: "AccountRetries" });
export type AccountRetries = typeof AccountRetries.Type;

export const KeyState = Schema.Union([
  Schema.Literal("anonymous"),
  Schema.Literal("stored"),
  Schema.Literal("missing"),
  Schema.Literal("environment"),
  Schema.Literal("environment_unusable"),
  Schema.Literal("unread"),
]).annotate({ identifier: "KeyState" });
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

export const AdmissionRequirement = Schema.Union([
  Schema.Literal("standing"),
  Schema.Literal("tested"),
  Schema.Literal("contract_kept"),
  Schema.Literal("double_validated"),
]).annotate({ identifier: "AdmissionRequirement" });
export type AdmissionRequirement = typeof AdmissionRequirement.Type;

export const AgentSource = Schema.Union([
  Schema.Literal("registry"),
  Schema.Literal("detected"),
  Schema.Literal("pasted"),
]).annotate({ identifier: "AgentSource" });
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

export const LoginKind = Schema.Union([
  Schema.Literal("agent"),
  Schema.Literal("terminal"),
]).annotate({ identifier: "LoginKind" });
export type LoginKind = typeof LoginKind.Type;

export const AuthMethod = Schema.Struct({
  id: Schema.String,
  kind: LoginKind,
  name: Schema.String,
}).annotate({ identifier: "AuthMethod" });
export type AuthMethod = typeof AuthMethod.Type;

export const LoginState = Schema.Union([
  Schema.Literal("unasked"),
  Schema.Literal("ready"),
  Schema.Literal("required"),
]).annotate({ identifier: "LoginState" });
export type LoginState = typeof LoginState.Type;

export const PinState = Schema.Union([
  Schema.Literal("exact"),
  Schema.Literal("floating"),
  Schema.Literal("unknown"),
]).annotate({ identifier: "PinState" });
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
  fuel: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  interpreter: Schema.optional(Interpreter),
  mounts: Schema.Array(Address),
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

export const AuditVerdict = Schema.Union([
  Schema.Literals(["pass", "warn", "fail"]),
  Schema.Literal("unreachable"),
]).annotate({ identifier: "AuditVerdict" });
export type AuditVerdict = typeof AuditVerdict.Type;

export const CatalogChange = Schema.Literals(["added", "changed", "removed"]).annotate({ identifier: "CatalogChange" });
export type CatalogChange = typeof CatalogChange.Type;

export const Exposure = Schema.Literals(["core", "dormant", "off"]).annotate({ identifier: "Exposure" });
export type Exposure = typeof Exposure.Type;

export const ExposureFrom = Schema.Union([
  Schema.Literal("fixed"),
  Schema.Literal("default"),
  Schema.Literal("city"),
  Schema.Literal("building"),
]).annotate({ identifier: "ExposureFrom" });
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

export const CatalogVersion = Schema.Struct({
  bytes: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  description: Schema.String,
  digest: B3Hash,
  hint: Schema.String,
  input_schema: Payload,
}).annotate({ identifier: "CatalogVersion" });
export type CatalogVersion = typeof CatalogVersion.Type;

export const CatalogTool = Schema.Struct({
  change: Schema.optional(Schema.NullOr(CatalogChange)),
  current: Schema.optional(Schema.NullOr(CatalogVersion)),
  exposure: Exposure,
  fallback: Exposure,
  from: ExposureFrom,
  last_used: Schema.optional(Schema.NullOr(TimeMs)),
  name: Schema.String,
  namespace: Schema.String,
  pinned: Schema.optional(Schema.NullOr(CatalogVersion)),
  runs_used: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
}).annotate({ identifier: "CatalogTool" });
export type CatalogTool = typeof CatalogTool.Type;

export const ExposureTable = Schema.Struct({
  namespaces: Schema.optional(Schema.Record(Schema.String, Exposure)),
  skills: Schema.optional(Schema.Record(Schema.String, Exposure)),
  tools: Schema.optional(Schema.Record(Schema.String, Exposure)),
}).annotate({ identifier: "ExposureTable" });
export type ExposureTable = typeof ExposureTable.Type;

export const McpReference = Schema.Struct({
  definition: Schema.optional(Schema.NullOr(Schema.String)),
  label: ServerLabel,
  tools: Schema.optional(Schema.NullOr(Schema.Record(Schema.String, B3Hash))),
}).annotate({ identifier: "McpReference" });
export type McpReference = typeof McpReference.Type;

export const RunId = Schema.String.check(Schema.isPattern(new RegExp("^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$", "u"))).pipe(Schema.brand("RunId"));
export type RunId = typeof RunId.Type;

export const CatalogAnswer = Schema.Struct({
  always: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  building: Address,
  changed: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  city: ExposureTable,
  core_bytes: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  dormant: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  index_bytes: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  open_sessions: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  reading_bytes: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  reading_room: Schema.Array(Schema.String),
  references: Schema.Array(McpReference),
  run: Schema.optional(Schema.NullOr(RunId)),
  runs_seen: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  skill_pins: Schema.Record(Schema.String, B3Hash),
  skill_rows: Schema.Array(CatalogSkill),
  skills: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  stated: ExposureTable,
  tools: Schema.Array(CatalogTool),
}).annotate({ identifier: "CatalogAnswer" });
export type CatalogAnswer = typeof CatalogAnswer.Type;

export const How = Schema.Union([
  Schema.Literals(["added", "modified", "deleted"]),
  Schema.Struct({
    renamed: Schema.Struct({
      from: Schema.String,
    }),
  }),
]).annotate({ identifier: "How" });
export type How = typeof How.Type;

export const Lines = Schema.Union([
  Schema.Literal("binary"),
  Schema.Struct({
    counted: Schema.Struct({
      added: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
      removed: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
    }),
  }),
]).annotate({ identifier: "Lines" });
export type Lines = typeof Lines.Type;

export const FileChange = Schema.Struct({
  how: How,
  lines: Lines,
  path: Schema.String,
}).annotate({ identifier: "FileChange" });
export type FileChange = typeof FileChange.Type;

export const GitOid = Schema.String.check(Schema.isPattern(new RegExp("^[0-9a-f]{40}$", "u"))).pipe(Schema.brand("GitOid"));
export type GitOid = typeof GitOid.Type;

export const ChangesAnswer = Schema.Struct({
  base: GitOid,
  files: Schema.Array(FileChange),
  head: Schema.optional(Schema.NullOr(GitOid)),
}).annotate({ identifier: "ChangesAnswer" });
export type ChangesAnswer = typeof ChangesAnswer.Type;

export const BuildingProgress = Schema.Struct({
  addr: Address,
  adoptable: Schema.Boolean,
  blocked: Schema.Array(BlockedLine),
  problems: Schema.Array(Schema.String),
  progress: Progress,
  ready: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
}).annotate({ identifier: "BuildingProgress" });
export type BuildingProgress = typeof BuildingProgress.Type;

export const HaltScope = Schema.Union([
  Schema.Literal("city"),
  Schema.Struct({
    building: Address,
  }),
  Schema.Struct({
    workshop: Address,
  }),
]).annotate({ identifier: "HaltScope" });
export type HaltScope = typeof HaltScope.Type;

export const PursuitState = Schema.Literals(["running", "paused"]).annotate({ identifier: "PursuitState" });
export type PursuitState = typeof PursuitState.Type;

export const PursuitVerdict = Schema.Union([
  Schema.Struct({
    kind: Schema.Literal("work"),
    next: NodeId,
  }),
  Schema.Struct({
    in_flight: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
    kind: Schema.Literal("waiting"),
  }),
  Schema.Struct({
    kind: Schema.Literal("paused"),
  }),
  Schema.Struct({
    kind: Schema.Literal("finished"),
  }),
]).annotate({ identifier: "PursuitVerdict" });
export type PursuitVerdict = typeof PursuitVerdict.Type;

export const PursuitLine = Schema.Struct({
  addr: Address,
  goal: Schema.String,
  state: PursuitState,
  verdict: PursuitVerdict,
}).annotate({ identifier: "PursuitLine" });
export type PursuitLine = typeof PursuitLine.Type;

export const EventKind = Schema.Union([
  Schema.Literals(["city_initialized", "building_created", "run_started", "run_forked", "prompt_assembled", "model_called", "model_returned", "tool_called", "tool_result", "result_offloaded", "gate_checked", "gate_denied", "checkpoint_committed", "handoff_written", "steer_received", "cancel_received", "watchdog_fired", "budget_limit", "run_frozen", "log_truncated", "signal_enqueued", "signal_consumed", "draft_held", "draft_resolved", "goal_registered", "goal_conflict", "arbitration_verdict", "repair_started", "repair_reused", "worktree_opened", "pr_opened", "pr_merged", "pr_rejected", "roadmap_claimed", "roadmap_finished", "roadmap_released", "approval_requested", "approval_resolved", "policy_created", "policy_revoked", "taint_promoted", "cross_building_transfer", "city_halted", "backpressure_shed", "digest_invalidated", "endpoint_attached", "endpoint_lost", "model_selected", "provider_degraded", "eval_run", "asset_archived", "credential_lent", "secret_captured", "secret_egress_blocked", "file_discarded", "discard_restored", "autonomy_changed", "went_back", "file_restored"]),
  Schema.Literal("building_configured"),
  Schema.Literal("building_removed"),
  Schema.Literal("session_opened"),
  Schema.Literal("prompt_shape_compared"),
  Schema.Literal("roadmap_split"),
  Schema.Literal("roadmap_blocked"),
  Schema.Literal("pursuit_changed"),
  Schema.Literal("endpoint_probed"),
  Schema.Literal("login_started"),
  Schema.Literal("governed_document_written"),
  Schema.Literal("spine_document_written"),
  Schema.Literal("rules_changed"),
  Schema.Literal("toolkit_link_opened"),
  Schema.Literal("embedding_called"),
  Schema.Literal("rerank_called"),
  Schema.Literal("adviser_asked"),
  Schema.Literal("adviser_answered"),
  Schema.Literal("adviser_fell_back"),
  Schema.Literal("cache_renewed"),
  Schema.Literal("harness_reported"),
  Schema.Literal("harness_answered"),
  Schema.Literal("remote_opened"),
  Schema.Literal("remote_closed"),
  Schema.Literal("device_paired"),
  Schema.Literal("device_revoked"),
  Schema.Literal("remote_session_started"),
  Schema.Literal("document_written"),
  Schema.Literal("proposal_offered"),
  Schema.Literal("proposal_decided"),
  Schema.Literal("proposal_withdrawn"),
  Schema.Literal("run_policy_changed"),
  Schema.Literal("session_named"),
  Schema.Literal("skill_audited"),
  Schema.Literal("signal_wait_started"),
  Schema.Literal("signal_wait_ended"),
  Schema.Literal("signal_landed"),
  Schema.Literal("skill_shelved"),
  Schema.Literal("prefix_held"),
]).annotate({ identifier: "EventKind" });
export type EventKind = typeof EventKind.Type;

export const Seq = Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)).pipe(Schema.brand("Seq"));
export type Seq = typeof Seq.Type;

export const Waiting = Schema.Struct({
  on: Address,
  until: TimeMs,
}).annotate({ identifier: "Waiting" });
export type Waiting = typeof Waiting.Type;

export const RunSummary = Schema.Struct({
  addr: Schema.optional(Schema.NullOr(Address)),
  ask: Schema.optional(Schema.NullOr(Schema.String)),
  completion: Schema.optional(Schema.NullOr(Schema.String)),
  frozen: Schema.Boolean,
  goal: Schema.optional(Schema.NullOr(Schema.String)),
  last_kind: EventKind,
  last_seq: Seq,
  pr: Schema.optional(Schema.NullOr(Schema.String)),
  run: RunId,
  started: Schema.optional(Schema.NullOr(TimeMs)),
  task: Schema.optional(Schema.NullOr(Schema.String)),
  waiting: Schema.optional(Schema.NullOr(Waiting)),
  who: Schema.String,
}).annotate({ identifier: "RunSummary" });
export type RunSummary = typeof RunSummary.Type;

export const CityAnswer = Schema.Struct({
  active: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  buildings: Schema.Array(BuildingProgress),
  frozen: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  halted: Schema.Array(HaltScope),
  proved: Schema.optional(Schema.NullOr(Seq)),
  pursuits: Schema.Array(PursuitLine),
  runs: Schema.Array(RunSummary),
}).annotate({ identifier: "CityAnswer" });
export type CityAnswer = typeof CityAnswer.Type;

export const CommitAt = Schema.Struct({
  oid: GitOid,
  seq: Seq,
}).annotate({ identifier: "CommitAt" });
export type CommitAt = typeof CommitAt.Type;

export const Effort = Schema.Literals(["none", "minimal", "low", "medium", "high", "xhigh", "max"]).annotate({ identifier: "Effort" });
export type Effort = typeof Effort.Type;

export const SessionName = Schema.String.check(Schema.isPattern(new RegExp("^\\p{White_Space}*(?:[^\\p{White_Space}/\\\\:\\p{Cc}.]|[^\\p{White_Space}/\\\\:\\p{Cc}][^/\\\\:\\p{Cc}]{0,62}[^/\\\\:\\p{Cc}.\\p{White_Space}])\\p{White_Space}*$", "u"))).pipe(Schema.brand("SessionName"));
export type SessionName = typeof SessionName.Type;

export const CommitAnswer = Schema.Struct({
  actor: Address,
  at: TimeMs,
  b3: Schema.optional(Schema.NullOr(B3Hash)),
  effort: Schema.optional(Schema.NullOr(Effort)),
  lineage: Schema.Array(RunId),
  message: Schema.optional(Schema.NullOr(Schema.String)),
  model: Schema.String,
  oid: GitOid,
  parents: Schema.optional(Schema.NullOr(Schema.Array(GitOid))),
  previous: Schema.optional(Schema.NullOr(CommitAt)),
  run: RunId,
  seq: Seq,
  session: Schema.optional(Schema.NullOr(SessionName)),
  spent: UsdMicros,
}).annotate({ identifier: "CommitAnswer" });
export type CommitAnswer = typeof CommitAnswer.Type;

export const CommitsAnswer = Schema.Struct({
  before: Schema.optional(Schema.NullOr(Seq)),
  building: Schema.optional(Schema.NullOr(Address)),
  commits: Schema.Array(CommitAnswer),
  more: Schema.Boolean,
}).annotate({ identifier: "CommitsAnswer" });
export type CommitsAnswer = typeof CommitsAnswer.Type;

export const ConfigLayer = Schema.Union([
  Schema.Literal("default"),
  Schema.Literal("city"),
  Schema.Literal("building"),
  Schema.Literal("resident"),
]).annotate({ identifier: "ConfigLayer" });
export type ConfigLayer = typeof ConfigLayer.Type;

export const SettledEffort = Schema.Struct({
  effort: Effort,
  from: ConfigLayer,
}).annotate({ identifier: "SettledEffort" });
export type SettledEffort = typeof SettledEffort.Type;

export const ProviderAccount = Schema.Struct({
  header: Schema.optional(Schema.NullOr(Schema.String)),
  id: ServerLabel,
  reference: Schema.optional(Schema.NullOr(Schema.String)),
}).annotate({ identifier: "ProviderAccount" });
export type ProviderAccount = typeof ProviderAccount.Type;

export const SearchSupplier = Schema.Struct({
  accounts: Schema.Array(ProviderAccount),
  count_field: Schema.optional(Schema.NullOr(Schema.String)),
  id: ServerLabel,
  objective_field: Schema.optional(Schema.NullOr(Schema.String)),
  query_field: Schema.String,
  remote: Schema.String,
  url: Schema.String,
}).annotate({ identifier: "SearchSupplier" });
export type SearchSupplier = typeof SearchSupplier.Type;

export const SearchConfiguration = Schema.Union([
  Schema.Literal("default"),
  Schema.Struct({
    custom: Schema.Struct({
      selected: ServerLabel,
      suppliers: Schema.Array(SearchSupplier),
    }),
  }),
  Schema.Literal("off"),
]).annotate({ identifier: "SearchConfiguration" });
export type SearchConfiguration = typeof SearchConfiguration.Type;

export const SupplierAccounts = Schema.Struct({
  accounts: Schema.Array(AccountStatus),
  supplier: ServerLabel,
}).annotate({ identifier: "SupplierAccounts" });
export type SupplierAccounts = typeof SupplierAccounts.Type;

export const SettledSearch = Schema.Struct({
  account_status: Schema.Array(SupplierAccounts),
  city: Schema.optional(Schema.NullOr(SearchConfiguration)),
  configuration: SearchConfiguration,
  default_url: Schema.String,
  from: ConfigLayer,
}).annotate({ identifier: "SettledSearch" });
export type SettledSearch = typeof SettledSearch.Type;

export const SecondDomain = Schema.Struct({
  max: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  min: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
}).annotate({ identifier: "SecondDomain" });
export type SecondDomain = typeof SecondDomain.Type;

export const SettledSecond = Schema.Struct({
  domain: SecondDomain,
  from: ConfigLayer,
  percent: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
}).annotate({ identifier: "SettledSecond" });
export type SettledSecond = typeof SettledSecond.Type;

export const Proxying = Schema.Union([
  Schema.Literal("except_local"),
  Schema.Literal("always"),
  Schema.Literal("never"),
]).annotate({ identifier: "Proxying" });
export type Proxying = typeof Proxying.Type;

export const TuningDefaults = Schema.Struct({
  account_retries: AccountRetries,
  from: ConfigLayer,
  proxying: Proxying,
  request_max_retries: Schema.optional(Schema.NullOr(Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)))),
  stream_idle_timeout_ms: Schema.optional(Schema.NullOr(Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)))),
  timeout_ms: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
}).annotate({ identifier: "TuningDefaults" });
export type TuningDefaults = typeof TuningDefaults.Type;

export const ConfigAnswer = Schema.Struct({
  addr: Address,
  context_lock_tokens: Schema.optional(Schema.NullOr(Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)))),
  effort: Schema.optional(Schema.NullOr(SettledEffort)),
  first: Schema.optional(Schema.NullOr(Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)))),
  search: SettledSearch,
  second: SettledSecond,
  tuning: TuningDefaults,
}).annotate({ identifier: "ConfigAnswer" });
export type ConfigAnswer = typeof ConfigAnswer.Type;

export const ContentAnswer = Schema.Struct({
  binary: Schema.Boolean,
  bytes: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  locator: Locator,
  text: Schema.String,
  truncated: Schema.Boolean,
}).annotate({ identifier: "ContentAnswer" });
export type ContentAnswer = typeof ContentAnswer.Type;

export const Renewals = Schema.Struct({
  billed: UsdMicros,
  late: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  read: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  refused: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  unreported: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
}).annotate({ identifier: "Renewals" });
export type Renewals = typeof Renewals.Type;

export const UnpricedCalls = Schema.Struct({
  calls: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  tokens: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
}).annotate({ identifier: "UnpricedCalls" });
export type UnpricedCalls = typeof UnpricedCalls.Type;

export const CostAnswer = Schema.Struct({
  by_actor: Schema.Array(Schema.Tuple([Schema.String, UsdMicros])),
  by_run: Schema.Array(Schema.Tuple([Schema.String, UsdMicros])),
  by_segment: Schema.Array(Schema.Tuple([Schema.String, UsdMicros])),
  by_skill: Schema.Array(Schema.Tuple([Schema.String, UsdMicros])),
  by_tool: Schema.Array(Schema.Tuple([Schema.String, UsdMicros])),
  renewals: Renewals,
  total: UsdMicros,
  unpriced: UnpricedCalls,
}).annotate({ identifier: "CostAnswer" });
export type CostAnswer = typeof CostAnswer.Type;

export const CostOfAnswer = Schema.Struct({
  node: NodeId,
  runs: Schema.Array(Schema.Tuple([RunId, UsdMicros])),
  spent: UsdMicros,
}).annotate({ identifier: "CostOfAnswer" });
export type CostOfAnswer = typeof CostOfAnswer.Type;

export const DeviceId = Schema.String.pipe(Schema.brand("DeviceId"));
export type DeviceId = typeof DeviceId.Type;

export const DeviceLine = Schema.Struct({
  id: DeviceId,
  label: Schema.String,
  last_seen: Schema.optional(Schema.NullOr(TimeMs)),
  paired_at: TimeMs,
}).annotate({ identifier: "DeviceLine" });
export type DeviceLine = typeof DeviceLine.Type;

export const DevicesAnswer = Schema.Struct({
  devices: Schema.Array(DeviceLine),
}).annotate({ identifier: "DevicesAnswer" });
export type DevicesAnswer = typeof DevicesAnswer.Type;

export const Restoration = Schema.Union([
  Schema.Struct({
    tracked: Locator,
  }),
  Schema.Struct({
    interred: Locator,
  }),
  Schema.Struct({
    rebuildable: Schema.Struct({
      reason: Schema.String,
    }),
  }),
]).annotate({ identifier: "Restoration" });
export type Restoration = typeof Restoration.Type;

export const DiscardLine = Schema.Struct({
  at: TimeMs,
  path: Schema.String,
  restoration: Schema.optional(Schema.NullOr(Restoration)),
  restored: Schema.Boolean,
}).annotate({ identifier: "DiscardLine" });
export type DiscardLine = typeof DiscardLine.Type;

export const DiscardAnswer = Schema.Struct({
  rows: Schema.Array(DiscardLine),
}).annotate({ identifier: "DiscardAnswer" });
export type DiscardAnswer = typeof DiscardAnswer.Type;

export const DoctorCore = Schema.Union([
  Schema.Literal("raised"),
  Schema.Literal("held_by_setting"),
  Schema.Struct({
    refused: Schema.Struct({
      said: Schema.String,
    }),
  }),
  Schema.Literal("lowered_by_valve"),
  Schema.Struct({
    unasked: Schema.Struct({
      said: Schema.String,
    }),
  }),
]).annotate({ identifier: "DoctorCore" });
export type DoctorCore = typeof DoctorCore.Type;

export const DoctorCustodyLifetime = Schema.Literals(["across_reboots", "with_passphrase", "until_reboot", "this_process"]).annotate({ identifier: "DoctorCustodyLifetime" });
export type DoctorCustodyLifetime = typeof DoctorCustodyLifetime.Type;

export const DoctorCustodyStore = Schema.Union([
  Schema.Literal("platform_service"),
  Schema.Literal("encrypted_file"),
  Schema.Literal("session_memory"),
]).annotate({ identifier: "DoctorCustodyStore" });
export type DoctorCustodyStore = typeof DoctorCustodyStore.Type;

export const DoctorCustody = Schema.Struct({
  keeps: DoctorCustodyLifetime,
  refusal: Schema.optional(Schema.NullOr(Schema.String)),
  store: DoctorCustodyStore,
}).annotate({ identifier: "DoctorCustody" });
export type DoctorCustody = typeof DoctorCustody.Type;

export const DoctorInstall = Schema.Union([
  Schema.Struct({
    command: Schema.Struct({
      spelled: Schema.String,
    }),
  }),
  Schema.Struct({
    print: Schema.Struct({
      spelled: Schema.String,
    }),
  }),
  Schema.Struct({
    manual: Schema.Struct({
      how: Schema.String,
    }),
  }),
  Schema.Literal("unknown_platform"),
]).annotate({ identifier: "DoctorInstall" });
export type DoctorInstall = typeof DoctorInstall.Type;

export const DoctorNeed = Schema.Union([
  Schema.Literal("required"),
  Schema.Literal("optional"),
]).annotate({ identifier: "DoctorNeed" });
export type DoctorNeed = typeof DoctorNeed.Type;

export const DoctorPack = Schema.Union([
  Schema.Literal("rust_tools"),
]).annotate({ identifier: "DoctorPack" });
export type DoctorPack = typeof DoctorPack.Type;

export const DoctorAbsence = Schema.Union([
  Schema.Literals(["not_on_search_path", "no_home", "not_in_this_build"]),
  Schema.Struct({
    variable_names_nothing: Schema.Struct({
      path: Schema.String,
      variable: Schema.String,
    }),
  }),
  Schema.Struct({
    no_component: Schema.Struct({
      dir: Schema.String,
    }),
  }),
]).annotate({ identifier: "DoctorAbsence" });
export type DoctorAbsence = typeof DoctorAbsence.Type;

export const DoctorFault = Schema.Union([
  Schema.Struct({
    will_not_start: Schema.Struct({
      said: Schema.String,
    }),
  }),
  Schema.Literal("half_written"),
  Schema.Struct({
    unreadable: Schema.Struct({
      said: Schema.String,
    }),
  }),
]).annotate({ identifier: "DoctorFault" });
export type DoctorFault = typeof DoctorFault.Type;

export const DoctorVersion = Schema.Union([
  Schema.Struct({
    said: Schema.Struct({
      text: Schema.String,
    }),
  }),
  Schema.Literal("silent"),
  Schema.Literal("unreadable"),
  Schema.Literal("late"),
]).annotate({ identifier: "DoctorVersion" });
export type DoctorVersion = typeof DoctorVersion.Type;

export const DoctorState = Schema.Union([
  Schema.Struct({
    present: Schema.Struct({
      at: Schema.String,
      version: DoctorVersion,
    }),
  }),
  Schema.Struct({
    broken: Schema.Struct({
      at: Schema.String,
      fault: DoctorFault,
    }),
  }),
  Schema.Struct({
    absent: Schema.Struct({
      absence: DoctorAbsence,
    }),
  }),
]).annotate({ identifier: "DoctorState" });
export type DoctorState = typeof DoctorState.Type;

export const DoctorTier = Schema.Union([
  Schema.Literal("use"),
  Schema.Literal("develop"),
]).annotate({ identifier: "DoctorTier" });
export type DoctorTier = typeof DoctorTier.Type;

export const DoctorItem = Schema.Struct({
  homepage: Schema.optional(Schema.NullOr(Schema.String)),
  install: DoctorInstall,
  name: Schema.String,
  need: DoctorNeed,
  pack: Schema.optional(Schema.NullOr(DoctorPack)),
  pinned: Schema.optional(Schema.NullOr(Schema.String)),
  state: DoctorState,
  tier: DoctorTier,
}).annotate({ identifier: "DoctorItem" });
export type DoctorItem = typeof DoctorItem.Type;

export const DoctorCoverage = Schema.Literals(["kept", "not_kept"]).annotate({ identifier: "DoctorCoverage" });
export type DoctorCoverage = typeof DoctorCoverage.Type;

export const DoctorGuaranteeAxis = Schema.Literals(["filesystem", "network", "process_tree", "user", "resources"]).annotate({ identifier: "DoctorGuaranteeAxis" });
export type DoctorGuaranteeAxis = typeof DoctorGuaranteeAxis.Type;

export const DoctorGuarantee = Schema.Struct({
  axis: DoctorGuaranteeAxis,
  kept: DoctorCoverage,
}).annotate({ identifier: "DoctorGuarantee" });
export type DoctorGuarantee = typeof DoctorGuarantee.Type;

export const DoctorSandboxMissing = Schema.Literal("scratch_directory").annotate({ identifier: "DoctorSandboxMissing" });
export type DoctorSandboxMissing = typeof DoctorSandboxMissing.Type;

export const DoctorSandboxArm = Schema.Union([
  Schema.Literal("linux_namespaces"),
  Schema.Literal("windows_job_object"),
  Schema.Literal("macos_seatbelt"),
  Schema.Literal("copied_tree"),
  Schema.Struct({
    unavailable: Schema.Struct({
      missing: DoctorSandboxMissing,
    }),
  }),
]).annotate({ identifier: "DoctorSandboxArm" });
export type DoctorSandboxArm = typeof DoctorSandboxArm.Type;

export const DoctorSandbox = Schema.Struct({
  arm: DoctorSandboxArm,
  coverage: Schema.Array(DoctorGuarantee),
  named: SandboxArm,
}).annotate({ identifier: "DoctorSandbox" });
export type DoctorSandbox = typeof DoctorSandbox.Type;

export const DoctorUntold = Schema.Union([
  Schema.Literal("no_disk"),
  Schema.Literal("admin_only"),
  Schema.Struct({
    unread: Schema.Struct({
      command: Schema.String,
    }),
  }),
  Schema.Struct({
    failed: Schema.Struct({
      code: Schema.optional(Schema.NullOr(Schema.Int)),
      command: Schema.String,
    }),
  }),
  Schema.Struct({
    unstarted: Schema.Struct({
      command: Schema.String,
    }),
  }),
  Schema.Struct({
    unanswered: Schema.Struct({
      command: Schema.String,
      stopping: Schema.optional(Schema.NullOr(Schema.String)),
    }),
  }),
]).annotate({ identifier: "DoctorUntold" });
export type DoctorUntold = typeof DoctorUntold.Type;

export const DoctorDrive = Schema.Union([
  Schema.Literal("trusted"),
  Schema.Struct({
    untrusted: Schema.Struct({
      volume: Schema.String,
    }),
  }),
  Schema.Struct({
    not: Schema.Struct({
      file_system: Schema.String,
      volume: Schema.String,
    }),
  }),
  Schema.Struct({
    untold: Schema.Struct({
      why: DoctorUntold,
    }),
  }),
]).annotate({ identifier: "DoctorDrive" });
export type DoctorDrive = typeof DoctorDrive.Type;

export const DoctorExclusion = Schema.Union([
  Schema.Literal("outside"),
  Schema.Struct({
    inside: Schema.Struct({
      under: Schema.String,
    }),
  }),
  Schema.Struct({
    untold: Schema.Struct({
      why: DoctorUntold,
    }),
  }),
]).annotate({ identifier: "DoctorExclusion" });
export type DoctorExclusion = typeof DoctorExclusion.Type;

export const DoctorScanning = Schema.Union([
  Schema.Literal("does_not_apply"),
  Schema.Literal("stopped"),
  Schema.Struct({
    read: Schema.Struct({
      city: Schema.String,
      drive: DoctorDrive,
      exclusion: DoctorExclusion,
    }),
  }),
]).annotate({ identifier: "DoctorScanning" });
export type DoctorScanning = typeof DoctorScanning.Type;

export const DoctorVerdict = Schema.Struct({
  missing: Schema.Array(Schema.String),
  tier: DoctorTier,
}).annotate({ identifier: "DoctorVerdict" });
export type DoctorVerdict = typeof DoctorVerdict.Type;

export const DoctorAnswer = Schema.Struct({
  core: DoctorCore,
  custody: DoctorCustody,
  items: Schema.Array(DoctorItem),
  sandbox: DoctorSandbox,
  scanning: DoctorScanning,
  tiers: Schema.Array(DoctorVerdict),
}).annotate({ identifier: "DoctorAnswer" });
export type DoctorAnswer = typeof DoctorAnswer.Type;

export const DoctorUnread = Schema.Union([
  Schema.Literal("with_toolchain"),
  Schema.Literal("many_brands"),
  Schema.Literal("matches_browser"),
  Schema.Literal("this_project"),
  Schema.Literal("no_source"),
  Schema.Literal("unknown_item"),
]).annotate({ identifier: "DoctorUnread" });
export type DoctorUnread = typeof DoctorUnread.Type;

export const DoctorNewest = Schema.Union([
  Schema.Literal("asking"),
  Schema.Struct({
    read: Schema.Struct({
      version: Schema.String,
    }),
  }),
  Schema.Struct({
    unread: Schema.Struct({
      why: DoctorUnread,
    }),
  }),
  Schema.Struct({
    refused: Schema.Struct({
      said: Schema.String,
    }),
  }),
]).annotate({ identifier: "DoctorNewest" });
export type DoctorNewest = typeof DoctorNewest.Type;

export const DoctorUpstream = Schema.Struct({
  item: Schema.String,
  newest: DoctorNewest,
}).annotate({ identifier: "DoctorUpstream" });
export type DoctorUpstream = typeof DoctorUpstream.Type;

export const Format = Schema.Union([
  Schema.Literal("markdown"),
  Schema.Literal("plain"),
]).annotate({ identifier: "Format" });
export type Format = typeof Format.Type;

export const Coverage = Schema.Union([
  Schema.Literal("whole"),
  Schema.Literal("head"),
]).annotate({ identifier: "Coverage" });
export type Coverage = typeof Coverage.Type;

export const Encoding = Schema.Union([
  Schema.Literal("utf8"),
  Schema.Literal("utf8_bom"),
  Schema.Literal("utf16_le"),
  Schema.Literal("utf16_be"),
]).annotate({ identifier: "Encoding" });
export type Encoding = typeof Encoding.Type;

export const Window2 = Schema.Struct({
  span: Span,
  text: Schema.String,
}).annotate({ identifier: "Window2" });
export type Window2 = typeof Window2.Type;

export const DocumentBody = Schema.Union([
  Schema.Struct({
    text: Schema.Struct({
      coverage: Coverage,
      encoding: Encoding,
      head: Window2,
    }),
  }),
  Schema.Literal("opaque"),
]).annotate({ identifier: "DocumentBody" });
export type DocumentBody = typeof DocumentBody.Type;

export const HeldDocument = Schema.Struct({
  body: DocumentBody,
  bytes: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  format: Format,
  version: B3Hash,
}).annotate({ identifier: "HeldDocument" });
export type HeldDocument = typeof HeldDocument.Type;

export const DocumentState = Schema.Union([
  Schema.Literal("missing"),
  Schema.Struct({
    unreadable: Schema.Struct({
      reason: Schema.String,
    }),
  }),
  Schema.Struct({
    empty: Schema.Struct({
      format: Format,
      version: B3Hash,
    }),
  }),
  Schema.Struct({
    held: HeldDocument,
  }),
]).annotate({ identifier: "DocumentState" });
export type DocumentState = typeof DocumentState.Type;

export const DocumentAnswer = Schema.Struct({
  at: Address,
  state: DocumentState,
}).annotate({ identifier: "DocumentAnswer" });
export type DocumentAnswer = typeof DocumentAnswer.Type;

export const Ceiling = Schema.Int.check(Schema.isGreaterThanOrEqualTo(1)).pipe(Schema.brand("Ceiling"));
export type Ceiling = typeof Ceiling.Type;

export const ModelTag = Schema.Union([
  Schema.Literal("main"),
  Schema.Literal("digest"),
  Schema.Literal("transcribe"),
  Schema.Literal("ocr"),
]).annotate({ identifier: "ModelTag" });
export type ModelTag = typeof ModelTag.Type;

export const ChosenSummary = Schema.Struct({
  endpoint: Schema.String,
  max_output_tokens: Schema.optional(Schema.NullOr(Ceiling)),
  model: Schema.String,
  tag: ModelTag,
}).annotate({ identifier: "ChosenSummary" });
export type ChosenSummary = typeof ChosenSummary.Type;

export const DialectKind = Schema.Union([
  Schema.Literal("anthropic"),
  Schema.Literal("open_ai"),
  Schema.Literal("open_ai_responses"),
]).annotate({ identifier: "DialectKind" });
export type DialectKind = typeof DialectKind.Type;

export const BodyOverride = Schema.Struct({
  pointer: Schema.String,
  value: Schema.String,
}).annotate({ identifier: "BodyOverride" });
export type BodyOverride = typeof BodyOverride.Type;

export const HeaderPair = Schema.Struct({
  name: Schema.String,
  value: Schema.String,
}).annotate({ identifier: "HeaderPair" });
export type HeaderPair = typeof HeaderPair.Type;

export const GenerationSettings = Schema.Struct({
  temperature: Schema.optional(Schema.NullOr(Schema.String)),
  top_k: Schema.optional(Schema.NullOr(Schema.String)),
  top_p: Schema.optional(Schema.NullOr(Schema.String)),
}).annotate({ identifier: "GenerationSettings" });
export type GenerationSettings = typeof GenerationSettings.Type;

export const ParameterSupport = Schema.Union([
  Schema.Literal("unknown"),
  Schema.Literal("supported"),
  Schema.Literal("unsupported"),
]).annotate({ identifier: "ParameterSupport" });
export type ParameterSupport = typeof ParameterSupport.Type;

export const GenerationSupport = Schema.Struct({
  temperature: Schema.optional(Schema.NullOr(ParameterSupport)),
  top_k: Schema.optional(Schema.NullOr(ParameterSupport)),
  top_p: Schema.optional(Schema.NullOr(ParameterSupport)),
}).annotate({ identifier: "GenerationSupport" });
export type GenerationSupport = typeof GenerationSupport.Type;

export const ModelGeneration = Schema.Struct({
  model: Schema.String,
  output_tokens: Schema.optional(Schema.NullOr(Ceiling)),
  settings: Schema.optional(GenerationSettings),
  support: Schema.optional(GenerationSupport),
}).annotate({ identifier: "ModelGeneration" });
export type ModelGeneration = typeof ModelGeneration.Type;

export const EndpointTuning = Schema.Struct({
  account_retries: Schema.optional(Schema.NullOr(AccountRetries)),
  accounts: Schema.optional(Schema.NullOr(Schema.Array(ProviderAccount))),
  generation: Schema.optional(Schema.Array(ModelGeneration)),
  headers: Schema.Array(HeaderPair),
  label: Schema.optional(Schema.NullOr(Schema.String)),
  max_in_flight: Schema.optional(Schema.NullOr(Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)))),
  overrides: Schema.Array(BodyOverride),
  proxying: Schema.optional(Schema.NullOr(Proxying)),
  request_max_retries: Schema.optional(Schema.NullOr(Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)))),
  stream_idle_timeout_ms: Schema.optional(Schema.NullOr(Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)))),
  timeout_ms: Schema.optional(Schema.NullOr(Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)))),
}).annotate({ identifier: "EndpointTuning" });
export type EndpointTuning = typeof EndpointTuning.Type;

export const ParameterSource = Schema.Union([
  Schema.Literal("person"),
  Schema.Literal("upstream"),
  Schema.Literal("preset"),
]).annotate({ identifier: "ParameterSource" });
export type ParameterSource = typeof ParameterSource.Type;

export const ParameterAnswer = Schema.Struct({
  source: Schema.optional(Schema.NullOr(ParameterSource)),
  support: ParameterSupport,
}).annotate({ identifier: "ParameterAnswer" });
export type ParameterAnswer = typeof ParameterAnswer.Type;

export const GenerationAnswer = Schema.Struct({
  output_pointer: Schema.String,
  output_tokens: Schema.optional(Schema.NullOr(Ceiling)),
  settings: GenerationSettings,
  temperature: ParameterAnswer,
  top_k: ParameterAnswer,
  top_p: ParameterAnswer,
}).annotate({ identifier: "GenerationAnswer" });
export type GenerationAnswer = typeof GenerationAnswer.Type;

export const Listing = Schema.Union([
  Schema.Literal("listed"),
  Schema.Literal("named"),
]).annotate({ identifier: "Listing" });
export type Listing = typeof Listing.Type;

export const EffortWord = Schema.Struct({
  effort: Effort,
  word: Schema.String,
}).annotate({ identifier: "EffortWord" });
export type EffortWord = typeof EffortWord.Type;

export const OfferSource = Schema.Union([
  Schema.Literal("person"),
  Schema.Literal("upstream"),
  Schema.Literal("preset"),
  Schema.Literal("unknown"),
]).annotate({ identifier: "OfferSource" });
export type OfferSource = typeof OfferSource.Type;

export const Switch = Schema.Literals(["allowed", "refused", "unknown"]).annotate({ identifier: "Switch" });
export type Switch = typeof Switch.Type;

export const ThinkingOffer = Schema.Struct({
  default: Schema.optional(Schema.NullOr(Effort)),
  default_on: Schema.optional(Schema.NullOr(Schema.Boolean)),
  from: OfferSource,
  levels: Schema.Array(Effort),
  on: Switch,
  source: Schema.optional(Schema.NullOr(Schema.String)),
  words: Schema.Array(EffortWord),
}).annotate({ identifier: "ThinkingOffer" });
export type ThinkingOffer = typeof ThinkingOffer.Type;

export const Window = Schema.Int.check(Schema.isGreaterThanOrEqualTo(1)).pipe(Schema.brand("Window"));
export type Window = typeof Window.Type;

export const ModelFactsSummary = Schema.Struct({
  canonical: Schema.String,
  context_tokens: Schema.optional(Schema.NullOr(Window)),
  generation: GenerationAnswer,
  id: Schema.String,
  input_modalities: Schema.Array(Schema.String),
  input_price: Schema.optional(Schema.NullOr(Schema.String)),
  listing: Listing,
  max_output_tokens: Schema.optional(Schema.NullOr(Ceiling)),
  output_price: Schema.optional(Schema.NullOr(Schema.String)),
  thinking: ThinkingOffer,
}).annotate({ identifier: "ModelFactsSummary" });
export type ModelFactsSummary = typeof ModelFactsSummary.Type;

export const EndpointSummary = Schema.Struct({
  account_status: Schema.Array(AccountStatus),
  base_url: Schema.String,
  connection_kind: Schema.String,
  dialect: DialectKind,
  has_credential: Schema.Boolean,
  label: Schema.String,
  local: Schema.Boolean,
  models: Schema.Array(ModelFactsSummary),
  name: Schema.String,
  tuning: EndpointTuning,
}).annotate({ identifier: "EndpointSummary" });
export type EndpointSummary = typeof EndpointSummary.Type;

export const EndpointsAnswer = Schema.Struct({
  chosen: Schema.Array(ChosenSummary),
  endpoints: Schema.Array(EndpointSummary),
}).annotate({ identifier: "EndpointsAnswer" });
export type EndpointsAnswer = typeof EndpointsAnswer.Type;

export const EvidenceKind = Schema.Union([
  Schema.Literal("screenshot"),
  Schema.Literal("finished"),
]).annotate({ identifier: "EvidenceKind" });
export type EvidenceKind = typeof EvidenceKind.Type;

export const Picture = Schema.Struct({
  height: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  media_type: Schema.String,
  width: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
}).annotate({ identifier: "Picture" });
export type Picture = typeof Picture.Type;

export const EvidenceItem = Schema.Struct({
  at: Seq,
  kind: EvidenceKind,
  locator: Locator,
  picture: Schema.optional(Schema.NullOr(Picture)),
}).annotate({ identifier: "EvidenceItem" });
export type EvidenceItem = typeof EvidenceItem.Type;

export const EvidenceAnswer = Schema.Struct({
  items: Schema.Array(EvidenceItem),
  run: RunId,
}).annotate({ identifier: "EvidenceAnswer" });
export type EvidenceAnswer = typeof EvidenceAnswer.Type;

export const ExportAnswer = Schema.Struct({
  html: Schema.String,
  version: B3Hash,
}).annotate({ identifier: "ExportAnswer" });
export type ExportAnswer = typeof ExportAnswer.Type;

export const Walked = Schema.Union([
  Schema.Literal("whole"),
  Schema.Literal("cut"),
]).annotate({ identifier: "Walked" });
export type Walked = typeof Walked.Type;

export const FindAnswer = Schema.Struct({
  paths: Schema.Array(Address),
  text: Schema.String,
  under: Address,
  walked: Walked,
}).annotate({ identifier: "FindAnswer" });
export type FindAnswer = typeof FindAnswer.Type;

export const Drift = Schema.Struct({
  ahead: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  behind: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
}).annotate({ identifier: "Drift" });
export type Drift = typeof Drift.Type;

export const GitStatusAnswer = Schema.Struct({
  branch: Schema.optional(Schema.NullOr(Schema.String)),
  building: Address,
  checkpoint: Schema.optional(Schema.NullOr(CommitAnswer)),
  drift: Schema.optional(Schema.NullOr(Drift)),
  files: Schema.Array(FileChange),
}).annotate({ identifier: "GitStatusAnswer" });
export type GitStatusAnswer = typeof GitStatusAnswer.Type;

export const GithubReading = Schema.Union([
  Schema.Struct({
    found: Schema.Struct({
      login: Schema.String,
    }),
  }),
  Schema.Literal("no_cli"),
  Schema.Literal("not_logged_in"),
  Schema.Struct({
    failed: Schema.Struct({
      exit: Schema.optional(Schema.NullOr(Schema.Int)),
    }),
  }),
  Schema.Struct({
    stuck: Schema.Struct({
      why: Schema.String,
    }),
  }),
  Schema.Literal("not_a_host"),
]).annotate({ identifier: "GithubReading" });
export type GithubReading = typeof GithubReading.Type;

export const GithubLoginAnswer = Schema.Struct({
  host: Schema.String,
  reading: GithubReading,
}).annotate({ identifier: "GithubLoginAnswer" });
export type GithubLoginAnswer = typeof GithubLoginAnswer.Type;

export const ResidentId = Schema.String.pipe(Schema.brand("ResidentId"));
export type ResidentId = typeof ResidentId.Type;

export const Autonomy = Schema.Union([
  Schema.Literal("owner"),
  Schema.Struct({
    delegate: ResidentId,
  }),
]).annotate({ identifier: "Autonomy" });
export type Autonomy = typeof Autonomy.Type;

export const Ruling = Schema.Literals(["allow", "deny"]).annotate({ identifier: "Ruling" });
export type Ruling = typeof Ruling.Type;

export const Decision = Schema.Struct({
  at: TimeMs,
  cluster: ClusterKey,
  item: Schema.String,
  verdict: Ruling,
}).annotate({ identifier: "Decision" });
export type Decision = typeof Decision.Type;

export const GovernanceAnswer = Schema.Struct({
  autonomy: Autonomy,
  decided: Schema.Array(Decision),
}).annotate({ identifier: "GovernanceAnswer" });
export type GovernanceAnswer = typeof GovernanceAnswer.Type;

export const GuideMark = Schema.Literals(["seen", "skipped"]).annotate({ identifier: "GuideMark" });
export type GuideMark = typeof GuideMark.Type;

export const GuideState = Schema.Union([
  Schema.Literal("open"),
  Schema.Literal("left"),
]).annotate({ identifier: "GuideState" });
export type GuideState = typeof GuideState.Type;

export const GuideStep = Schema.Literals(["provider", "dependencies", "texts", "skills", "mcp"]).annotate({ identifier: "GuideStep" });
export type GuideStep = typeof GuideStep.Type;

export const GuideProgress = Schema.Struct({
  at: Schema.optional(Schema.NullOr(GuideStep)),
  dependencies: Schema.optional(Schema.NullOr(GuideMark)),
  mcp: Schema.optional(Schema.NullOr(GuideMark)),
  skills: Schema.optional(Schema.NullOr(GuideMark)),
  state: Schema.optional(GuideState),
  texts: Schema.optional(Schema.NullOr(GuideMark)),
}).annotate({ identifier: "GuideProgress" });
export type GuideProgress = typeof GuideProgress.Type;

export const EventRecord = Schema.Struct({
  addr: Schema.optional(Schema.NullOr(Address)),
  data: Payload,
  ig: Schema.optional(Schema.Boolean),
  kind: EventKind,
  prev: B3Hash,
  run: RunId,
  seq: Seq,
  t: TimeMs,
  v: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  who: Schema.String,
}).annotate({ identifier: "EventRecord" });
export type EventRecord = typeof EventRecord.Type;

export const HistoryAnswer = Schema.Struct({
  earlier: Schema.optional(Schema.NullOr(Seq)),
  records: Schema.Array(EventRecord),
}).annotate({ identifier: "HistoryAnswer" });
export type HistoryAnswer = typeof HistoryAnswer.Type;

export const HistoryRangeAnswer = Schema.Struct({
  from: Seq,
  next: Schema.optional(Schema.NullOr(Seq)),
  records: Schema.Array(EventRecord),
  to: Seq,
}).annotate({ identifier: "HistoryRangeAnswer" });
export type HistoryRangeAnswer = typeof HistoryRangeAnswer.Type;

export const PatchLine = Schema.Struct({
  number: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  text: Schema.String,
}).annotate({ identifier: "PatchLine" });
export type PatchLine = typeof PatchLine.Type;

export const Withheld = Schema.Struct({
  number: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  reason: Schema.String,
}).annotate({ identifier: "Withheld" });
export type Withheld = typeof Withheld.Type;

export const HunksAnswer = Schema.Struct({
  lines: Schema.Array(PatchLine),
  oid_a: GitOid,
  oid_b: GitOid,
  path: Schema.String,
  withheld: Schema.Array(Withheld),
}).annotate({ identifier: "HunksAnswer" });
export type HunksAnswer = typeof HunksAnswer.Type;

export const GovernedDocument = Schema.Union([
  Schema.Literal("mayor"),
  Schema.Literal("clerk"),
  Schema.Literal("preferences"),
]).annotate({ identifier: "GovernedDocument" });
export type GovernedDocument = typeof GovernedDocument.Type;

export const StatedIdentity = Schema.Struct({
  about: Schema.String,
  imported_from: Schema.optional(Schema.NullOr(Schema.String)),
  mayor: Schema.optional(Schema.NullOr(Schema.String)),
  mayor_text: Schema.String,
  preferences_text: Schema.String,
  user_id: Schema.optional(Schema.NullOr(Schema.String)),
  version: Schema.String,
}).annotate({ identifier: "StatedIdentity" });
export type StatedIdentity = typeof StatedIdentity.Type;

export const IdentityAnswer = Schema.Union([
  Schema.Struct({
    stated: StatedIdentity,
  }),
  Schema.Struct({
    unreadable: Schema.Struct({
      document: GovernedDocument,
      line: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
      why: Schema.String,
    }),
  }),
]).annotate({ identifier: "IdentityAnswer" });
export type IdentityAnswer = typeof IdentityAnswer.Type;

export const SignalLine = Schema.Struct({
  at: TimeMs,
  first_line: Schema.optional(Schema.NullOr(Schema.String)),
  from: Schema.String,
  id: Schema.String,
  kind: Schema.String,
}).annotate({ identifier: "SignalLine" });
export type SignalLine = typeof SignalLine.Type;

export const InboxAnswer = Schema.Struct({
  addr: Address,
  waiting: Schema.Array(SignalLine),
}).annotate({ identifier: "InboxAnswer" });
export type InboxAnswer = typeof InboxAnswer.Type;

export const KnownFace = Schema.Struct({
  base_url: Schema.String,
  dialect: DialectKind,
}).annotate({ identifier: "KnownFace" });
export type KnownFace = typeof KnownFace.Type;

export const KnownHost = Schema.Struct({
  faces: Schema.Array(KnownFace),
  host: Schema.String,
}).annotate({ identifier: "KnownHost" });
export type KnownHost = typeof KnownHost.Type;

export const KnownHostsAnswer = Schema.Struct({
  hosts: Schema.Array(KnownHost),
}).annotate({ identifier: "KnownHostsAnswer" });
export type KnownHostsAnswer = typeof KnownHostsAnswer.Type;

export const Construct = Schema.Union([
  Schema.Literal("html"),
  Schema.Literal("math"),
  Schema.Literal("front_matter"),
  Schema.Literal("nesting"),
  Schema.Literal("extension"),
]).annotate({ identifier: "Construct" });
export type Construct = typeof Construct.Type;

export type Inline = "soft_break" | "line_break" | { readonly text: string } | { readonly code: string } | { readonly emphasis: readonly Inline[] } | { readonly strong: readonly Inline[] } | { readonly strikethrough: readonly Inline[] } | { readonly link: { readonly content: readonly Inline[]; readonly target: string; readonly title: string } } | { readonly image: { readonly alt: string; readonly target: string; readonly title: string } } | { readonly footnote_reference: { readonly name: string } } | { readonly unsupported: { readonly construct: typeof Construct.Type; readonly source: string } }
export type InlineEncoded = "soft_break" | "line_break" | { readonly text: string } | { readonly code: string } | { readonly emphasis: readonly InlineEncoded[] } | { readonly strong: readonly InlineEncoded[] } | { readonly strikethrough: readonly InlineEncoded[] } | { readonly link: { readonly content: readonly InlineEncoded[]; readonly target: string; readonly title: string } } | { readonly image: { readonly alt: string; readonly target: string; readonly title: string } } | { readonly footnote_reference: { readonly name: string } } | { readonly unsupported: { readonly construct: typeof Construct.Encoded; readonly source: string } }
export const Inline: Schema.Codec<Inline, InlineEncoded> = Schema.Union([
  Schema.Literals(["soft_break", "line_break"]),
  Schema.Struct({
    text: Schema.String,
  }),
  Schema.Struct({
    code: Schema.String,
  }),
  Schema.Struct({
    emphasis: Schema.Array(Schema.suspend((): Schema.Codec<Inline, InlineEncoded> => Inline)),
  }),
  Schema.Struct({
    strong: Schema.Array(Schema.suspend((): Schema.Codec<Inline, InlineEncoded> => Inline)),
  }),
  Schema.Struct({
    strikethrough: Schema.Array(Schema.suspend((): Schema.Codec<Inline, InlineEncoded> => Inline)),
  }),
  Schema.Struct({
    link: Schema.Struct({
      content: Schema.Array(Schema.suspend((): Schema.Codec<Inline, InlineEncoded> => Inline)),
      target: Schema.String,
      title: Schema.String,
    }),
  }),
  Schema.Struct({
    image: Schema.Struct({
      alt: Schema.String,
      target: Schema.String,
      title: Schema.String,
    }),
  }),
  Schema.Struct({
    footnote_reference: Schema.Struct({
      name: Schema.String,
    }),
  }),
  Schema.Struct({
    unsupported: Schema.Struct({
      construct: Construct,
      source: Schema.String,
    }),
  }),
]).annotate({ identifier: "Inline" });

export const Check = Schema.Literals(["not_a_task", "open", "done"]).annotate({ identifier: "Check" });
export type Check = typeof Check.Type;

export interface ListItem { readonly blocks: readonly Block[]; readonly check: typeof Check.Type; readonly span: typeof Span.Type }
export interface ListItemEncoded { readonly blocks: readonly BlockEncoded[]; readonly check: typeof Check.Encoded; readonly span: typeof Span.Encoded }
export const ListItem: Schema.Codec<ListItem, ListItemEncoded> = Schema.Struct({
  blocks: Schema.Array(Schema.suspend((): Schema.Codec<Block, BlockEncoded> => Block)),
  check: Check,
  span: Span,
}).annotate({ identifier: "ListItem" });

export const Order = Schema.Union([
  Schema.Literal("bullet"),
  Schema.Struct({
    ordered: Schema.Struct({
      start: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
    }),
  }),
]).annotate({ identifier: "Order" });
export type Order = typeof Order.Type;

export const Row = Schema.Struct({
  cells: Schema.Array(Schema.Array(Inline)),
}).annotate({ identifier: "Row" });
export type Row = typeof Row.Type;

export const Spacing = Schema.Literals(["tight", "loose"]).annotate({ identifier: "Spacing" });
export type Spacing = typeof Spacing.Type;

export type Block = { readonly heading: { readonly inline: readonly Inline[]; readonly level: number; readonly span: typeof Span.Type } } | { readonly paragraph: { readonly inline: readonly Inline[]; readonly span: typeof Span.Type } } | { readonly list: { readonly items: readonly ListItem[]; readonly order: typeof Order.Type; readonly spacing: typeof Spacing.Type; readonly span: typeof Span.Type } } | { readonly quote: { readonly blocks: readonly Block[]; readonly span: typeof Span.Type } } | { readonly code: { readonly info: string; readonly span: typeof Span.Type; readonly text: string } } | { readonly table: { readonly align: readonly (typeof Align.Type)[]; readonly body: readonly (typeof Row.Type)[]; readonly head: typeof Row.Type; readonly span: typeof Span.Type } } | { readonly rule: { readonly span: typeof Span.Type } } | { readonly footnote: { readonly blocks: readonly Block[]; readonly name: string; readonly span: typeof Span.Type } } | { readonly unsupported: { readonly construct: typeof Construct.Type; readonly source: string; readonly span: typeof Span.Type } }
export type BlockEncoded = { readonly heading: { readonly inline: readonly InlineEncoded[]; readonly level: number; readonly span: typeof Span.Encoded } } | { readonly paragraph: { readonly inline: readonly InlineEncoded[]; readonly span: typeof Span.Encoded } } | { readonly list: { readonly items: readonly ListItemEncoded[]; readonly order: typeof Order.Encoded; readonly spacing: typeof Spacing.Encoded; readonly span: typeof Span.Encoded } } | { readonly quote: { readonly blocks: readonly BlockEncoded[]; readonly span: typeof Span.Encoded } } | { readonly code: { readonly info: string; readonly span: typeof Span.Encoded; readonly text: string } } | { readonly table: { readonly align: readonly (typeof Align.Encoded)[]; readonly body: readonly (typeof Row.Encoded)[]; readonly head: typeof Row.Encoded; readonly span: typeof Span.Encoded } } | { readonly rule: { readonly span: typeof Span.Encoded } } | { readonly footnote: { readonly blocks: readonly BlockEncoded[]; readonly name: string; readonly span: typeof Span.Encoded } } | { readonly unsupported: { readonly construct: typeof Construct.Encoded; readonly source: string; readonly span: typeof Span.Encoded } }
export const Block: Schema.Codec<Block, BlockEncoded> = Schema.Union([
  Schema.Struct({
    heading: Schema.Struct({
      inline: Schema.Array(Schema.suspend((): Schema.Codec<Inline, InlineEncoded> => Inline)),
      level: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)).check(Schema.isLessThanOrEqualTo(255)),
      span: Span,
    }),
  }),
  Schema.Struct({
    paragraph: Schema.Struct({
      inline: Schema.Array(Schema.suspend((): Schema.Codec<Inline, InlineEncoded> => Inline)),
      span: Span,
    }),
  }),
  Schema.Struct({
    list: Schema.Struct({
      items: Schema.Array(Schema.suspend((): Schema.Codec<ListItem, ListItemEncoded> => ListItem)),
      order: Order,
      spacing: Spacing,
      span: Span,
    }),
  }),
  Schema.Struct({
    quote: Schema.Struct({
      blocks: Schema.Array(Schema.suspend((): Schema.Codec<Block, BlockEncoded> => Block)),
      span: Span,
    }),
  }),
  Schema.Struct({
    code: Schema.Struct({
      info: Schema.String,
      span: Span,
      text: Schema.String,
    }),
  }),
  Schema.Struct({
    table: Schema.Struct({
      align: Schema.Array(Align),
      body: Schema.Array(Row),
      head: Row,
      span: Span,
    }),
  }),
  Schema.Struct({
    rule: Schema.Struct({
      span: Span,
    }),
  }),
  Schema.Struct({
    footnote: Schema.Struct({
      blocks: Schema.Array(Schema.suspend((): Schema.Codec<Block, BlockEncoded> => Block)),
      name: Schema.String,
      span: Span,
    }),
  }),
  Schema.Struct({
    unsupported: Schema.Struct({
      construct: Construct,
      source: Schema.String,
      span: Span,
    }),
  }),
]).annotate({ identifier: "Block" });

export const Laid = Schema.Struct({
  blocks: Schema.Array(Block),
  span: Span,
}).annotate({ identifier: "Laid" });
export type Laid = typeof Laid.Type;

export const LayerState = Schema.Union([
  Schema.Struct({
    sent: Schema.Struct({
      bytes: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
    }),
  }),
  Schema.Struct({
    replaced: Schema.Struct({
      by: Address,
    }),
  }),
  Schema.Literal("absent"),
  Schema.Struct({
    withheld: Schema.Struct({
      why: Schema.String,
    }),
  }),
]).annotate({ identifier: "LayerState" });
export type LayerState = typeof LayerState.Type;

export const PrefixSlot = Schema.Literals(["city", "building", "resident", "run"]).annotate({ identifier: "PrefixSlot" });
export type PrefixSlot = typeof PrefixSlot.Type;

export const PromptKind = Schema.Union([
  Schema.Literal("city"),
  Schema.Literal("conventions"),
  Schema.Literal("identity"),
  Schema.Struct({
    model_note: Schema.Struct({
      model: Schema.String,
      provider: Schema.String,
    }),
  }),
]).annotate({ identifier: "PromptKind" });
export type PromptKind = typeof PromptKind.Type;

export const PromptLayer = Schema.Literals(["city", "building", "room"]).annotate({ identifier: "PromptLayer" });
export type PromptLayer = typeof PromptLayer.Type;

export const DefaultStanding = Schema.Literals(["absent", "current", "earlier", "edited"]).annotate({ identifier: "DefaultStanding" });
export type DefaultStanding = typeof DefaultStanding.Type;

export const ShippedDefault = Schema.Struct({
  standing: DefaultStanding,
  version: B3Hash,
}).annotate({ identifier: "ShippedDefault" });
export type ShippedDefault = typeof ShippedDefault.Type;

export const LayerDocument = Schema.Struct({
  at: Address,
  kind: PromptKind,
  layer: PromptLayer,
  shipped: Schema.optional(Schema.NullOr(ShippedDefault)),
  slot: PrefixSlot,
  state: LayerState,
  version: B3Hash,
}).annotate({ identifier: "LayerDocument" });
export type LayerDocument = typeof LayerDocument.Type;

export const LayersAnswer = Schema.Struct({
  addr: Address,
  documents: Schema.Array(LayerDocument),
}).annotate({ identifier: "LayersAnswer" });
export type LayersAnswer = typeof LayersAnswer.Type;

export const EntryKind = Schema.Union([
  Schema.Literal("directory"),
  Schema.Struct({
    file: Schema.Struct({
      bytes: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
    }),
  }),
]).annotate({ identifier: "EntryKind" });
export type EntryKind = typeof EntryKind.Type;

export const Entry = Schema.Struct({
  kind: EntryKind,
  name: Schema.String,
}).annotate({ identifier: "Entry" });
export type Entry = typeof Entry.Type;

export const ListingAnswer = Schema.Struct({
  at: Schema.optional(Schema.NullOr(Address)),
  entries: Schema.Array(Entry),
}).annotate({ identifier: "ListingAnswer" });
export type ListingAnswer = typeof ListingAnswer.Type;

export const AxCode = Schema.Literals(["E_PATH_NOT_FOUND", "E_TOOL_UNKNOWN", "E_TOOL_UNAVAILABLE", "E_INVALID_ARGS", "E_OUTSIDE_WRITE_DOMAIN", "E_VERSION_CONFLICT", "E_GATE_DENIED", "E_BUDGET_EXHAUSTED", "E_TIMEOUT", "E_PROVIDER", "E_EVIDENCE_MISSING", "E_LOOP_SUSPECTED", "E_LOCATOR_INVALID", "E_SANDBOX_DENIED", "E_BUSY", "E_DRAFT_STALE", "E_GOAL_CONFLICT", "E_TAINTED_ACTION", "E_REPAIR_BUSY", "E_DELEGATION_DEPTH", "E_APPROVAL_PENDING", "E_APPROVAL_DENIED", "E_CROSS_BUILDING_DENIED", "E_DIGEST_SUSPECT", "E_CREDENTIAL_MISSING", "E_MODEL_UNCHOSEN", "E_CONFIG_INVALID", "E_CAS_CORRUPT", "E_STORAGE_FATAL", "E_WORKTREE_BUSY", "E_BROWSER_UNAVAILABLE", "E_ENDPOINT_DIALECT_UNSUPPORTED", "E_WIRE_MISMATCH", "E_LOG_VERSION_UNSUPPORTED", "E_LEDGER_HELD", "E_HISTORY_UNPROVEN", "E_SECRET_EGRESS", "E_DISCARD_IRREVERSIBLE", "E_BACKPRESSURE_SHED", "E_TOOL_OUTCOME_UNKNOWN", "E_PLAN_MISSING", "E_PROVIDER_ACCOUNTS_EXHAUSTED", "E_AUTH_REQUIRED", "E_PAIRING_REFUSED"]).annotate({ identifier: "AxCode" });
export type AxCode = typeof AxCode.Type;

export const GateRefusal = Schema.Struct({
  alternative: Schema.String,
  rule: Schema.String,
  violation: Schema.String,
}).annotate({ identifier: "GateRefusal" });
export type GateRefusal = typeof GateRefusal.Type;

export const ProviderFailureKind = Schema.Union([
  Schema.Struct({
    kind: Schema.Literal("exchange"),
  }),
  Schema.Struct({
    kind: Schema.Literal("cut"),
  }),
  Schema.Struct({
    kind: Schema.Literal("silence"),
  }),
  Schema.Struct({
    kind: Schema.Literal("refused"),
    status: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  }),
  Schema.Struct({
    kind: Schema.Literal("quota"),
    status: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  }),
  Schema.Struct({
    kind: Schema.Literal("overflow"),
    status: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  }),
  Schema.Struct({
    kind: Schema.Literal("unreadable"),
  }),
  Schema.Struct({
    kind: Schema.Literal("reported"),
  }),
  Schema.Struct({
    kind: Schema.Literal("unbuilt"),
  }),
]).annotate({ identifier: "ProviderFailureKind" });
export type ProviderFailureKind = typeof ProviderFailureKind.Type;

export const Retry = Schema.Union([
  Schema.Literal("yes"),
  Schema.Literal("no"),
  Schema.Literal("unknown"),
]).annotate({ identifier: "Retry" });
export type Retry = typeof Retry.Type;

export const AxError = Schema.Struct({
  account: Schema.optional(AccountDisposition),
  action: Schema.String,
  code: AxCode,
  gate: Schema.optional(Schema.NullOr(GateRefusal)),
  nearby: Schema.Array(Schema.String),
  provider: Schema.optional(Schema.NullOr(ProviderFailureKind)),
  recovery: Schema.String,
  retry: Retry,
  retry_after_ms: Schema.optional(Schema.NullOr(Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)))),
  subject: Schema.String,
}).annotate({ identifier: "AxError" });
export type AxError = typeof AxError.Type;

export const McpToolLine = Schema.Struct({
  disclosure: Schema.String,
  input_schema: Payload,
  name: Schema.String,
  remote: Schema.String,
}).annotate({ identifier: "McpToolLine" });
export type McpToolLine = typeof McpToolLine.Type;

export const McpState = Schema.Union([
  Schema.Struct({
    connected: Schema.Struct({
      protocol_version: Schema.String,
      server: Schema.String,
      tools: Schema.Array(McpToolLine),
    }),
  }),
  Schema.Struct({
    authenticating: Schema.Struct({
      recovery: Schema.String,
    }),
  }),
  Schema.Struct({
    failed: Schema.Struct({
      refusal: AxError,
    }),
  }),
]).annotate({ identifier: "McpState" });
export type McpState = typeof McpState.Type;

export const McpServerHealth = Schema.Struct({
  label: ServerLabel,
  state: McpState,
  target: Schema.String,
  transport: Schema.String,
}).annotate({ identifier: "McpServerHealth" });
export type McpServerHealth = typeof McpServerHealth.Type;

export const McpHealthAnswer = Schema.Struct({
  addr: Address,
  servers: Schema.Array(McpServerHealth),
}).annotate({ identifier: "McpHealthAnswer" });
export type McpHealthAnswer = typeof McpHealthAnswer.Type;

export const DayCount = Schema.Struct({
  count: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  day: Schema.String,
}).annotate({ identifier: "DayCount" });
export type DayCount = typeof DayCount.Type;

export const UseOutcome = Schema.Union([
  Schema.Literals(["ok", "failed"]),
  Schema.Literal("unknown"),
]).annotate({ identifier: "UseOutcome" });
export type UseOutcome = typeof UseOutcome.Type;

export const McpUse = Schema.Struct({
  at: TimeMs,
  outcome: UseOutcome,
  resident: Schema.optional(Schema.NullOr(Address)),
  run: RunId,
  seq: Seq,
}).annotate({ identifier: "McpUse" });
export type McpUse = typeof McpUse.Type;

export const McpToolUsage = Schema.Struct({
  per_day: Schema.Array(DayCount),
  tool: Schema.String,
  uses: Schema.Array(McpUse),
}).annotate({ identifier: "McpToolUsage" });
export type McpToolUsage = typeof McpToolUsage.Type;

export const McpServerUsage = Schema.Struct({
  configured: Schema.Boolean,
  server: Schema.optional(Schema.NullOr(Schema.String)),
  tools: Schema.Array(McpToolUsage),
}).annotate({ identifier: "McpServerUsage" });
export type McpServerUsage = typeof McpServerUsage.Type;

export const McpUsageAnswer = Schema.Struct({
  servers: Schema.Array(McpServerUsage),
}).annotate({ identifier: "McpUsageAnswer" });
export type McpUsageAnswer = typeof McpUsageAnswer.Type;

export const MetricsAnswer = Schema.Struct({
  approvals_waiting: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  buildings: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  discards_outstanding: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  events: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  runs_active: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  runs_frozen: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  signals_waiting: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
}).annotate({ identifier: "MetricsAnswer" });
export type MetricsAnswer = typeof MetricsAnswer.Type;

export const FigureFrom = Schema.Union([
  Schema.Literal("person"),
  Schema.Literal("upstream"),
  Schema.Literal("registered"),
  Schema.Literal("preset"),
  Schema.Literal("policy"),
]).annotate({ identifier: "FigureFrom" });
export type FigureFrom = typeof FigureFrom.Type;

export const CeilingLine = Schema.Union([
  Schema.Struct({
    sent: Schema.Struct({
      from: FigureFrom,
      tokens: Ceiling,
    }),
  }),
  Schema.Literal("provider_default"),
]).annotate({ identifier: "CeilingLine" });
export type CeilingLine = typeof CeilingLine.Type;

export const InputKinds = Schema.Literals(["text", "text_image"]).annotate({ identifier: "InputKinds" });
export type InputKinds = typeof InputKinds.Type;

export const WindowLine = Schema.Struct({
  from: FigureFrom,
  tokens: Window,
}).annotate({ identifier: "WindowLine" });
export type WindowLine = typeof WindowLine.Type;

export const FiguresLine = Schema.Struct({
  ceiling: Schema.optional(Schema.NullOr(CeilingLine)),
  id: Schema.String,
  input: InputKinds,
  window: Schema.optional(Schema.NullOr(WindowLine)),
}).annotate({ identifier: "FiguresLine" });
export type FiguresLine = typeof FiguresLine.Type;

export const ModelFiguresAnswer = Schema.Struct({
  models: Schema.Array(FiguresLine),
}).annotate({ identifier: "ModelFiguresAnswer" });
export type ModelFiguresAnswer = typeof ModelFiguresAnswer.Type;

export const OfferedCard = Schema.Struct({
  at: Schema.optional(Schema.NullOr(TimeMs)),
  doc: Address,
  id: B3Hash,
}).annotate({ identifier: "OfferedCard" });
export type OfferedCard = typeof OfferedCard.Type;

export const OpenProposalsAnswer = Schema.Struct({
  open: Schema.Array(OfferedCard),
}).annotate({ identifier: "OpenProposalsAnswer" });
export type OpenProposalsAnswer = typeof OpenProposalsAnswer.Type;

export const Chroma = Schema.Literals(["full", "off"]).annotate({ identifier: "Chroma" });
export type Chroma = typeof Chroma.Type;

export const Density = Schema.Literals(["comfortable", "compact"]).annotate({ identifier: "Density" });
export type Density = typeof Density.Type;

export const Face = Schema.Union([
  Schema.Literal("paper"),
  Schema.Literal("maple"),
  Schema.Literal("system"),
  Schema.Literal("custom"),
]).annotate({ identifier: "Face" });
export type Face = typeof Face.Type;

export const Glass = Schema.Literals(["on", "off"]).annotate({ identifier: "Glass" });
export type Glass = typeof Glass.Type;

export const Lighting = Schema.Literals(["system", "dark", "light"]).annotate({ identifier: "Lighting" });
export type Lighting = typeof Lighting.Type;

export const Motion = Schema.Literals(["system", "on", "off"]).annotate({ identifier: "Motion" });
export type Motion = typeof Motion.Type;

export const ReadingFace = Schema.Union([
  Schema.Literal("interface"),
  Schema.Literal("libron"),
]).annotate({ identifier: "ReadingFace" });
export type ReadingFace = typeof ReadingFace.Type;

export const TypeFeature = Schema.Union([
  Schema.Literal("coding_ligatures"),
  Schema.Literal("duospace"),
  Schema.Literal("narrow_space"),
  Schema.Literal("small_arrows"),
  Schema.Literal("single_story_a"),
  Schema.Literal("slashed_zero"),
]).annotate({ identifier: "TypeFeature" });
export type TypeFeature = typeof TypeFeature.Type;

export const Weight = Schema.Literals(["light", "regular", "medium"]).annotate({ identifier: "Weight" });
export type Weight = typeof Weight.Type;

export const Appearance = Schema.Struct({
  blend_percent: Schema.optional(Schema.NullOr(Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)))),
  body_px: Schema.optional(Schema.NullOr(Schema.Int.check(Schema.isGreaterThanOrEqualTo(12)))),
  chroma: Chroma,
  density: Density,
  fallbacks: Schema.optional(Schema.Array(Schema.String)),
  features: Schema.optional(Schema.Array(TypeFeature)),
  glass: Schema.optional(Schema.NullOr(Glass)),
  letter_spacing: Schema.optional(Schema.NullOr(Schema.Int.check(Schema.isGreaterThanOrEqualTo(-50)))),
  lighting: Lighting,
  line_height: Schema.optional(Schema.NullOr(Schema.Int.check(Schema.isGreaterThanOrEqualTo(1200)))),
  mono: Face,
  mono_stack: Schema.String,
  motion: Motion,
  reading: Schema.optional(Schema.NullOr(ReadingFace)),
  sans: Face,
  sans_stack: Schema.String,
  weight: Schema.optional(Schema.NullOr(Weight)),
}).annotate({ identifier: "Appearance" });
export type Appearance = typeof Appearance.Type;

export const Chord = Schema.Struct({
  action: Schema.String,
  spelled: Schema.String,
}).annotate({ identifier: "Chord" });
export type Chord = typeof Chord.Type;

export const CorePlacement = Schema.Literals(["none", "soft", "soft_shares", "pinned"]).annotate({ identifier: "CorePlacement" });
export type CorePlacement = typeof CorePlacement.Type;

export const CorePriority = Schema.Literals(["raised", "normal"]).annotate({ identifier: "CorePriority" });
export type CorePriority = typeof CorePriority.Type;

export const CorePreferences = Schema.Struct({
  memory_bytes: Schema.optional(Schema.NullOr(Schema.Int.check(Schema.isGreaterThanOrEqualTo(1)))),
  placement: Schema.optional(CorePlacement),
  priority: Schema.optional(CorePriority),
}).annotate({ identifier: "CorePreferences" });
export type CorePreferences = typeof CorePreferences.Type;

export const Lang = Schema.Literals(["en", "zh"]).annotate({ identifier: "Lang" });
export type Lang = typeof Lang.Type;

export const Tag = Schema.String.check(Schema.isPattern(new RegExp("^[-_\\p{Alphabetic}\\p{N}]{1,24}$", "u"))).pipe(Schema.brand("Tag"));
export type Tag = typeof Tag.Type;

export const SessionTags = Schema.Struct({
  began: Seq,
  city: Address,
  room: Address,
  tags: Schema.Array(Tag),
}).annotate({ identifier: "SessionTags" });
export type SessionTags = typeof SessionTags.Type;

export const ThemeOverride = Schema.Struct({
  css: Schema.optional(Schema.NullOr(Schema.String)),
  tokens: Schema.optional(Schema.Record(Schema.String, Schema.String)),
}).annotate({ identifier: "ThemeOverride" });
export type ThemeOverride = typeof ThemeOverride.Type;

export const Tier = Schema.Literals(["zen", "blend", "panorama"]).annotate({ identifier: "Tier" });
export type Tier = typeof Tier.Type;

export const PreferencesAnswer = Schema.Struct({
  appearance: Schema.optional(Appearance),
  chords: Schema.optional(Schema.Array(Chord)),
  core: Schema.optional(CorePreferences),
  lang: Schema.optional(Schema.NullOr(Lang)),
  panel: Schema.optional(Schema.Boolean),
  proxying: Schema.optional(Proxying),
  tags: Schema.optional(Schema.Array(SessionTags)),
  theme: Schema.optional(ThemeOverride),
  tier: Schema.optional(Schema.NullOr(Tier)),
  welcomed: Schema.optional(Schema.Boolean),
}).annotate({ identifier: "PreferencesAnswer" });
export type PreferencesAnswer = typeof PreferencesAnswer.Type;

export const PrefixSource = Schema.Struct({
  addr: Address,
  dropped: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  kept: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
}).annotate({ identifier: "PrefixSource" });
export type PrefixSource = typeof PrefixSource.Type;

export const PrefixSegment = Schema.Struct({
  bytes: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  hash: B3Hash,
  slot: PrefixSlot,
  sources: Schema.Array(PrefixSource),
  stored: Schema.Boolean,
  text: Schema.String,
}).annotate({ identifier: "PrefixSegment" });
export type PrefixSegment = typeof PrefixSegment.Type;

export const PromptTool = Schema.Struct({
  digest: Schema.optional(Schema.NullOr(Schema.String)),
  exposure: Exposure,
  from: ExposureFrom,
  hint: Schema.String,
  len: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  name: Schema.String,
  namespace: Schema.String,
}).annotate({ identifier: "PromptTool" });
export type PromptTool = typeof PromptTool.Type;

export const PromptTools = Schema.Struct({
  always: Schema.optional(Schema.Int.check(Schema.isGreaterThanOrEqualTo(0))),
  dormant: Schema.optional(Schema.Int.check(Schema.isGreaterThanOrEqualTo(0))),
  hash: Schema.String,
  index: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  len: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  reading: Schema.optional(Schema.Int.check(Schema.isGreaterThanOrEqualTo(0))),
  rows: Schema.Array(PromptTool),
  skills: Schema.optional(Schema.Int.check(Schema.isGreaterThanOrEqualTo(0))),
}).annotate({ identifier: "PromptTools" });
export type PromptTools = typeof PromptTools.Type;

export const PrefixAnswer = Schema.Struct({
  run: RunId,
  segments: Schema.Array(PrefixSegment),
  tools: Schema.optional(Schema.NullOr(PromptTools)),
}).annotate({ identifier: "PrefixAnswer" });
export type PrefixAnswer = typeof PrefixAnswer.Type;

export const Preview = Schema.Union([
  Schema.Struct({
    laid: Laid,
  }),
  Schema.Struct({
    unsupported: Schema.Struct({
      encoding: Encoding,
    }),
  }),
]).annotate({ identifier: "Preview" });
export type Preview = typeof Preview.Type;

export const PreviewAnswer = Schema.Struct({
  preview: Preview,
  version: B3Hash,
}).annotate({ identifier: "PreviewAnswer" });
export type PreviewAnswer = typeof PreviewAnswer.Type;

export const PrivacyBuildEffect = Schema.Union([
  Schema.Literal("documented"),
  Schema.Literal("uncertain"),
  Schema.Literal("no_current_effect"),
]).annotate({ identifier: "PrivacyBuildEffect" });
export type PrivacyBuildEffect = typeof PrivacyBuildEffect.Type;

export const PrivacyCategory = Schema.Literals(["diagnostics", "speech_input", "location_sensors", "search", "content", "activity_sync", "cloud_services", "app_permissions", "windows_ai"]).annotate({ identifier: "PrivacyCategory" });
export type PrivacyCategory = typeof PrivacyCategory.Type;

export const PrivacyControl = Schema.Literals(["powershell_telemetry_optout", "ceip_consolidator_task", "ceip_usb_task", "windows_ceip", "diagnostic_data", "feedback_notifications", "steps_recorder", "error_reporting", "error_reporting_additional_data", "diagnostic_log_collection", "dump_collection", "application_telemetry", "device_census_task", "device_census_user_task", "dotnet_cli_telemetry_optout", "voice_activation", "voice_activation_above_lock", "online_speech_recognition", "implicit_text_collection", "implicit_ink_collection", "handwriting_error_reports", "input_personalization", "linguistic_data_collection", "handwriting_data_sharing", "location_provider", "location_feature", "sensors", "search_web", "search_web_results", "cloud_search", "cortana", "cortana_above_lock", "search_location", "search_highlights", "tailored_experiences", "spotlight", "consumer_features", "cloud_optimized_content", "windows_tips", "consumer_account_state_content", "third_party_suggestions", "settings_online_tips", "widgets", "advertising_id", "language_list_websites", "start_launch_tracking", "activity_feed", "activity_publish", "activity_upload", "clipboard_history", "cloud_clipboard", "continue_experiences", "phone_pc_linking", "message_sync", "onesettings_downloads", "find_my_device", "delivery_optimization", "onedrive_file_storage", "onedrive_pre_signin_traffic", "push_notifications_network", "app_location", "app_account_info", "app_motion", "app_phone_calls", "app_trusted_devices", "app_unpaired_devices", "app_diagnostic_info", "app_contacts", "app_calendar", "app_call_history", "app_email", "app_tasks", "app_messaging", "app_radios", "app_camera", "app_microphone", "app_notifications", "app_background", "app_screen_capture_programmatic", "app_screen_capture_borderless", "app_gaze_input", "app_human_presence", "app_foreground_text", "camera_device", "app_system_ai_models", "recall_snapshots", "recall_component", "click_to_do"]).annotate({ identifier: "PrivacyControl" });
export type PrivacyControl = typeof PrivacyControl.Type;

export const PrivacyValue = Schema.Union([
  Schema.Struct({
    value: Schema.Literal("absent"),
  }),
  Schema.Struct({
    number: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
    value: Schema.Literal("dword"),
  }),
  Schema.Struct({
    text: Schema.String,
    value: Schema.Literal("text"),
  }),
  Schema.Struct({
    hex: Schema.String,
    kind: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
    value: Schema.Literal("raw"),
  }),
  Schema.Struct({
    value: Schema.Literal("task_absent"),
  }),
  Schema.Struct({
    definition_sha256: Schema.String,
    value: Schema.Literal("task_enabled"),
  }),
  Schema.Struct({
    definition_sha256: Schema.String,
    value: Schema.Literal("task_disabled"),
  }),
]).annotate({ identifier: "PrivacyValue" });
export type PrivacyValue = typeof PrivacyValue.Type;

export const PrivacyCurrent = Schema.Union([
  Schema.Struct({
    read: Schema.Struct({
      value: PrivacyValue,
    }),
  }),
  Schema.Literal("access_denied"),
  Schema.Struct({
    failed: Schema.Struct({
      error: AxError,
    }),
  }),
  Schema.Literal("not_read"),
]).annotate({ identifier: "PrivacyCurrent" });
export type PrivacyCurrent = typeof PrivacyCurrent.Type;

export const PrivacyEditionFit = Schema.Union([
  Schema.Literal("honoured"),
  Schema.Literal("ignored"),
  Schema.Literal("not_stated"),
]).annotate({ identifier: "PrivacyEditionFit" });
export type PrivacyEditionFit = typeof PrivacyEditionFit.Type;

export const PrivacyEdition = Schema.Literals(["home", "pro", "enterprise", "education", "iot_enterprise", "server"]).annotate({ identifier: "PrivacyEdition" });
export type PrivacyEdition = typeof PrivacyEdition.Type;

export const PrivacyEditions = Schema.Struct({
  honoured: Schema.Array(PrivacyEdition),
  ignored: Schema.Array(PrivacyEdition),
}).annotate({ identifier: "PrivacyEditions" });
export type PrivacyEditions = typeof PrivacyEditions.Type;

export const PrivacyOriginal = Schema.Literals(["k01", "k02", "k03", "k04", "k05", "k06", "k07", "k08", "k09", "k10", "k11", "k12", "k13", "k14", "k15", "k16", "k17", "k18", "k19", "k20", "k21", "k22", "k23", "k24", "k25", "k26", "k27", "k28", "k29", "k30", "k31", "k32", "k33", "k34", "k35", "k36", "k37", "k38", "k39", "k40", "k41", "k42", "k43", "k44", "k45", "k46", "k47", "k48", "k49", "k50", "k51", "k52"]).annotate({ identifier: "PrivacyOriginal" });
export type PrivacyOriginal = typeof PrivacyOriginal.Type;

export const PrivacyOriginalLine = Schema.Struct({
  item: PrivacyOriginal,
  text: Schema.String,
}).annotate({ identifier: "PrivacyOriginalLine" });
export type PrivacyOriginalLine = typeof PrivacyOriginalLine.Type;

export const PrivacyScope = Schema.Literals(["user", "machine"]).annotate({ identifier: "PrivacyScope" });
export type PrivacyScope = typeof PrivacyScope.Type;

export const PrivacyTarget = Schema.Union([
  Schema.Struct({
    kind: Schema.Literal("registry_value_hklm"),
    name: Schema.String,
    path: Schema.String,
  }),
  Schema.Struct({
    kind: Schema.Literal("registry_value_hkcu"),
    name: Schema.String,
    path: Schema.String,
  }),
  Schema.Struct({
    kind: Schema.Literal("environment_variable_user"),
    name: Schema.String,
  }),
  Schema.Struct({
    kind: Schema.Literal("scheduled_task_enabled"),
    name: Schema.String,
    path: Schema.String,
  }),
]).annotate({ identifier: "PrivacyTarget" });
export type PrivacyTarget = typeof PrivacyTarget.Type;

export const PrivacyControlEntry = Schema.Struct({
  build_effect: PrivacyBuildEffect,
  category: PrivacyCategory,
  control: PrivacyControl,
  current: PrivacyCurrent,
  editions: PrivacyEditions,
  host_fit: PrivacyEditionFit,
  originals: Schema.Array(PrivacyOriginalLine),
  scope: PrivacyScope,
  target: PrivacyTarget,
  written: Schema.optional(Schema.NullOr(PrivacyValue)),
}).annotate({ identifier: "PrivacyControlEntry" });
export type PrivacyControlEntry = typeof PrivacyControlEntry.Type;

export const PrivacyIntent = Schema.Struct({
  control: PrivacyControl,
  modified: PrivacyValue,
  operation: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  original: PrivacyValue,
  restore_of: Schema.optional(Schema.NullOr(Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)))),
}).annotate({ identifier: "PrivacyIntent" });
export type PrivacyIntent = typeof PrivacyIntent.Type;

export const PrivacyHistory = Schema.Union([
  Schema.Struct({
    disclosed: Schema.Struct({
      owned: Schema.Array(PrivacyIntent),
      unresolved: Schema.optional(Schema.NullOr(PrivacyIntent)),
    }),
  }),
  Schema.Struct({
    withheld: Schema.Struct({
      error: AxError,
    }),
  }),
  Schema.Struct({
    unreadable: Schema.Struct({
      error: AxError,
    }),
  }),
]).annotate({ identifier: "PrivacyHistory" });
export type PrivacyHistory = typeof PrivacyHistory.Type;

export const PrivacyWindows = Schema.Struct({
  build: Schema.String,
  display_version: Schema.optional(Schema.NullOr(Schema.String)),
  edition: Schema.optional(Schema.NullOr(PrivacyEdition)),
  edition_id: Schema.String,
}).annotate({ identifier: "PrivacyWindows" });
export type PrivacyWindows = typeof PrivacyWindows.Type;

export const PrivacyHost = Schema.Union([
  Schema.Struct({
    windows: PrivacyWindows,
  }),
  Schema.Struct({
    unreadable: Schema.Struct({
      error: AxError,
    }),
  }),
  Schema.Literal("not_windows"),
]).annotate({ identifier: "PrivacyHost" });
export type PrivacyHost = typeof PrivacyHost.Type;

export const PrivacyNotWritten = Schema.Union([
  Schema.Literal("absent"),
  Schema.Literal("obsolete"),
  Schema.Literal("undeterminable"),
  Schema.Literal("needs_operation_kind"),
]).annotate({ identifier: "PrivacyNotWritten" });
export type PrivacyNotWritten = typeof PrivacyNotWritten.Type;

export const PrivacyNotWrittenEntry = Schema.Struct({
  alternatives: Schema.Array(PrivacyControl),
  line: PrivacyOriginalLine,
  reason: PrivacyNotWritten,
}).annotate({ identifier: "PrivacyNotWrittenEntry" });
export type PrivacyNotWrittenEntry = typeof PrivacyNotWrittenEntry.Type;

export const PrivacyAction = Schema.Union([
  Schema.Struct({
    apply: Schema.Struct({
      control: PrivacyControl,
      expected: PrivacyValue,
    }),
  }),
  Schema.Struct({
    restore: Schema.Struct({
      control: PrivacyControl,
      expected: PrivacyValue,
    }),
  }),
  Schema.Struct({
    reconcile: Schema.Struct({
      expected: PrivacyValue,
    }),
  }),
]).annotate({ identifier: "PrivacyAction" });
export type PrivacyAction = typeof PrivacyAction.Type;

export const PrivacyFaultCode = Schema.Literals(["identity", "clock", "history", "unreadable", "unresolved", "changed", "target_absent", "nothing_owned", "conflict", "nothing_unresolved", "history_full", "expired", "access_denied", "elevation_declined", "not_applied", "readback_mismatch", "unknown", "receipt_lost"]).annotate({ identifier: "PrivacyFaultCode" });
export type PrivacyFaultCode = typeof PrivacyFaultCode.Type;

export const PrivacySettlement = Schema.Union([
  Schema.Literal("applied"),
  Schema.Literal("not_applied"),
  Schema.Literal("restored"),
  Schema.Literal("abandoned"),
]).annotate({ identifier: "PrivacySettlement" });
export type PrivacySettlement = typeof PrivacySettlement.Type;

export const PrivacyResult = Schema.Union([
  Schema.Literal("running"),
  Schema.Struct({
    applied: Schema.Struct({
      operation: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
    }),
  }),
  Schema.Struct({
    restored: Schema.Struct({
      operation: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
    }),
  }),
  Schema.Literal("already_written"),
  Schema.Struct({
    reconciled: Schema.Struct({
      operation: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
      settlement: PrivacySettlement,
    }),
  }),
  Schema.Struct({
    refused: Schema.Struct({
      code: PrivacyFaultCode,
      error: AxError,
    }),
  }),
]).annotate({ identifier: "PrivacyResult" });
export type PrivacyResult = typeof PrivacyResult.Type;

export const PrivacyOutcome = Schema.Struct({
  action: PrivacyAction,
  idem: IdemKey,
  result: PrivacyResult,
}).annotate({ identifier: "PrivacyOutcome" });
export type PrivacyOutcome = typeof PrivacyOutcome.Type;

export const PrivacyAnswer = Schema.Struct({
  controls: Schema.Array(PrivacyControlEntry),
  history: PrivacyHistory,
  host: PrivacyHost,
  not_written: Schema.Array(PrivacyNotWrittenEntry),
  outcomes: Schema.Array(PrivacyOutcome),
}).annotate({ identifier: "PrivacyAnswer" });
export type PrivacyAnswer = typeof PrivacyAnswer.Type;

export const ShapePart = Schema.Struct({
  bytes: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  hash: Schema.String,
}).annotate({ identifier: "ShapePart" });
export type ShapePart = typeof ShapePart.Type;

export const PromptShapeAnswer = Schema.Struct({
  conversation: ShapePart,
  run: RunId,
  tools: ShapePart,
  upper_bound: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
}).annotate({ identifier: "PromptShapeAnswer" });
export type PromptShapeAnswer = typeof PromptShapeAnswer.Type;

export const SliceKind = Schema.Union([
  Schema.Literal("same"),
  Schema.Literal("delete"),
  Schema.Literal("insert"),
]).annotate({ identifier: "SliceKind" });
export type SliceKind = typeof SliceKind.Type;

export const Slice = Schema.Struct({
  kind: SliceKind,
  lead: Schema.String,
  text: Schema.String,
  trail: Schema.String,
}).annotate({ identifier: "Slice" });
export type Slice = typeof Slice.Type;

export const ProposalCard = Schema.Struct({
  baseline: B3Hash,
  id: B3Hash,
  offered: Schema.optional(Schema.NullOr(Seq)),
  run: RunId,
  slices: Schema.Array(Slice),
  span: Span,
}).annotate({ identifier: "ProposalCard" });
export type ProposalCard = typeof ProposalCard.Type;

export const ProposalsAnswer = Schema.Struct({
  doc: Address,
  open: Schema.Array(ProposalCard),
  version: Schema.optional(Schema.NullOr(B3Hash)),
}).annotate({ identifier: "ProposalsAnswer" });
export type ProposalsAnswer = typeof ProposalsAnswer.Type;

export const RangeAnswer = Schema.Struct({
  version: B3Hash,
  window: Window2,
}).annotate({ identifier: "RangeAnswer" });
export type RangeAnswer = typeof RangeAnswer.Type;

export const RegistryLine = Schema.Struct({
  addr: Address,
  at: TimeMs,
  kind: Schema.String,
  subject: Schema.String,
}).annotate({ identifier: "RegistryLine" });
export type RegistryLine = typeof RegistryLine.Type;

export const RegistryAnswer = Schema.Struct({
  assets: Schema.Array(RegistryLine),
}).annotate({ identifier: "RegistryAnswer" });
export type RegistryAnswer = typeof RegistryAnswer.Type;

export const Registry = Schema.Literals(["npm", "crates_io", "github"]).annotate({ identifier: "Registry" });
export type Registry = typeof Registry.Type;

export const ReleaseLine = Schema.Struct({
  released: Schema.String,
  version: Schema.String,
}).annotate({ identifier: "ReleaseLine" });
export type ReleaseLine = typeof ReleaseLine.Type;

export const RegistryReading = Schema.Union([
  Schema.Struct({
    read: Schema.Struct({
      newest: ReleaseLine,
    }),
  }),
  Schema.Struct({
    refused: Schema.Struct({
      refusal: AxError,
    }),
  }),
]).annotate({ identifier: "RegistryReading" });
export type RegistryReading = typeof RegistryReading.Type;

export const RegistryNewest = Schema.Struct({
  reading: RegistryReading,
  registry: Registry,
}).annotate({ identifier: "RegistryNewest" });
export type RegistryNewest = typeof RegistryNewest.Type;

export const ReleaseVerdict = Schema.Union([
  Schema.Literal("current"),
  Schema.Literal("behind"),
  Schema.Literal("ahead"),
]).annotate({ identifier: "ReleaseVerdict" });
export type ReleaseVerdict = typeof ReleaseVerdict.Type;

export const InstallChannel = Schema.Literals(["npm", "bun", "binstall", "unknown", "package", "cargo_or_binstall", "cargo", "archive", "aur", "source"]).annotate({ identifier: "InstallChannel" });
export type InstallChannel = typeof InstallChannel.Type;

export const UpdateHint = Schema.Struct({
  alternatives: Schema.Array(Schema.String),
  channel: InstallChannel,
  command: Schema.optional(Schema.NullOr(Schema.String)),
}).annotate({ identifier: "UpdateHint" });
export type UpdateHint = typeof UpdateHint.Type;

export const ReleaseAnswer = Schema.Union([
  Schema.Struct({
    stands: Schema.Struct({
      mine: ReleaseLine,
      newest: ReleaseLine,
      registries: Schema.Array(RegistryNewest),
      update: UpdateHint,
      verdict: ReleaseVerdict,
    }),
  }),
  Schema.Struct({
    unreleased: Schema.Struct({
      registries: Schema.Array(RegistryNewest),
      update: UpdateHint,
    }),
  }),
  Schema.Struct({
    unconfirmed: Schema.Struct({
      mine: ReleaseLine,
      registries: Schema.Array(RegistryNewest),
      update: UpdateHint,
    }),
  }),
  Schema.Struct({
    refused: Schema.Struct({
      refusal: AxError,
    }),
  }),
]).annotate({ identifier: "ReleaseAnswer" });
export type ReleaseAnswer = typeof ReleaseAnswer.Type;

export const Closing = Schema.Struct({
  at: TimeMs,
  completion: Schema.String,
  failure: Schema.optional(Schema.NullOr(AxError)),
}).annotate({ identifier: "Closing" });
export type Closing = typeof Closing.Type;

export const FrozenGeneration = Schema.Struct({
  max_tokens: Schema.optional(Schema.NullOr(Ceiling)),
  overrides: Schema.optional(Schema.Array(Schema.Tuple([Schema.String, Schema.String]))),
  settings: Schema.optional(GenerationSettings),
}).annotate({ identifier: "FrozenGeneration" });
export type FrozenGeneration = typeof FrozenGeneration.Type;

export const FrozenNames = Schema.Struct({
  mayor: Schema.optional(Schema.NullOr(Schema.String)),
}).annotate({ identifier: "FrozenNames" });
export type FrozenNames = typeof FrozenNames.Type;

export const LandingPolicy = Schema.Union([
  Schema.Literal("ordinary"),
  Schema.Literal("experiment"),
]).annotate({ identifier: "LandingPolicy" });
export type LandingPolicy = typeof LandingPolicy.Type;

export const Mode = Schema.Union([
  Schema.Literal("chat"),
  Schema.Literal("work"),
]).annotate({ identifier: "Mode" });
export type Mode = typeof Mode.Type;

export const WriteLimit = Schema.Union([
  Schema.Literal("full"),
  Schema.Literal("create"),
]).annotate({ identifier: "WriteLimit" });
export type WriteLimit = typeof WriteLimit.Type;

export const RunPolicy = Schema.Struct({
  admit: AdmissionRequirement,
  landing: LandingPolicy,
  mode: Mode,
  write: WriteLimit,
}).annotate({ identifier: "RunPolicy" });
export type RunPolicy = typeof RunPolicy.Type;

export const Opening = Schema.Struct({
  at: TimeMs,
  dispatched_by: Schema.optional(Schema.NullOr(Schema.String)),
  effort: Schema.optional(Schema.NullOr(Effort)),
  generation: Schema.optional(Schema.NullOr(FrozenGeneration)),
  goal: Schema.String,
  names: Schema.optional(Schema.NullOr(FrozenNames)),
  parent: Schema.optional(Schema.NullOr(RunId)),
  policy: Schema.optional(Schema.NullOr(RunPolicy)),
  task: Schema.String,
}).annotate({ identifier: "Opening" });
export type Opening = typeof Opening.Type;

export const Effect = Schema.Union([
  Schema.Literal("read"),
  Schema.Struct({
    write: Schema.Struct({
      domain: Address,
    }),
  }),
  Schema.Literal("egress"),
  Schema.Struct({
    connector: Schema.Struct({
      label: Schema.String,
    }),
  }),
  Schema.Literal("spawn"),
  Schema.Literal("govern"),
  Schema.Struct({
    attach_user_browser: Schema.Struct({
      address: Schema.optional(Schema.NullOr(Schema.String)),
    }),
  }),
  Schema.Literal("spend"),
]).annotate({ identifier: "Effect" });
export type Effect = typeof Effect.Type;

export const Landing = Schema.Union([
  Schema.Literal("delivered"),
  Schema.Literal("queued"),
  Schema.Literal("knocked"),
]).annotate({ identifier: "Landing" });
export type Landing = typeof Landing.Type;

export const Outcome = Schema.Union([
  Schema.Literal("waiting"),
  Schema.Literal("answered"),
  Schema.Literal("failed"),
]).annotate({ identifier: "Outcome" });
export type Outcome = typeof Outcome.Type;

export const Output = Schema.Struct({
  cut: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  head: Schema.String,
  pinned: Schema.optional(Schema.NullOr(Schema.String)),
}).annotate({ identifier: "Output" });
export type Output = typeof Output.Type;

export const RenderIntent = Schema.Union([
  Schema.Literals(["generic", "terminal"]),
  Schema.Struct({
    diff: Schema.Struct({
      locations: Schema.Array(Address),
    }),
  }),
  Schema.Literal("signal"),
  Schema.Literal("delegate"),
]).annotate({ identifier: "RenderIntent" });
export type RenderIntent = typeof RenderIntent.Type;

export const Timing = Schema.Union([
  Schema.Literal("measured"),
  Schema.Literal("unmeasured"),
]).annotate({ identifier: "Timing" });
export type Timing = typeof Timing.Type;

export const Call = Schema.Struct({
  answered: Schema.optional(Schema.NullOr(TimeMs)),
  arguments: Schema.optional(Schema.NullOr(Output)),
  at: Seq,
  called: TimeMs,
  effect: Schema.optional(Schema.NullOr(Effect)),
  exit_code: Schema.optional(Schema.NullOr(Schema.Int)),
  landing: Schema.optional(Schema.NullOr(Landing)),
  outcome: Outcome,
  output: Schema.optional(Schema.NullOr(Output)),
  render: Schema.optional(Schema.NullOr(RenderIntent)),
  subject: Schema.optional(Schema.NullOr(Schema.String)),
  timing: Timing,
  took_us: Schema.optional(Schema.NullOr(Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)))),
  tool: Schema.String,
}).annotate({ identifier: "Call" });
export type Call = typeof Call.Type;

export const HandbackNote = Schema.Union([
  Schema.Struct({
    finished: Schema.Struct({
      verified_by: Schema.String,
    }),
  }),
  Schema.Struct({
    stopped: Schema.Struct({
      because: Schema.String,
    }),
  }),
]).annotate({ identifier: "HandbackNote" });
export type HandbackNote = typeof HandbackNote.Type;

export const ReplyEnd = Schema.Literals(["reply", "timeout", "left"]).annotate({ identifier: "ReplyEnd" });
export type ReplyEnd = typeof ReplyEnd.Type;

export const ReplyEnded = Schema.Struct({
  by: ReplyEnd,
  t: TimeMs,
}).annotate({ identifier: "ReplyEnded" });
export type ReplyEnded = typeof ReplyEnded.Type;

export const Speaker = Schema.Literals(["user", "city", "resident"]).annotate({ identifier: "Speaker" });
export type Speaker = typeof Speaker.Type;

export const Note = Schema.Union([
  Schema.Struct({
    refused: Schema.Struct({
      at: Seq,
      error: AxError,
    }),
  }),
  Schema.Struct({
    checkpointed: Schema.Struct({
      at: Seq,
      oid: GitOid,
    }),
  }),
  Schema.Struct({
    waiting: Schema.Struct({
      answered: Schema.optional(Schema.NullOr(TimeMs)),
      at: Seq,
      t: TimeMs,
    }),
  }),
  Schema.Struct({
    arrived: Schema.Struct({
      at: Seq,
      by: Speaker,
      from: Schema.optional(Schema.NullOr(Schema.String)),
      handback: Schema.optional(Schema.NullOr(HandbackNote)),
      kind: Schema.optional(Schema.NullOr(Schema.String)),
      said: Schema.optional(Schema.NullOr(Schema.String)),
      session: Schema.optional(Schema.NullOr(RunId)),
      t: TimeMs,
    }),
  }),
  Schema.Struct({
    awaiting_reply: Schema.Struct({
      at: Seq,
      ended: Schema.optional(Schema.NullOr(ReplyEnded)),
      on: Address,
      t: TimeMs,
      until: TimeMs,
    }),
  }),
  Schema.Struct({
    discarded: Schema.Struct({
      at: Seq,
      count: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
    }),
  }),
  Schema.Struct({
    unreadable: Schema.Struct({
      at: Seq,
      cause: Schema.String,
    }),
  }),
]).annotate({ identifier: "Note" });
export type Note = typeof Note.Type;

export const Used = Schema.Struct({
  cache_write: Schema.optional(Schema.NullOr(Tokens)),
  cached: Schema.optional(Schema.NullOr(Tokens)),
  first_us: Schema.optional(Schema.NullOr(Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)))),
  input: Tokens,
  output: Tokens,
  took_us: Schema.optional(Schema.NullOr(Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)))),
}).annotate({ identifier: "Used" });
export type Used = typeof Used.Type;

export const Turn = Schema.Struct({
  calls: Schema.Array(Call),
  first_at: Schema.optional(Schema.NullOr(TimeMs)),
  model: Schema.optional(Schema.NullOr(Schema.String)),
  notes: Schema.Array(Note),
  number: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  opened: Seq,
  returned: Schema.optional(Schema.NullOr(TimeMs)),
  said: Schema.optional(Schema.NullOr(Schema.String)),
  spent: Schema.optional(Schema.NullOr(UsdMicros)),
  stopped: Schema.optional(Schema.NullOr(Schema.String)),
  t: TimeMs,
  thought: Schema.optional(Schema.NullOr(Schema.String)),
  timing: Timing,
  used: Schema.optional(Schema.NullOr(Used)),
}).annotate({ identifier: "Turn" });
export type Turn = typeof Turn.Type;

export const RoundsAnswer = Schema.Struct({
  closing: Schema.optional(Schema.NullOr(Closing)),
  opened_at: Schema.optional(Schema.NullOr(GitOid)),
  opening: Schema.optional(Schema.NullOr(Opening)),
  run: RunId,
  turns: Schema.Array(Turn),
  worktree: Schema.optional(Schema.NullOr(Schema.String)),
}).annotate({ identifier: "RoundsAnswer" });
export type RoundsAnswer = typeof RoundsAnswer.Type;

export const RunCostsAnswer = Schema.Struct({
  asked: Schema.Array(RunId),
  runs: Schema.Array(Schema.Tuple([RunId, UsdMicros])),
}).annotate({ identifier: "RunCostsAnswer" });
export type RunCostsAnswer = typeof RunCostsAnswer.Type;

export const Origin = Schema.Struct({
  at_seq: Seq,
  run: RunId,
}).annotate({ identifier: "Origin" });
export type Origin = typeof Origin.Type;

export const SessionRef = Schema.Struct({
  began: Seq,
  room: Address,
}).annotate({ identifier: "SessionRef" });
export type SessionRef = typeof SessionRef.Type;

export const Carry = Schema.Union([
  Schema.Literal("nothing"),
  Schema.Literal("handoff"),
]).annotate({ identifier: "Carry" });
export type Carry = typeof Carry.Type;

export const SessionStart = Schema.Union([
  Schema.Struct({
    dispatched: Schema.Struct({
      by: Schema.optional(Schema.NullOr(Schema.String)),
    }),
  }),
  Schema.Struct({
    opened: Schema.Struct({
      carry: Carry,
      from: Schema.optional(Schema.NullOr(Origin)),
    }),
  }),
]).annotate({ identifier: "SessionStart" });
export type SessionStart = typeof SessionStart.Type;

export const SessionLine = Schema.Struct({
  at: TimeMs,
  began: Seq,
  branched_from: Schema.optional(Schema.NullOr(Origin)),
  effort: Schema.optional(Schema.NullOr(Effort)),
  last: Seq,
  model: Schema.optional(Schema.NullOr(Schema.String)),
  name: Schema.optional(Schema.NullOr(Schema.String)),
  parent: Schema.optional(Schema.NullOr(SessionRef)),
  preview: Schema.optional(Schema.NullOr(Schema.String)),
  runs: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  start: SessionStart,
  tail: Schema.optional(Schema.NullOr(Origin)),
  workspace: Schema.optional(Schema.NullOr(Schema.String)),
}).annotate({ identifier: "SessionLine" });
export type SessionLine = typeof SessionLine.Type;

export const SessionsAnswer = Schema.Struct({
  earlier: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  room: Address,
  sessions: Schema.Array(SessionLine),
}).annotate({ identifier: "SessionsAnswer" });
export type SessionsAnswer = typeof SessionsAnswer.Type;

export const ShellCalls = Schema.Struct({
  calls: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  failures: Schema.Record(Schema.String, Schema.Int.check(Schema.isGreaterThanOrEqualTo(0))),
  interpreter: Schema.String,
}).annotate({ identifier: "ShellCalls" });
export type ShellCalls = typeof ShellCalls.Type;

export const ShellsAnswer = Schema.Struct({
  interpreters: Schema.Array(ShellCalls),
}).annotate({ identifier: "ShellsAnswer" });
export type ShellsAnswer = typeof ShellsAnswer.Type;

export const SkillAudit = Schema.Union([
  Schema.Struct({
    state: Schema.Literal("unaudited"),
  }),
  Schema.Struct({
    at: Seq,
    state: Schema.Literal("audited"),
    verdict: AuditVerdict,
  }),
  Schema.Struct({
    audited: B3Hash,
    state: Schema.Literal("stale"),
  }),
]).annotate({ identifier: "SkillAudit" });
export type SkillAudit = typeof SkillAudit.Type;

export const SkillShelf = Schema.Union([
  Schema.Literal("library"),
  Schema.Struct({
    building: Address,
  }),
  Schema.Struct({
    external: Schema.Struct({
      index: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
      path: Schema.String,
    }),
  }),
]).annotate({ identifier: "SkillShelf" });
export type SkillShelf = typeof SkillShelf.Type;

export const HeldSkill = Schema.Struct({
  audit: SkillAudit,
  digest: B3Hash,
  shelf: SkillShelf,
}).annotate({ identifier: "HeldSkill" });
export type HeldSkill = typeof HeldSkill.Type;

export const SkillUse = Schema.Struct({
  at: TimeMs,
  digest: B3Hash,
  outcome: UseOutcome,
  part: Schema.String,
  resident: Schema.optional(Schema.NullOr(Address)),
  run: RunId,
  seq: Seq,
}).annotate({ identifier: "SkillUse" });
export type SkillUse = typeof SkillUse.Type;

export const ShelvedFrom = Schema.Union([
  Schema.Struct({
    kind: Schema.Literal("path"),
  }),
  Schema.Struct({
    kind: Schema.Literal("git"),
    rev: Schema.optional(Schema.NullOr(Schema.String)),
    url: Schema.String,
  }),
  Schema.Struct({
    kind: Schema.Literal("skills_sh"),
    name: Schema.String,
  }),
  Schema.Struct({
    kind: Schema.Literal("shipped"),
  }),
  Schema.Struct({
    kind: Schema.Literal("page"),
  }),
]).annotate({ identifier: "ShelvedFrom" });
export type ShelvedFrom = typeof ShelvedFrom.Type;

export const VersionAuthor = Schema.Union([
  Schema.Struct({
    by: Schema.Literal("shelved"),
    from: ShelvedFrom,
    seq: Seq,
  }),
  Schema.Struct({
    by: Schema.Literal("outside_shelf"),
  }),
  Schema.Struct({
    by: Schema.Literal("unrecorded"),
  }),
]).annotate({ identifier: "VersionAuthor" });
export type VersionAuthor = typeof VersionAuthor.Type;

export const SkillVersion = Schema.Struct({
  at: TimeMs,
  author: Schema.optional(VersionAuthor),
  digest: B3Hash,
  run: RunId,
  seq: Seq,
}).annotate({ identifier: "SkillVersion" });
export type SkillVersion = typeof SkillVersion.Type;

export const SkillUsageLine = Schema.Struct({
  held: Schema.Array(HeldSkill),
  name: Schema.String,
  per_day: Schema.Array(DayCount),
  uses: Schema.Array(SkillUse),
  versions: Schema.Array(SkillVersion),
}).annotate({ identifier: "SkillUsageLine" });
export type SkillUsageLine = typeof SkillUsageLine.Type;

export const SkillUsageAnswer = Schema.Struct({
  skills: Schema.Array(SkillUsageLine),
}).annotate({ identifier: "SkillUsageAnswer" });
export type SkillUsageAnswer = typeof SkillUsageAnswer.Type;

export const SkillLine = Schema.Struct({
  admitted: Schema.Boolean,
  disclosure: Schema.String,
  hash: B3Hash,
  name: Schema.String,
  pinned_by: Schema.Array(RunId),
  section: Schema.String,
  shelf: SkillShelf,
}).annotate({ identifier: "SkillLine" });
export type SkillLine = typeof SkillLine.Type;

export const SkillsAnswer = Schema.Struct({
  building: Address,
  external_shelves: Schema.optional(Schema.Array(Schema.String)),
  missing: Schema.Array(Schema.String),
  skills: Schema.Array(SkillLine),
}).annotate({ identifier: "SkillsAnswer" });
export type SkillsAnswer = typeof SkillsAnswer.Type;

export const TemplateName = Schema.String.pipe(Schema.brand("TemplateName"));
export type TemplateName = typeof TemplateName.Type;

export const TemplateAnswer = Schema.Struct({
  addr: Address,
  rules: Schema.String,
  template: TemplateName,
}).annotate({ identifier: "TemplateAnswer" });
export type TemplateAnswer = typeof TemplateAnswer.Type;

export const Standing = Schema.Union([
  Schema.Literal("absent"),
  Schema.Struct({
    awaiting: Schema.Struct({
      consent_url: Schema.String,
    }),
  }),
  Schema.Struct({
    connected: Schema.Struct({
      alias: Schema.String,
    }),
  }),
  Schema.Struct({
    refused: Schema.Struct({
      refusal: AxError,
    }),
  }),
]).annotate({ identifier: "Standing" });
export type Standing = typeof Standing.Type;

export const ToolkitSlug = Schema.String.pipe(Schema.brand("ToolkitSlug"));
export type ToolkitSlug = typeof ToolkitSlug.Type;

export const ToolkitLine = Schema.Struct({
  auth: Schema.String,
  name: Schema.String,
  slug: ToolkitSlug,
  standing: Standing,
}).annotate({ identifier: "ToolkitLine" });
export type ToolkitLine = typeof ToolkitLine.Type;

export const ToolkitsAnswer = Schema.Union([
  Schema.Literal("unenrolled"),
  Schema.Struct({
    shelf: Schema.Struct({
      toolkits: Schema.Array(ToolkitLine),
    }),
  }),
  Schema.Struct({
    refused: Schema.Struct({
      refusal: AxError,
    }),
  }),
]).annotate({ identifier: "ToolkitsAnswer" });
export type ToolkitsAnswer = typeof ToolkitsAnswer.Type;

export const ExportFormat = Schema.Literals(["jsonl", "csv"]).annotate({ identifier: "ExportFormat" });
export type ExportFormat = typeof ExportFormat.Type;

export const UsageKind = Schema.Literals(["skills", "mcp"]).annotate({ identifier: "UsageKind" });
export type UsageKind = typeof UsageKind.Type;

export const UsageExportAnswer = Schema.Struct({
  body: Schema.String,
  format: ExportFormat,
  what: UsageKind,
}).annotate({ identifier: "UsageExportAnswer" });
export type UsageExportAnswer = typeof UsageExportAnswer.Type;

export const VersionSource = Schema.Union([
  Schema.Literal("on_disk"),
  Schema.Struct({
    saved: Schema.Struct({
      at: TimeMs,
      seq: Seq,
    }),
  }),
  Schema.Struct({
    before: Schema.Struct({
      seq: Seq,
    }),
  }),
  Schema.Literal("shipped"),
]).annotate({ identifier: "VersionSource" });
export type VersionSource = typeof VersionSource.Type;

export const DocumentVersion = Schema.Struct({
  bytes: Schema.optional(Schema.NullOr(Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)))),
  kept: Schema.Boolean,
  source: VersionSource,
  version: B3Hash,
}).annotate({ identifier: "DocumentVersion" });
export type DocumentVersion = typeof DocumentVersion.Type;

export const VersionsAnswer = Schema.Struct({
  at: Address,
  more: Schema.Boolean,
  versions: Schema.Array(DocumentVersion),
}).annotate({ identifier: "VersionsAnswer" });
export type VersionsAnswer = typeof VersionsAnswer.Type;

export const Answer = Schema.Union([
  Schema.Struct({
    history: HistoryAnswer,
  }),
  Schema.Struct({
    history_range: HistoryRangeAnswer,
  }),
  Schema.Struct({
    sessions: SessionsAnswer,
  }),
  Schema.Struct({
    changes: ChangesAnswer,
  }),
  Schema.Struct({
    commit: CommitAnswer,
  }),
  Schema.Struct({
    city: CityAnswer,
  }),
  Schema.Struct({
    run: Schema.NullOr(RunSummary),
  }),
  Schema.Struct({
    approvals: ApprovalsAnswer,
  }),
  Schema.Struct({
    cost: CostAnswer,
  }),
  Schema.Struct({
    endpoints: EndpointsAnswer,
  }),
  Schema.Struct({
    known_hosts: KnownHostsAnswer,
  }),
  Schema.Struct({
    agent_catalog: AgentCatalogAnswer,
  }),
  Schema.Struct({
    agent_spec: AgentOffer,
  }),
  Schema.Struct({
    devices: DevicesAnswer,
  }),
  Schema.Struct({
    building: BuildingAnswer,
  }),
  Schema.Struct({
    inbox: InboxAnswer,
  }),
  Schema.Struct({
    discards: DiscardAnswer,
  }),
  Schema.Struct({
    registry: RegistryAnswer,
  }),
  Schema.Struct({
    archive: ArchiveAnswer,
  }),
  Schema.Struct({
    metrics: MetricsAnswer,
  }),
  Schema.Struct({
    governance: GovernanceAnswer,
  }),
  Schema.Struct({
    hunks: HunksAnswer,
  }),
  Schema.Struct({
    rounds: RoundsAnswer,
  }),
  Schema.Struct({
    evidence: EvidenceAnswer,
  }),
  Schema.Struct({
    cost_of: CostOfAnswer,
  }),
  Schema.Struct({
    run_costs: RunCostsAnswer,
  }),
  Schema.Struct({
    listing: ListingAnswer,
  }),
  Schema.Struct({
    find: FindAnswer,
  }),
  Schema.Struct({
    document: DocumentAnswer,
  }),
  Schema.Struct({
    proposals: ProposalsAnswer,
  }),
  Schema.Struct({
    open_proposals: OpenProposalsAnswer,
  }),
  Schema.Struct({
    range: RangeAnswer,
  }),
  Schema.Struct({
    versions: VersionsAnswer,
  }),
  Schema.Struct({
    bytes: BytesAnswer,
  }),
  Schema.Struct({
    export: ExportAnswer,
  }),
  Schema.Struct({
    preview: PreviewAnswer,
  }),
  Schema.Struct({
    reply: Laid,
  }),
  Schema.Struct({
    commits: CommitsAnswer,
  }),
  Schema.Struct({
    doctor: DoctorAnswer,
  }),
  Schema.Struct({
    upstream: DoctorUpstream,
  }),
  Schema.Struct({
    prefix: PrefixAnswer,
  }),
  Schema.Struct({
    prompt_shape: PromptShapeAnswer,
  }),
  Schema.Struct({
    content: ContentAnswer,
  }),
  Schema.Struct({
    skills: SkillsAnswer,
  }),
  Schema.Struct({
    template: TemplateAnswer,
  }),
  Schema.Struct({
    layers: LayersAnswer,
  }),
  Schema.Struct({
    catalog: CatalogAnswer,
  }),
  Schema.Struct({
    git_status: GitStatusAnswer,
  }),
  Schema.Struct({
    mcp_health: McpHealthAnswer,
  }),
  Schema.Struct({
    skill_usage: SkillUsageAnswer,
  }),
  Schema.Struct({
    mcp_usage: McpUsageAnswer,
  }),
  Schema.Struct({
    usage_export: UsageExportAnswer,
  }),
  Schema.Struct({
    shells: ShellsAnswer,
  }),
  Schema.Struct({
    toolkits: ToolkitsAnswer,
  }),
  Schema.Struct({
    release: ReleaseAnswer,
  }),
  Schema.Struct({
    preferences: PreferencesAnswer,
  }),
  Schema.Struct({
    config: ConfigAnswer,
  }),
  Schema.Struct({
    identity: IdentityAnswer,
  }),
  Schema.Struct({
    automation: AutomationAnswer,
  }),
  Schema.Struct({
    github_login: GithubLoginAnswer,
  }),
  Schema.Struct({
    guide: GuideProgress,
  }),
  Schema.Struct({
    privacy: PrivacyAnswer,
  }),
  Schema.Struct({
    model_figures: ModelFiguresAnswer,
  }),
  Schema.Struct({
    unavailable: Schema.Struct({
      query: Schema.String,
      reason: Schema.optional(Schema.NullOr(Schema.String)),
    }),
  }),
]).annotate({ identifier: "Answer" });
export type Answer = typeof Answer.Type;

export const AskId = Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)).pipe(Schema.brand("AskId"));
export type AskId = typeof AskId.Type;

export const AskOutcome = Schema.Union([
  Schema.Struct({
    answer: Answer,
  }),
  Schema.Struct({
    refusal: AxError,
  }),
]).annotate({ identifier: "AskOutcome" });
export type AskOutcome = typeof AskOutcome.Type;

export const Answered = Schema.Struct({
  as_of: Seq,
  ask_id: AskId,
  outcome: AskOutcome,
}).annotate({ identifier: "Answered" });
export type Answered = typeof Answered.Type;

export const FiguresAsked = Schema.Struct({
  context_tokens: Schema.optional(Schema.NullOr(Window)),
  id: Schema.String,
  max_output_tokens: Schema.optional(Schema.NullOr(Ceiling)),
  tag: Schema.optional(Schema.NullOr(ModelTag)),
}).annotate({ identifier: "FiguresAsked" });
export type FiguresAsked = typeof FiguresAsked.Type;

export const ProviderName = Schema.String.pipe(Schema.brand("ProviderName"));
export type ProviderName = typeof ProviderName.Type;

export const ReplyState = Schema.Union([
  Schema.Literal("streaming"),
  Schema.Literal("settled"),
]).annotate({ identifier: "ReplyState" });
export type ReplyState = typeof ReplyState.Type;

export const Query = Schema.Union([
  Schema.Literals(["city_view", "approval_queue", "metrics", "cost_view", "registry_view", "discard_view"]),
  Schema.Struct({
    history: Schema.Struct({
      before: Schema.optional(Schema.NullOr(Seq)),
      limit: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
    }),
  }),
  Schema.Struct({
    run_history: Schema.Struct({
      before: Schema.optional(Schema.NullOr(Seq)),
      limit: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
      run: RunId,
    }),
  }),
  Schema.Struct({
    history_range: Schema.Struct({
      from: Seq,
      limit: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
      to: Seq,
    }),
  }),
  Schema.Struct({
    sessions: Schema.Struct({
      room: Address,
    }),
  }),
  Schema.Struct({
    changes: Schema.Struct({
      base: GitOid,
      head: Schema.optional(Schema.NullOr(GitOid)),
    }),
  }),
  Schema.Struct({
    hunks: Schema.Struct({
      oid_a: GitOid,
      oid_b: GitOid,
      path: Schema.String,
    }),
  }),
  Schema.Struct({
    commit: Schema.Struct({
      oid: GitOid,
    }),
  }),
  Schema.Struct({
    run_view: Schema.Struct({
      run: RunId,
    }),
  }),
  Schema.Struct({
    inbox_view: Schema.Struct({
      addr: Address,
    }),
  }),
  Schema.Struct({
    archive_search: Schema.Struct({
      needle: Schema.String,
    }),
  }),
  Schema.Literal("endpoint_view"),
  Schema.Literal("known_hosts"),
  Schema.Literal("agent_catalog"),
  Schema.Struct({
    parse_agent_spec: Schema.Struct({
      text: Schema.String,
    }),
  }),
  Schema.Literal("devices"),
  Schema.Struct({
    building_view: Schema.Struct({
      addr: Address,
    }),
  }),
  Schema.Literal("identity"),
  Schema.Literal("automation"),
  Schema.Struct({
    github_login: Schema.NullOr(Schema.String),
  }),
  Schema.Literal("guide"),
  Schema.Literal("governance"),
  Schema.Struct({
    rounds: Schema.Struct({
      run: RunId,
    }),
  }),
  Schema.Struct({
    evidence: Schema.Struct({
      run: RunId,
    }),
  }),
  Schema.Struct({
    cost_of: Schema.Struct({
      node: NodeId,
    }),
  }),
  Schema.Struct({
    listing: Schema.Struct({
      at: Schema.optional(Schema.NullOr(Address)),
    }),
  }),
  Schema.Struct({
    find: Schema.Struct({
      text: Schema.String,
      under: Address,
    }),
  }),
  Schema.Struct({
    document: Schema.Struct({
      at: Address,
    }),
  }),
  Schema.Struct({
    proposals: Address,
  }),
  Schema.Literal("open_proposals"),
  Schema.Struct({
    range: Schema.Struct({
      range: Span,
      version: B3Hash,
    }),
  }),
  Schema.Struct({
    versions: Schema.Struct({
      at: Address,
    }),
  }),
  Schema.Struct({
    bytes: Schema.Struct({
      range: Span,
      version: B3Hash,
    }),
  }),
  Schema.Struct({
    export: Schema.Struct({
      at: Address,
      version: B3Hash,
    }),
  }),
  Schema.Struct({
    preview: Schema.Struct({
      version: B3Hash,
      viewport: Span,
    }),
  }),
  Schema.Struct({
    reply: Schema.Struct({
      state: ReplyState,
      text: Schema.String,
    }),
  }),
  Schema.Struct({
    commits: Schema.Struct({
      before: Schema.optional(Schema.NullOr(Seq)),
      building: Schema.optional(Schema.NullOr(Address)),
      limit: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
    }),
  }),
  Schema.Literal("doctor"),
  Schema.Struct({
    prefix: Schema.Struct({
      run: RunId,
    }),
  }),
  Schema.Struct({
    prompt_shape: Schema.Struct({
      run: RunId,
    }),
  }),
  Schema.Struct({
    content: Schema.Struct({
      locator: Locator,
    }),
  }),
  Schema.Struct({
    skills: Schema.Struct({
      building: Address,
    }),
  }),
  Schema.Struct({
    template: Schema.Struct({
      addr: Address,
      template: TemplateName,
    }),
  }),
  Schema.Struct({
    git_status: Schema.Struct({
      building: Address,
    }),
  }),
  Schema.Struct({
    mcp_health: Schema.Struct({
      addr: Address,
    }),
  }),
  Schema.Literal("toolkits"),
  Schema.Literal("newest_release"),
  Schema.Literal("privacy"),
  Schema.Literal("preferences"),
  Schema.Struct({
    config: Schema.Struct({
      addr: Address,
    }),
  }),
  Schema.Struct({
    run_costs: Schema.Struct({
      runs: Schema.Array(RunId),
    }),
  }),
  Schema.Struct({
    upstream_version: Schema.Struct({
      item: Schema.String,
    }),
  }),
  Schema.Struct({
    skill_usage: Schema.Struct({
      skill: Schema.optional(Schema.NullOr(Schema.String)),
    }),
  }),
  Schema.Struct({
    mcp_usage: Schema.Struct({
      server: Schema.optional(Schema.NullOr(Schema.String)),
    }),
  }),
  Schema.Struct({
    usage_export: Schema.Struct({
      format: ExportFormat,
      what: UsageKind,
    }),
  }),
  Schema.Literal("shells"),
  Schema.Struct({
    model_figures: Schema.Struct({
      base_url: Schema.String,
      endpoint: ProviderName,
      face: DialectKind,
      models: Schema.Array(FiguresAsked),
    }),
  }),
  Schema.Struct({
    layers: Schema.Struct({
      addr: Address,
    }),
  }),
  Schema.Struct({
    catalog: Schema.Struct({
      building: Address,
    }),
  }),
]).annotate({ identifier: "Query" });
export type Query = typeof Query.Type;

export const Ask = Schema.Struct({
  ask_id: AskId,
  query: Query,
}).annotate({ identifier: "Ask" });
export type Ask = typeof Ask.Type;

export const BeatMs = Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)).pipe(Schema.brand("BeatMs"));
export type BeatMs = typeof BeatMs.Type;

export const BuildingSettings = Schema.Struct({
  addr: Address,
  catalog: Schema.optional(Schema.NullOr(ExposureTable)),
  context_second_threshold: Schema.optional(Schema.NullOr(Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)))),
  desktop: Schema.optional(Schema.NullOr(Schema.String)),
  idem: IdemKey,
  mcp: Schema.optional(Schema.NullOr(Schema.Array(McpServer))),
  mcp_references: Schema.optional(Schema.NullOr(Schema.Array(McpReference))),
  reading_room: Schema.optional(Schema.NullOr(Schema.Array(Schema.String))),
  sandbox: Schema.optional(Schema.NullOr(SandboxLimits)),
  skill_pins: Schema.optional(Schema.NullOr(Schema.Record(Schema.String, B3Hash))),
}).annotate({ identifier: "BuildingSettings" });
export type BuildingSettings = typeof BuildingSettings.Type;

export const CloseMode = Schema.Union([
  Schema.Literal("drain"),
  Schema.Literal("interrupt"),
]).annotate({ identifier: "CloseMode" });
export type CloseMode = typeof CloseMode.Type;

export const CityClosing = Schema.Struct({
  idem: IdemKey,
  mode: CloseMode,
}).annotate({ identifier: "CityClosing" });
export type CityClosing = typeof CityClosing.Type;

export const KeepWarm = Schema.Union([
  Schema.Literal("off"),
  Schema.Literal("five_minute"),
]).annotate({ identifier: "KeepWarm" });
export type KeepWarm = typeof KeepWarm.Type;

export const CitySettings = Schema.Struct({
  catalog: Schema.optional(Schema.NullOr(ExposureTable)),
  effort: Schema.optional(Schema.NullOr(Effort)),
  idem: IdemKey,
  keep_warm: Schema.optional(Schema.NullOr(KeepWarm)),
  search: Schema.optional(Schema.NullOr(SearchConfiguration)),
  skill_shelves: Schema.optional(Schema.NullOr(Schema.Array(Schema.String))),
}).annotate({ identifier: "CitySettings" });
export type CitySettings = typeof CitySettings.Type;

export const DeviceForgetting = Schema.Struct({
  device: DeviceId,
  idem: IdemKey,
}).annotate({ identifier: "DeviceForgetting" });
export type DeviceForgetting = typeof DeviceForgetting.Type;

export const DoorAnswer = Schema.Struct({
  code: Schema.String,
  idem: IdemKey,
}).annotate({ identifier: "DoorAnswer" });
export type DoorAnswer = typeof DoorAnswer.Type;

export const DoorOpening = Schema.Struct({
  idem: IdemKey,
  lasting_ms: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
}).annotate({ identifier: "DoorOpening" });
export type DoorOpening = typeof DoorOpening.Type;

export const DoorStep = Schema.Struct({
  idem: IdemKey,
}).annotate({ identifier: "DoorStep" });
export type DoorStep = typeof DoorStep.Type;

export const IdentityCard = Schema.Union([
  Schema.Struct({
    person: Schema.Struct({
      about: Schema.optional(Schema.NullOr(Schema.String)),
      imported_from: Schema.optional(Schema.NullOr(Schema.String)),
      user_id: Schema.optional(Schema.NullOr(Schema.String)),
    }),
  }),
  Schema.Struct({
    mayor: Schema.Struct({
      name: Schema.optional(Schema.NullOr(Schema.String)),
    }),
  }),
]).annotate({ identifier: "IdentityCard" });
export type IdentityCard = typeof IdentityCard.Type;

export const NoSecret = Schema.Never.annotate({ identifier: "NoSecret" });
export type NoSecret = typeof NoSecret.Type;

export const PolicyChange = Schema.Struct({
  idem: IdemKey,
  policy: RunPolicy,
  room: Address,
}).annotate({ identifier: "PolicyChange" });
export type PolicyChange = typeof PolicyChange.Type;

export const PreferencePatch = Schema.Union([
  Schema.Struct({
    lang: Lang,
  }),
  Schema.Struct({
    welcomed: Schema.Boolean,
  }),
  Schema.Struct({
    panel: Schema.Boolean,
  }),
  Schema.Struct({
    tier: Tier,
  }),
  Schema.Struct({
    appearance: Appearance,
  }),
  Schema.Struct({
    proxying: Proxying,
  }),
  Schema.Struct({
    chord: Chord,
  }),
  Schema.Struct({
    core_priority: CorePriority,
  }),
  Schema.Struct({
    core_placement: CorePlacement,
  }),
  Schema.Struct({
    run_memory: Schema.NullOr(Schema.Int.check(Schema.isGreaterThanOrEqualTo(1))),
  }),
  Schema.Struct({
    tags: SessionTags,
  }),
  Schema.Struct({
    theme: ThemeOverride,
  }),
]).annotate({ identifier: "PreferencePatch" });
export type PreferencePatch = typeof PreferencePatch.Type;

export const PrivacyRequest = Schema.Struct({
  action: PrivacyAction,
  idem: IdemKey,
}).annotate({ identifier: "PrivacyRequest" });
export type PrivacyRequest = typeof PrivacyRequest.Type;

export const PromptStep = Schema.Union([
  Schema.Literal("own"),
  Schema.Literal("inherit"),
  Schema.Literal("restore"),
]).annotate({ identifier: "PromptStep" });
export type PromptStep = typeof PromptStep.Type;

export const PromptWrite = Schema.Struct({
  at: Address,
  base: B3Hash,
  idem: IdemKey,
  step: PromptStep,
}).annotate({ identifier: "PromptWrite" });
export type PromptWrite = typeof PromptWrite.Type;

export const Verdict = Schema.Union([
  Schema.Literal("accept"),
  Schema.Struct({
    amend: Schema.Struct({
      text: Schema.String,
    }),
  }),
]).annotate({ identifier: "Verdict" });
export type Verdict = typeof Verdict.Type;

export const SliceVerdict = Schema.Struct({
  slice: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  verdict: Verdict,
}).annotate({ identifier: "SliceVerdict" });
export type SliceVerdict = typeof SliceVerdict.Type;

export const ProposalDecision = Schema.Struct({
  proposal: B3Hash,
  verdicts: Schema.Array(SliceVerdict),
}).annotate({ identifier: "ProposalDecision" });
export type ProposalDecision = typeof ProposalDecision.Type;

export const ProposalDecisions = Schema.Struct({
  decisions: Schema.Array(ProposalDecision),
  doc: Address,
  idem: IdemKey,
}).annotate({ identifier: "ProposalDecisions" });
export type ProposalDecisions = typeof ProposalDecisions.Type;

export const PursuitStep = Schema.Union([
  Schema.Literals(["pause", "resume", "clear"]),
  Schema.Struct({
    set: Schema.Struct({
      goal: Schema.String,
    }),
  }),
]).annotate({ identifier: "PursuitStep" });
export type PursuitStep = typeof PursuitStep.Type;

export const TextEdit = Schema.Struct({
  span: Span,
  text: Schema.String,
}).annotate({ identifier: "TextEdit" });
export type TextEdit = typeof TextEdit.Type;

export const RangeWrite = Schema.Struct({
  baseline: B3Hash,
  doc: Address,
  edits: Schema.Array(TextEdit),
  idem: IdemKey,
}).annotate({ identifier: "RangeWrite" });
export type RangeWrite = typeof RangeWrite.Type;

export const RulesWrite = Schema.Struct({
  base: Schema.String,
  body: Schema.String,
  building: Address,
  idem: IdemKey,
}).annotate({ identifier: "RulesWrite" });
export type RulesWrite = typeof RulesWrite.Type;

export const SecretForgetting = Schema.Struct({
  idem: IdemKey,
  reference: Schema.String,
}).annotate({ identifier: "SecretForgetting" });
export type SecretForgetting = typeof SecretForgetting.Type;

export const SessionNaming = Schema.Struct({
  began: Seq,
  idem: IdemKey,
  name: Schema.String,
  room: Address,
}).annotate({ identifier: "SessionNaming" });
export type SessionNaming = typeof SessionNaming.Type;

export const Shelf = Schema.Union([
  Schema.Literal("library"),
  Schema.Struct({
    building: Address,
  }),
]).annotate({ identifier: "Shelf" });
export type Shelf = typeof Shelf.Type;

export const SpineDocument = Schema.Union([
  Schema.Literal("roadmap"),
  Schema.Literal("memo"),
  Schema.Literal("handoff"),
  Schema.Literal("spec"),
]).annotate({ identifier: "SpineDocument" });
export type SpineDocument = typeof SpineDocument.Type;

export const WorkspaceChoiceWrite = Schema.Struct({
  base: Schema.String,
  building: Address,
  effort: Effort,
  endpoint: Schema.String,
  idem: IdemKey,
  model: Schema.String,
}).annotate({ identifier: "WorkspaceChoiceWrite" });
export type WorkspaceChoiceWrite = typeof WorkspaceChoiceWrite.Type;

export const Command = Schema.Union([
  Schema.Struct({
    dispatch: Schema.Struct({
      addr: Address,
      effort: Schema.optional(Schema.NullOr(Effort)),
      goal: Schema.String,
      idem: IdemKey,
      model: Schema.optional(Schema.NullOr(Schema.String)),
      policy: RunPolicy,
      session: Schema.optional(Schema.NullOr(SessionName)),
      task: Schema.String,
    }),
  }),
  Schema.Struct({
    probe_endpoint: Schema.Struct({
      auth_header: Schema.optional(Schema.NullOr(Schema.String)),
      base_url: Schema.String,
      dialect: DialectKind,
      idem: IdemKey,
      name: ProviderName,
      secret: Schema.optional(Schema.NullOr(Schema.String)),
      tuning: EndpointTuning,
    }),
  }),
  Schema.Struct({
    configure_building: BuildingSettings,
  }),
  Schema.Struct({
    attach_endpoint: Schema.Struct({
      admit: Schema.Array(Schema.String),
      auth_header: Schema.optional(Schema.NullOr(Schema.String)),
      base_url: Schema.String,
      dialect: DialectKind,
      idem: IdemKey,
      name: ProviderName,
      named: Schema.Array(Schema.String),
      secret: Schema.optional(Schema.NullOr(Schema.String)),
      tuning: EndpointTuning,
    }),
  }),
  Schema.Struct({
    select_model: Schema.Struct({
      context_tokens: Schema.optional(Schema.NullOr(Window)),
      endpoint: ProviderName,
      idem: IdemKey,
      input: Schema.optional(Schema.NullOr(InputKinds)),
      max_output_tokens: Schema.optional(Schema.NullOr(Ceiling)),
      model: Schema.String,
      tag: ModelTag,
    }),
  }),
  Schema.Struct({
    open_session: Schema.Struct({
      addr: Address,
      carry: Carry,
      from: Schema.optional(Schema.NullOr(Origin)),
      idem: IdemKey,
    }),
  }),
  Schema.Struct({
    create_building: Schema.Struct({
      addr: Address,
      idem: IdemKey,
      rules: Schema.optional(Schema.NullOr(Schema.String)),
      source: Schema.optional(Schema.NullOr(Schema.String)),
      template: TemplateName,
    }),
  }),
  Schema.Struct({
    remove_building: Schema.Struct({
      addr: Address,
      idem: IdemKey,
    }),
  }),
  Schema.Struct({
    put_secret: Schema.Struct({
      name: Schema.String,
      realm: Schema.String,
      value: NoSecret,
    }),
  }),
  Schema.Struct({
    steer: Schema.Struct({
      idem: IdemKey,
      run: RunId,
      text: Schema.String,
    }),
  }),
  Schema.Struct({
    cancel: Schema.Struct({
      idem: IdemKey,
      run: RunId,
    }),
  }),
  Schema.Struct({
    halt: Schema.Struct({
      idem: IdemKey,
      scope: HaltScope,
    }),
  }),
  Schema.Struct({
    reveal: Schema.Struct({
      at: Address,
      idem: IdemKey,
    }),
  }),
  Schema.Struct({
    restore_discard: Schema.Struct({
      idem: IdemKey,
      restoration: Restoration,
    }),
  }),
  Schema.Struct({
    doctor_install: Schema.Struct({
      idem: IdemKey,
      item: Schema.String,
    }),
  }),
  Schema.Struct({
    doctor_refresh: Schema.Struct({
      idem: IdemKey,
    }),
  }),
  Schema.Struct({
    release: Schema.Struct({
      idem: IdemKey,
      scope: HaltScope,
    }),
  }),
  Schema.Struct({
    batch_by_building: Schema.Struct({
      addr: Address,
      idem: IdemKey,
    }),
  }),
  Schema.Struct({
    approve: Schema.Struct({
      idem: IdemKey,
      item: ApprovalId,
      verdict: Ruling,
    }),
  }),
  Schema.Struct({
    hand_off: Schema.Struct({
      idem: IdemKey,
      item: ApprovalId,
      to: ResidentId,
    }),
  }),
  Schema.Struct({
    set_autonomy: Schema.Struct({
      autonomy: Autonomy,
      idem: IdemKey,
      scope: HaltScope,
    }),
  }),
  Schema.Struct({
    pursue: Schema.Struct({
      addr: Address,
      idem: IdemKey,
      step: PursuitStep,
    }),
  }),
  Schema.Struct({
    wake: Schema.Struct({
      body: Schema.String,
      idem: IdemKey,
      source: Schema.String,
      subject: Schema.String,
    }),
  }),
  Schema.Struct({
    put_document: Schema.Struct({
      base: Schema.String,
      body: Schema.String,
      idem: IdemKey,
      which: GovernedDocument,
    }),
  }),
  Schema.Struct({
    put_identity: Schema.Struct({
      base: Schema.String,
      card: IdentityCard,
      idem: IdemKey,
    }),
  }),
  Schema.Struct({
    put_rules: RulesWrite,
  }),
  Schema.Struct({
    set_workspace_choice: WorkspaceChoiceWrite,
  }),
  Schema.Struct({
    restore_file: Schema.Struct({
      at: Address,
      idem: IdemKey,
      point: GitOid,
    }),
  }),
  Schema.Struct({
    put_guide: Schema.Struct({
      idem: IdemKey,
      progress: GuideProgress,
    }),
  }),
  Schema.Struct({
    configure_city: CitySettings,
  }),
  Schema.Struct({
    put_spine: Schema.Struct({
      base: Schema.String,
      body: Schema.String,
      building: Address,
      idem: IdemKey,
      which: SpineDocument,
    }),
  }),
  Schema.Struct({
    put_range: RangeWrite,
  }),
  Schema.Struct({
    decide_proposals: ProposalDecisions,
  }),
  Schema.Struct({
    connect_toolkit: Schema.Struct({
      idem: IdemKey,
      toolkit: ToolkitSlug,
    }),
  }),
  Schema.Struct({
    put_preferences: Schema.Struct({
      idem: IdemKey,
      patch: PreferencePatch,
    }),
  }),
  Schema.Struct({
    put_shelved: Schema.Struct({
      idem: IdemKey,
      name: Schema.String,
      shelf: Shelf,
      text: Schema.String,
    }),
  }),
  Schema.Struct({
    name_session: SessionNaming,
  }),
  Schema.Struct({
    change_run_policy: PolicyChange,
  }),
  Schema.Struct({
    open_remote_door: DoorOpening,
  }),
  Schema.Struct({
    replace_city_key: DoorStep,
  }),
  Schema.Struct({
    confirm_remote_door: DoorAnswer,
  }),
  Schema.Struct({
    close_remote_door: DoorStep,
  }),
  Schema.Struct({
    privacy_operation: PrivacyRequest,
  }),
  Schema.Struct({
    forget_secret: SecretForgetting,
  }),
  Schema.Struct({
    close_city: CityClosing,
  }),
  Schema.Struct({
    add_agent: AgentAdding,
  }),
  Schema.Struct({
    agent_login: AgentLoginStart,
  }),
  Schema.Struct({
    forget_device: DeviceForgetting,
  }),
  Schema.Struct({
    put_prompt: PromptWrite,
  }),
  Schema.Struct({
    auth: Schema.Struct({
      token: Schema.String,
    }),
  }),
]).annotate({ identifier: "Command" });
export type Command = typeof Command.Type;

export const Hello = Schema.Struct({
  schema: B3Hash,
  token: Schema.optional(Schema.NullOr(Schema.String)),
  wire_v: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
}).annotate({ identifier: "Hello" });
export type Hello = typeof Hello.Type;

export const Monitoring = Schema.Union([
  Schema.Literal("release"),
  Schema.Literal("watch"),
  Schema.Literal("watch_summary"),
  Schema.Struct({
    beat: BeatMs,
  }),
]).annotate({ identifier: "Monitoring" });
export type Monitoring = typeof Monitoring.Type;

export const ClientFrame = Schema.Union([
  Schema.Struct({
    hello: Hello,
  }),
  Schema.Struct({
    command: Command,
  }),
  Schema.Struct({
    ask: Ask,
  }),
  Schema.Struct({
    monitor: Monitoring,
  }),
]).annotate({ identifier: "ClientFrame" });
export type ClientFrame = typeof ClientFrame.Type;

export const Increment = Schema.Union([
  Schema.Struct({
    said: Schema.String,
  }),
  Schema.Struct({
    thought: Schema.String,
  }),
]).annotate({ identifier: "Increment" });
export type Increment = typeof Increment.Type;

export const Delta = Schema.Struct({
  increment: Increment,
  run: RunId,
}).annotate({ identifier: "Delta" });
export type Delta = typeof Delta.Type;

export const Lagged = Schema.Struct({
  from: Seq,
  to: Seq,
}).annotate({ identifier: "Lagged" });
export type Lagged = typeof Lagged.Type;

export const OutputStream = Schema.Literals(["out", "err"]).annotate({ identifier: "OutputStream" });
export type OutputStream = typeof OutputStream.Type;

export const LiveOutput = Schema.Struct({
  run: RunId,
  stream: OutputStream,
  text: Schema.String,
}).annotate({ identifier: "LiveOutput" });
export type LiveOutput = typeof LiveOutput.Type;

export const LogLevel = Schema.Literals(["refuse", "effect", "decide", "trace", "wire"]).annotate({ identifier: "LogLevel" });
export type LogLevel = typeof LogLevel.Type;

export const LogLine = Schema.Struct({
  level: LogLevel,
  line: Schema.String,
  module: Schema.String,
  run: Schema.optional(Schema.NullOr(RunId)),
  seq: Seq,
  t: Schema.optional(Schema.NullOr(TimeMs)),
}).annotate({ identifier: "LogLine" });
export type LogLine = typeof LogLine.Type;

export const Sample = Schema.Struct({
  beat_ms: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  core_cpu_permille: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  core_private_bytes: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  core_read_bytes: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  core_working_set_bytes: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  core_written_bytes: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  durable_lag: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  event_to_screen_p50_nanos: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  ledger_queue_depth: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  machine_available_bytes: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  machine_cpu_permille: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  queued_runs: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  read_nanos: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  relay_p50_nanos: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  view_backlog: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  volume_free_bytes: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
}).annotate({ identifier: "Sample" });
export type Sample = typeof Sample.Type;

export const Welcome = Schema.Struct({
  city: Schema.optional(Schema.NullOr(Address)),
  epoch: Schema.optional(Schema.NullOr(B3Hash)),
  resume_from: Schema.optional(Schema.NullOr(Seq)),
  schema: B3Hash,
  wire_v: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
}).annotate({ identifier: "Welcome" });
export type Welcome = typeof Welcome.Type;

export const ServerFrame = Schema.Union([
  Schema.Struct({
    welcome: Welcome,
  }),
  Schema.Struct({
    event: EventRecord,
  }),
  Schema.Struct({
    answered: Answered,
  }),
  Schema.Struct({
    refusal: AxError,
  }),
  Schema.Struct({
    delta: Delta,
  }),
  Schema.Struct({
    log: LogLine,
  }),
  Schema.Struct({
    lagged: Lagged,
  }),
  Schema.Struct({
    output: LiveOutput,
  }),
  Schema.Struct({
    monitor: Sample,
  }),
]).annotate({ identifier: "ServerFrame" });
export type ServerFrame = typeof ServerFrame.Type;

