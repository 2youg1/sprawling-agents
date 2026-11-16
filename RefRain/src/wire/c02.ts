// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Schema } from "effect";

import {
  Address,
  B3Hash,
  SandboxArm,
  Span,
} from "./c00";

import {
  DiscardLine,
} from "./c01";

export const DiscardAnswer = Schema.Struct({
  rows: Schema.Array(DiscardLine),
}).annotate({ identifier: "DiscardAnswer" });
export type DiscardAnswer = typeof DiscardAnswer.Type;

export const DoctorCore = Schema.Union([
  Schema.Literals(["raised", "held_by_setting", "lowered_by_valve"]),
  Schema.Struct({
    refused: Schema.Struct({
      said: Schema.String,
    }),
  }),
  Schema.Struct({
    unasked: Schema.Struct({
      said: Schema.String,
    }),
  }),
]).annotate({ identifier: "DoctorCore" });
export type DoctorCore = typeof DoctorCore.Type;

export const DoctorCustodyLifetime = Schema.Literals(["across_reboots", "with_passphrase", "until_reboot", "this_process"]).annotate({ identifier: "DoctorCustodyLifetime" });
export type DoctorCustodyLifetime = typeof DoctorCustodyLifetime.Type;

export const DoctorCustodyStore = Schema.Literals(["platform_service", "encrypted_file", "session_memory"]).annotate({ identifier: "DoctorCustodyStore" });
export type DoctorCustodyStore = typeof DoctorCustodyStore.Type;

export const DoctorCustody = Schema.Struct({
  keeps: DoctorCustodyLifetime,
  refusal: Schema.optional(Schema.NullOr(Schema.String)),
  store: DoctorCustodyStore,
}).annotate({ identifier: "DoctorCustody" });
export type DoctorCustody = typeof DoctorCustody.Type;

export const DoctorInstall = Schema.Union([
  Schema.Literal("unknown_platform"),
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
]).annotate({ identifier: "DoctorInstall" });
export type DoctorInstall = typeof DoctorInstall.Type;

export const DoctorNeed = Schema.Literals(["required", "optional"]).annotate({ identifier: "DoctorNeed" });
export type DoctorNeed = typeof DoctorNeed.Type;

export const DoctorPack = Schema.Literal("rust_tools").annotate({ identifier: "DoctorPack" });
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
  Schema.Literal("half_written"),
  Schema.Struct({
    will_not_start: Schema.Struct({
      said: Schema.String,
    }),
  }),
  Schema.Struct({
    unreadable: Schema.Struct({
      said: Schema.String,
    }),
  }),
]).annotate({ identifier: "DoctorFault" });
export type DoctorFault = typeof DoctorFault.Type;

export const DoctorVersion = Schema.Union([
  Schema.Literals(["silent", "unreadable", "late"]),
  Schema.Struct({
    said: Schema.Struct({
      text: Schema.String,
    }),
  }),
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

export const DoctorTier = Schema.Literals(["use", "develop"]).annotate({ identifier: "DoctorTier" });
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
  Schema.Literals(["linux_namespaces", "windows_job_object", "macos_seatbelt", "copied_tree"]),
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
  Schema.Literals(["no_disk", "admin_only"]),
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
  Schema.Literals(["does_not_apply", "stopped"]),
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

export const DoctorUnread = Schema.Literals(["with_toolchain", "many_brands", "matches_browser", "this_project", "no_source", "unknown_item"]).annotate({ identifier: "DoctorUnread" });
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

export const Format = Schema.Literals(["markdown", "plain"]).annotate({ identifier: "Format" });
export type Format = typeof Format.Type;

export const Coverage = Schema.Literals(["whole", "head"]).annotate({ identifier: "Coverage" });
export type Coverage = typeof Coverage.Type;

export const Encoding = Schema.Literals(["utf8", "utf8_bom", "utf16_le", "utf16_be"]).annotate({ identifier: "Encoding" });
export type Encoding = typeof Encoding.Type;

export const Window2 = Schema.Struct({
  span: Span,
  text: Schema.String,
}).annotate({ identifier: "Window2" });
export type Window2 = typeof Window2.Type;

export const DocumentBody = Schema.Union([
  Schema.Literal("opaque"),
  Schema.Struct({
    text: Schema.Struct({
      coverage: Coverage,
      encoding: Encoding,
      head: Window2,
    }),
  }),
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

export const ModelTag = Schema.Literals(["main", "digest", "transcribe", "ocr"]).annotate({ identifier: "ModelTag" });
export type ModelTag = typeof ModelTag.Type;

export const ChosenSummary = Schema.Struct({
  endpoint: Schema.String,
  max_output_tokens: Schema.optional(Schema.NullOr(Ceiling)),
  model: Schema.String,
  tag: ModelTag,
}).annotate({ identifier: "ChosenSummary" });
export type ChosenSummary = typeof ChosenSummary.Type;

export const DialectKind = Schema.Literals(["anthropic", "open_ai", "open_ai_responses"]).annotate({ identifier: "DialectKind" });
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

export const ParameterSupport = Schema.Literals(["unknown", "supported", "unsupported"]).annotate({ identifier: "ParameterSupport" });
export type ParameterSupport = typeof ParameterSupport.Type;

export const GenerationSupport = Schema.Struct({
  temperature: Schema.optional(Schema.NullOr(ParameterSupport)),
  top_k: Schema.optional(Schema.NullOr(ParameterSupport)),
  top_p: Schema.optional(Schema.NullOr(ParameterSupport)),
}).annotate({ identifier: "GenerationSupport" });
export type GenerationSupport = typeof GenerationSupport.Type;

