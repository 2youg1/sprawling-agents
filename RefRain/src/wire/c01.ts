// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Schema } from "effect";

import {
  AccountRetries,
  AccountStatus,
  Address,
  B3Hash,
  BlockedLine,
  CatalogChange,
  CatalogSkill,
  Exposure,
  ExposureFrom,
  Locator,
  NodeId,
  Payload,
  Progress,
  ServerLabel,
  TimeMs,
  UsdMicros,
} from "./c00";

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

export const EventKind = Schema.Literals(["city_initialized", "building_created", "building_configured", "building_removed", "session_opened", "run_started", "run_forked", "prompt_assembled", "prompt_shape_compared", "model_called", "model_returned", "tool_called", "tool_result", "result_offloaded", "gate_checked", "gate_denied", "checkpoint_committed", "handoff_written", "steer_received", "cancel_received", "watchdog_fired", "budget_limit", "run_frozen", "log_truncated", "signal_enqueued", "signal_consumed", "draft_held", "draft_resolved", "goal_registered", "goal_conflict", "arbitration_verdict", "repair_started", "repair_reused", "worktree_opened", "pr_opened", "pr_merged", "pr_rejected", "roadmap_claimed", "roadmap_finished", "roadmap_released", "roadmap_split", "roadmap_blocked", "pursuit_changed", "approval_requested", "approval_resolved", "policy_created", "policy_revoked", "taint_promoted", "cross_building_transfer", "city_halted", "backpressure_shed", "digest_invalidated", "endpoint_attached", "endpoint_probed", "endpoint_lost", "model_selected", "provider_degraded", "login_started", "eval_run", "asset_archived", "credential_lent", "secret_captured", "secret_egress_blocked", "file_discarded", "discard_restored", "autonomy_changed", "went_back", "file_restored", "governed_document_written", "spine_document_written", "rules_changed", "toolkit_link_opened", "embedding_called", "rerank_called", "adviser_asked", "adviser_answered", "adviser_fell_back", "cache_renewed", "harness_reported", "harness_answered", "remote_opened", "remote_closed", "device_paired", "device_revoked", "remote_session_started", "document_written", "proposal_offered", "proposal_decided", "proposal_withdrawn", "run_policy_changed", "session_named", "skill_audited", "signal_wait_started", "signal_wait_ended", "signal_landed", "skill_shelved", "prefix_held"]).annotate({ identifier: "EventKind" });
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
  session_began: Schema.optional(Schema.NullOr(Seq)),
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

export const KeepWarm = Schema.Literals(["off", "five_minute"]).annotate({ identifier: "KeepWarm" });
export type KeepWarm = typeof KeepWarm.Type;

export const ConfigLayer = Schema.Literals(["default", "city", "building", "resident"]).annotate({ identifier: "ConfigLayer" });
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
  Schema.Literals(["default", "off"]),
  Schema.Struct({
    custom: Schema.Struct({
      selected: ServerLabel,
      suppliers: Schema.Array(SearchSupplier),
    }),
  }),
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

export const Proxying = Schema.Literals(["except_local", "always", "never"]).annotate({ identifier: "Proxying" });
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
  city_effort: Schema.optional(Schema.NullOr(Effort)),
  city_keep_warm: Schema.optional(Schema.NullOr(KeepWarm)),
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

