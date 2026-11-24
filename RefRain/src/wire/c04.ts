// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Schema } from "effect";

import {
  AccountDisposition,
  Address,
  Align,
  B3Hash,
  IdemKey,
  Payload,
  ServerLabel,
  Span,
  TimeMs,
} from "./c00";

import {
  RunId,
  Seq,
} from "./c01";

import {
  Ceiling,
} from "./c02";

import {
  Construct,
  Inline,
  type InlineEncoded,
  Window,
} from "./c03";

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
  Schema.Literal("absent"),
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
  Schema.Literals(["city", "conventions", "identity"]),
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

export const Retry = Schema.Literals(["yes", "no", "unknown"]).annotate({ identifier: "Retry" });
export type Retry = typeof Retry.Type;

export const AxError = Schema.Struct({
  account: Schema.optional(AccountDisposition),
  action: Schema.String,
  code: AxCode,
  gate: Schema.optional(Schema.NullOr(GateRefusal)),
  nearby: Schema.Array(Schema.String),
  provider: Schema.optional(Schema.NullOr(ProviderFailureKind)),
  recovery: Schema.String,
  request: Schema.optional(Schema.NullOr(IdemKey)),
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

export const UseOutcome = Schema.Literals(["ok", "failed", "unknown"]).annotate({ identifier: "UseOutcome" });
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

export const FigureFrom = Schema.Literals(["person", "upstream", "registered", "preset", "policy"]).annotate({ identifier: "FigureFrom" });
export type FigureFrom = typeof FigureFrom.Type;

export const CeilingLine = Schema.Union([
  Schema.Literal("provider_default"),
  Schema.Struct({
    sent: Schema.Struct({
      from: FigureFrom,
      tokens: Ceiling,
    }),
  }),
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

export const Face = Schema.Literals(["paper", "maple", "system", "custom"]).annotate({ identifier: "Face" });
export type Face = typeof Face.Type;

export const Glass = Schema.Literals(["on", "off"]).annotate({ identifier: "Glass" });
export type Glass = typeof Glass.Type;

