// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Schema } from "effect";

import {
  Address,
  B3Hash,
  Exposure,
  ExposureFrom,
  IdemKey,
} from "./c00";

import {
  Proxying,
  RunId,
  Seq,
} from "./c01";

import {
  Encoding,
} from "./c02";

import {
  AxError,
  Chroma,
  Density,
  Face,
  Glass,
  Laid,
  PrefixSlot,
} from "./c04";

export const Lighting = Schema.Literals(["system", "dark", "light"]).annotate({ identifier: "Lighting" });
export type Lighting = typeof Lighting.Type;

export const Motion = Schema.Literals(["system", "on", "off"]).annotate({ identifier: "Motion" });
export type Motion = typeof Motion.Type;

export const ReadingFace = Schema.Literals(["interface", "libron"]).annotate({ identifier: "ReadingFace" });
export type ReadingFace = typeof ReadingFace.Type;

export const TypeFeature = Schema.Literals(["coding_ligatures", "duospace", "narrow_space", "small_arrows", "single_story_a", "slashed_zero"]).annotate({ identifier: "TypeFeature" });
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

export const PrivacyBuildEffect = Schema.Literals(["documented", "uncertain", "no_current_effect"]).annotate({ identifier: "PrivacyBuildEffect" });
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
  Schema.Literals(["access_denied", "not_read"]),
  Schema.Struct({
    read: Schema.Struct({
      value: PrivacyValue,
    }),
  }),
  Schema.Struct({
    failed: Schema.Struct({
      error: AxError,
    }),
  }),
]).annotate({ identifier: "PrivacyCurrent" });
export type PrivacyCurrent = typeof PrivacyCurrent.Type;

export const PrivacyEditionFit = Schema.Literals(["honoured", "ignored", "not_stated"]).annotate({ identifier: "PrivacyEditionFit" });
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
  Schema.Literal("not_windows"),
  Schema.Struct({
    windows: PrivacyWindows,
  }),
  Schema.Struct({
    unreadable: Schema.Struct({
      error: AxError,
    }),
  }),
]).annotate({ identifier: "PrivacyHost" });
export type PrivacyHost = typeof PrivacyHost.Type;

export const PrivacyNotWritten = Schema.Literals(["absent", "obsolete", "undeterminable", "needs_operation_kind"]).annotate({ identifier: "PrivacyNotWritten" });
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

export const PrivacySettlement = Schema.Literals(["applied", "not_applied", "restored", "abandoned"]).annotate({ identifier: "PrivacySettlement" });
export type PrivacySettlement = typeof PrivacySettlement.Type;

export const PrivacyResult = Schema.Union([
  Schema.Literals(["running", "already_written"]),
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

