// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Answers to `Query::Privacy` as a host would give them, for the gallery
// and for the privacy page's wiring tests. The rows are a handful of real
// controls, each standing in one state worth looking at; their targets,
// written values and edition lists copy the host's table for the same
// control, so a fixture reads like the page a person meets.

import type {
  AxError,
  PrivacyAnswer,
  PrivacyCategory,
  PrivacyControlEntry,
  PrivacyHistory,
  PrivacyIntent,
  PrivacyValue,
} from "../../wire";
import { IdemKey } from "../../wire";

const ALL_EDITIONS = ["home", "pro", "enterprise", "education", "iot_enterprise", "server"] as const;
const POLICY_EDITIONS = ["pro", "enterprise", "education", "iot_enterprise"] as const;

const ONE: PrivacyValue = { value: "dword", number: 1 };
const ZERO: PrivacyValue = { value: "dword", number: 0 };
const ABSENT: PrivacyValue = { value: "absent" };
const DIGEST = "5f2c0d1e9a7b3c4d5e6f708192a3b4c5d6e7f8091a2b3c4d5e6f708192a3b4c5";

function hklm(path: string, name: string): PrivacyControlEntry["target"] {
  return { kind: "registry_value_hklm", path, name };
}

function hkcu(path: string, name: string): PrivacyControlEntry["target"] {
  return { kind: "registry_value_hkcu", path, name };
}

function read(value: PrivacyValue): PrivacyControlEntry["current"] {
  return { read: { value } };
}

function policy(category: PrivacyCategory, row: Partial<PrivacyControlEntry> & Pick<PrivacyControlEntry, "control" | "target">): PrivacyControlEntry {
  return {
    category,
    build_effect: "documented",
    current: read(ABSENT),
    editions: { honoured: [...POLICY_EDITIONS], ignored: [] },
    host_fit: "not_stated",
    originals: [],
    scope: "machine",
    written: ONE,
    ...row,
  };
}

const CONTROLS: readonly PrivacyControlEntry[] = [
  {
    control: "powershell_telemetry_optout",
    category: "diagnostics",
    build_effect: "documented",
    current: read(ABSENT),
    editions: { honoured: [...ALL_EDITIONS], ignored: [] },
    host_fit: "honoured",
    originals: [{ item: "k01", text: "POWERSHELLTELEMETRYOPTOUT=1" }],
    scope: "user",
    target: { kind: "environment_variable_user", name: "POWERSHELL_TELEMETRY_OPTOUT" },
    written: { value: "text", text: "1" },
  },
  {
    control: "ceip_usb_task",
    category: "diagnostics",
    build_effect: "documented",
    current: read({ value: "task_absent" }),
    editions: { honoured: [...ALL_EDITIONS], ignored: [] },
    host_fit: "honoured",
    originals: [{ item: "k07", text: "UsbCeip" }],
    scope: "machine",
    target: { kind: "scheduled_task_enabled", path: "\\Microsoft\\Windows\\Customer Experience Improvement Program\\", name: "UsbCeip" },
    written: null,
  },
  policy("diagnostics", {
    control: "diagnostic_data",
    target: hklm("SOFTWARE\\Policies\\Microsoft\\Windows\\DataCollection", "AllowTelemetry"),
    originals: [{ item: "k10", text: "AllowTelemetry=0" }],
    editions: { honoured: ["enterprise", "education", "iot_enterprise"], ignored: ["home", "pro"] },
    host_fit: "ignored",
    current: read({ value: "dword", number: 3 }),
    written: ZERO,
  }),
  {
    control: "device_census_task",
    category: "diagnostics",
    build_effect: "documented",
    current: read({ value: "task_enabled", definition_sha256: DIGEST }),
    editions: { honoured: [...ALL_EDITIONS], ignored: [] },
    host_fit: "honoured",
    originals: [],
    scope: "machine",
    target: { kind: "scheduled_task_enabled", path: "\\Microsoft\\Windows\\Device Information\\", name: "Device" },
    written: { value: "task_disabled", definition_sha256: DIGEST },
  },
  {
    control: "implicit_text_collection",
    category: "speech_input",
    build_effect: "documented",
    current: read(ZERO),
    editions: { honoured: [...ALL_EDITIONS], ignored: [] },
    host_fit: "honoured",
    originals: [
      { item: "k22", text: "RestrictImplicitTextCollection=1" },
      { item: "k31", text: "RestrictImplicitTextCollection=1 (InputPersonalization)" },
    ],
    scope: "user",
    target: hkcu("Software\\Microsoft\\InputPersonalization", "RestrictImplicitTextCollection"),
    written: ONE,
  },
  policy("location_sensors", {
    control: "location_provider",
    target: hklm("SOFTWARE\\Policies\\Microsoft\\Windows\\LocationAndSensors", "DisableWindowsLocationProvider"),
    originals: [{ item: "k14", text: "DisableWindowsLocationProvider=1" }],
    build_effect: "uncertain",
  }),
  policy("content", {
    control: "tailored_experiences",
    scope: "user",
    target: hkcu("SOFTWARE\\Policies\\Microsoft\\Windows\\CloudContent", "DisableTailoredExperiencesWithDiagnosticData"),
    originals: [{ item: "k18", text: "TailoredExperiencesWithDiagnosticDataEnabled=0" }],
  }),
  policy("activity_sync", {
    control: "activity_upload",
    target: hklm("SOFTWARE\\Policies\\Microsoft\\Windows\\System", "UploadUserActivities"),
    build_effect: "no_current_effect",
    written: ZERO,
  }),
  policy("app_permissions", {
    control: "app_location",
    target: hklm("SOFTWARE\\Policies\\Microsoft\\Windows\\AppPrivacy", "LetAppsAccessLocation"),
    originals: [{ item: "k33", text: "LetAppsAccessLocation=2" }],
    current: "access_denied",
    written: null,
  }),
  policy("windows_ai", {
    control: "recall_snapshots",
    target: hklm("SOFTWARE\\Policies\\Microsoft\\Windows\\WindowsAI", "DisableAIDataAnalysis"),
    current: read(ONE),
  }),
];

const RECALL: PrivacyIntent = { control: "recall_snapshots", operation: 1, original: ABSENT, modified: ONE };

const NOT_WRITTEN: PrivacyAnswer["not_written"] = [
  {
    line: { item: "k03", text: "DeviceCensus.exe outbound block" },
    // wording-ok: the wire name of a reason, which lang.json words
    reason: "needs_operation_kind",
    alternatives: ["device_census_task", "device_census_user_task"],
  },
  // wording-ok: the wire name of a reason, which lang.json words
  { line: { item: "k49", text: "LetAppsAccessDocumentsFolder=2" }, reason: "absent", alternatives: [] },
];

export const REFUSAL: AxError = {
  // wording-ok: fixture states the wire's own refusal fields, English as the city writes them
  action: "write the privacy control tailored_experiences",
  code: "E_SANDBOX_DENIED",
  nearby: [],
  // wording-ok: fixture states the wire's own refusal fields, English as the city writes them
  recovery: "this account may not write this policy key; nothing changed",
  retry: "no",
  // wording-ok: fixture states the wire's own refusal fields, English as the city writes them
  subject: "HKCU\\SOFTWARE\\Policies\\Microsoft\\Windows\\CloudContent",
};

export const IDEM = IdemKey.make("idem1-0123456789abcdef0123456789abcdef");

function answer(history: PrivacyHistory): PrivacyAnswer {
  return {
    host: { windows: { edition_id: "CoreCountrySpecific", edition: "home", build: "26100.4061", display_version: "24H2" } },
    controls: CONTROLS,
    not_written: NOT_WRITTEN,
    history,
    outcomes: [],
  };
}

// A Home host (`CoreCountrySpecific`) where this app owns one change.
export const HOME: PrivacyAnswer = answer({ disclosed: { owned: [RECALL] } });

// The same host with one change that has no conclusion: only its check
// is offered, on its own entry.
export const UNRESOLVED: PrivacyAnswer = answer({
  disclosed: { owned: [RECALL], unresolved: { control: "tailored_experiences", operation: 2, original: ABSENT, modified: ONE } },
});

// A city on another system: nothing read, nothing offered.
export const ELSEWHERE: PrivacyAnswer = {
  host: "not_windows",
  controls: CONTROLS.slice(0, 3).map((row) => ({ ...row, current: "not_read", written: null, host_fit: "not_stated" })),
  not_written: [],
  history: { disclosed: { owned: [] } },
  outcomes: [],
};
