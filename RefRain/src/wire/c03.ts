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
  ClusterKey,
  Locator,
  Payload,
  TimeMs,
} from "./c00";

import {
  CommitAnswer,
  Effort,
  EventKind,
  FileChange,
  GitOid,
  ProviderAccount,
  Proxying,
  RunId,
  Seq,
} from "./c01";

import {
  BodyOverride,
  Ceiling,
  ChosenSummary,
  DialectKind,
  GenerationSettings,
  HeaderPair,
  ParameterSupport,
} from "./c02";

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

export const ParameterSource = Schema.Literals(["person", "upstream", "preset"]).annotate({ identifier: "ParameterSource" });
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

export const Listing = Schema.Literals(["listed", "named"]).annotate({ identifier: "Listing" });
export type Listing = typeof Listing.Type;

export const EffortWord = Schema.Struct({
  effort: Effort,
  word: Schema.String,
}).annotate({ identifier: "EffortWord" });
export type EffortWord = typeof EffortWord.Type;

export const OfferSource = Schema.Literals(["person", "upstream", "preset", "unknown"]).annotate({ identifier: "OfferSource" });
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

export const EvidenceKind = Schema.Literals(["screenshot", "finished"]).annotate({ identifier: "EvidenceKind" });
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

export const Walked = Schema.Literals(["whole", "cut"]).annotate({ identifier: "Walked" });
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
  Schema.Literals(["no_cli", "not_logged_in", "not_a_host"]),
  Schema.Struct({
    found: Schema.Struct({
      login: Schema.String,
    }),
  }),
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

export const GuideState = Schema.Literals(["open", "left"]).annotate({ identifier: "GuideState" });
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

export const GovernedDocument = Schema.Literals(["mayor", "clerk", "preferences"]).annotate({ identifier: "GovernedDocument" });
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

export const Construct = Schema.Literals(["html", "math", "front_matter", "nesting", "extension"]).annotate({ identifier: "Construct" });
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

