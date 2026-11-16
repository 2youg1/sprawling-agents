// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Schema } from "effect";

import {
  Address,
  AuditVerdict,
  B3Hash,
  TimeMs,
} from "./c00";

import {
  Effort,
  RunId,
  Seq,
} from "./c01";

import {
  AxError,
  DayCount,
  UseOutcome,
} from "./c04";

import {
  Carry,
  Origin,
  SessionRef,
} from "./c06";

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
  Schema.Literals(["on_disk", "shipped"]),
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

