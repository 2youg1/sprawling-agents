// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Schema } from "effect";

import {
  Address,
  AdmissionRequirement,
  B3Hash,
  Span,
  TimeMs,
  Tokens,
  UsdMicros,
} from "./c00";

import {
  Effort,
  GitOid,
  RunId,
  Seq,
} from "./c01";

import {
  Ceiling,
  GenerationSettings,
  Window,
} from "./c02";

import {
  AxError,
} from "./c04";

import {
  ShapePart,
} from "./c05";

export const PromptShapeAnswer = Schema.Struct({
  conversation: ShapePart,
  run: RunId,
  tools: ShapePart,
  upper_bound: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
}).annotate({ identifier: "PromptShapeAnswer" });
export type PromptShapeAnswer = typeof PromptShapeAnswer.Type;

export const SliceKind = Schema.Literals(["same", "delete", "insert"]).annotate({ identifier: "SliceKind" });
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
  window: Window,
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

export const ReleaseVerdict = Schema.Literals(["current", "behind", "ahead"]).annotate({ identifier: "ReleaseVerdict" });
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

export const LandingPolicy = Schema.Literals(["ordinary", "experiment"]).annotate({ identifier: "LandingPolicy" });
export type LandingPolicy = typeof LandingPolicy.Type;

export const Mode = Schema.Literals(["chat", "work"]).annotate({ identifier: "Mode" });
export type Mode = typeof Mode.Type;

export const WriteLimit = Schema.Literals(["full", "create"]).annotate({ identifier: "WriteLimit" });
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
  Schema.Literals(["read", "egress", "spawn", "govern", "spend"]),
  Schema.Struct({
    write: Schema.Struct({
      domain: Address,
    }),
  }),
  Schema.Struct({
    connector: Schema.Struct({
      label: Schema.String,
    }),
  }),
  Schema.Struct({
    attach_user_browser: Schema.Struct({
      address: Schema.optional(Schema.NullOr(Schema.String)),
    }),
  }),
]).annotate({ identifier: "Effect" });
export type Effect = typeof Effect.Type;

export const Landing = Schema.Literals(["delivered", "queued", "knocked"]).annotate({ identifier: "Landing" });
export type Landing = typeof Landing.Type;

export const Outcome = Schema.Literals(["waiting", "answered", "failed"]).annotate({ identifier: "Outcome" });
export type Outcome = typeof Outcome.Type;

export const Output = Schema.Struct({
  cut: Schema.Int.check(Schema.isGreaterThanOrEqualTo(0)),
  head: Schema.String,
  pinned: Schema.optional(Schema.NullOr(Schema.String)),
}).annotate({ identifier: "Output" });
export type Output = typeof Output.Type;

export const RenderIntent = Schema.Union([
  Schema.Literals(["generic", "terminal", "signal", "delegate"]),
  Schema.Struct({
    diff: Schema.Struct({
      locations: Schema.Array(Address),
    }),
  }),
]).annotate({ identifier: "RenderIntent" });
export type RenderIntent = typeof RenderIntent.Type;

export const Timing = Schema.Literals(["measured", "unmeasured"]).annotate({ identifier: "Timing" });
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

export const Carry = Schema.Literals(["nothing", "handoff"]).annotate({ identifier: "Carry" });
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

