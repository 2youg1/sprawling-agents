// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Schema } from "effect";

import {
  Address,
  B3Hash,
  IdemKey,
  Locator,
  McpServer,
  NodeId,
  SandboxLimits,
  Span,
} from "./c00";

import {
  DeviceId,
  Effort,
  ExposureTable,
  GitOid,
  McpReference,
  Proxying,
  RunId,
  SearchConfiguration,
  Seq,
} from "./c01";

import {
  DialectKind,
} from "./c02";

import {
  Appearance,
  Chord,
  CorePlacement,
  CorePriority,
  Lang,
  PrivacyAction,
  SessionTags,
  ThemeOverride,
  Tier,
} from "./c05";

import {
  RunPolicy,
} from "./c06";

import {
  ExportFormat,
  TemplateName,
  UsageKind,
} from "./c07";

import {
  AskId,
  FiguresAsked,
  ProviderName,
  ReplyState,
} from "./c08";

export const Query = Schema.Union([
  Schema.Literals(["city_view", "approval_queue", "metrics", "cost_view", "registry_view", "discard_view", "endpoint_view", "known_hosts", "agent_catalog", "devices", "identity", "automation", "guide", "governance", "open_proposals", "doctor", "toolkits", "newest_release", "privacy", "preferences", "shells"]),
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
  Schema.Struct({
    parse_agent_spec: Schema.Struct({
      text: Schema.String,
    }),
  }),
  Schema.Struct({
    building_view: Schema.Struct({
      addr: Address,
    }),
  }),
  Schema.Struct({
    github_login: Schema.NullOr(Schema.String),
  }),
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

export const CloseMode = Schema.Literals(["drain", "interrupt"]).annotate({ identifier: "CloseMode" });
export type CloseMode = typeof CloseMode.Type;

export const CityClosing = Schema.Struct({
  idem: IdemKey,
  mode: CloseMode,
}).annotate({ identifier: "CityClosing" });
export type CityClosing = typeof CityClosing.Type;

export const KeepWarm = Schema.Literals(["off", "five_minute"]).annotate({ identifier: "KeepWarm" });
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

export const PromptStep = Schema.Literals(["own", "inherit", "restore"]).annotate({ identifier: "PromptStep" });
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

