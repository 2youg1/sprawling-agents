// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { WIRE_APIS, dialectOf } from "../../../core/commands";
import type { Endpoint, Pair, Tuning, WireApi } from "../../../core/commands";
import { referenceFor, referenceText } from "../../../core/enrol";
import type { Key } from "../../../core/lang";
import { get } from "svelte/store";
import { PROXYING_RULES, preferences } from "../../../core/prefs";
import type { Proxying, TuningDefaults } from "../../../wire";
import type { Choice, Group } from "../../parts/segmented";

export const ID_SHAPE = /^[a-z0-9][a-z0-9-]*$/;

export const BASE_URL = /^(?:https?:\/\/)?([a-zA-Z0-9.\x2D]+)(?::[0-9]+)?(?:\/[^\s?#]*)?$/;

export function hostOf(baseUrl: string): string | null {
  return BASE_URL.exec(baseUrl.trim())?.[1] ?? null;
}

export type Naming =
  { readonly kind: "derived" } | { readonly kind: "typed"; readonly value: string };

export function naming(typed: string): Naming {
  return typed.trim() === "" ? { kind: "derived" } : { kind: "typed", value: typed };
}

export function boxed(held: Naming): string {
  return held.kind === "typed" ? held.value : "";
}

export interface Line {
  readonly key: string;
  name: string;
  value: string;
}

let made = 0;

export function blankLine(): Line {
  made += 1;
  return { key: `line-${String(made)}`, name: "", value: "" };
}

export interface Draft {
  id: Naming;
  label: Naming;
  baseUrl: string;
  wireApi: WireApi;
  key: string;
  timeoutMs: string;
  requestRetries: string;
  streamIdleMs: string;
  proxying: Proxying;
  headers: Line[];
  overrides: Line[];
}

export function idFrom(host: string | null): string {
  if (host === null) return "";
  const lower = host.toLowerCase();
  const labels = lower.split(".");
  const second = labels.at(-2);
  const named = labels.every((label) => /^[0-9]+$/.test(label)) ? lower : (second ?? lower);
  return named
    .replace(/[^a-z0-9-]+/g, "-")
    .replace(/^-+/, "")
    .replace(/-+$/, "");
}

export function idOf(draft: Draft): string {
  return draft.id.kind === "typed" ? draft.id.value.trim() : idFrom(hostOf(draft.baseUrl));
}

export function referenceOf(id: string): string {
  return referenceText(referenceFor(id === "" ? "<id>" : id));
}

const PROXYING_WORDS: Readonly<Record<Proxying, readonly [Key, Key]>> = {
  except_local: ["setup_proxying_except_local", "setup_proxying_note_except_local"],
  always: ["setup_proxying_always", "setup_proxying_note_always"],
  never: ["setup_proxying_never", "setup_proxying_note_never"],
};

export const PROXYINGS: readonly (readonly [Proxying, Key, Key])[] = PROXYING_RULES.map(
  (rule) => [rule, ...PROXYING_WORDS[rule]] as const,
);

export function proxyingNote(rule: Proxying): Key | undefined {
  return PROXYINGS.find(([setting]) => setting === rule)?.[2];
}

const OPEN_AI: Group = { label: "OpenAI", tone: "plain" };

const ANTHROPIC: Group = { label: "Anthropic", tone: "alert" };

export function wireChoices(): readonly Choice<WireApi>[] {
  return WIRE_APIS.map((api) => ({
    value: api,
    label: api,
    group: api === "messages" ? ANTHROPIC : OPEN_AI,
  }));
}

const FRESH: Omit<Draft, "proxying"> = {
  id: { kind: "derived" },
  label: { kind: "derived" },
  baseUrl: "",
  wireApi: "chat",
  key: "",
  timeoutMs: "",
  requestRetries: "",
  streamIdleMs: "",
  headers: [],
  overrides: [],
};

export type TuningField = "timeoutMs" | "requestRetries" | "streamIdleMs";

export function tuningHints(
  draft: Draft,
  defaults: TuningDefaults | undefined,
  untilHalted: string,
): Readonly<Record<TuningField, string>> {
  if (defaults === undefined) {
    return { timeoutMs: "", requestRetries: "", streamIdleMs: "" };
  }
  const timeout = figureIn(draft.timeoutMs) ?? defaults.timeout_ms;
  return {
    timeoutMs: String(defaults.timeout_ms),
    requestRetries: String(defaults.request_max_retries ?? untilHalted),
    streamIdleMs: String(defaults.stream_idle_timeout_ms ?? timeout),
  };
}

export function freshDraft(): Draft {
  return { ...FRESH, proxying: get(preferences().held).proxying };
}

export function figureIn(text: string): number | null {
  const trimmed = text.trim();
  return /^[0-9]+$/.test(trimmed) ? Number.parseInt(trimmed, 10) : null;
}

export function tuningOf(draft: Draft): Tuning {
  const named = (rows: readonly Line[]): Pair[] =>
    rows
      .filter((row) => row.name.trim() !== "")
      .map((row) => ({ name: row.name.trim(), value: row.value }));
  return {
    label: draft.label.kind === "typed" ? draft.label.value : null,
    timeoutMs: figureIn(draft.timeoutMs),
    requestMaxRetries: figureIn(draft.requestRetries),
    streamIdleTimeoutMs: figureIn(draft.streamIdleMs),
    headers: named(draft.headers),
    overrides: named(draft.overrides),
    proxying: draft.proxying,
  };
}

export function endpointOf(draft: Draft, secret: string | null): Endpoint {
  return {
    id: idOf(draft),
    baseUrl: draft.baseUrl.trim(),
    dialect: dialectOf(draft.wireApi),
    secret,
    authHeader: null,
    tuning: tuningOf(draft),
  };
}
