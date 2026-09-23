// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the attach form holds, what each field of it means, and the
// derivations the previews and the two commands read them through. The
// boxes themselves are `form.svelte`; this file touches no DOM, so the
// rules below are testable without compiling a component.
//
// **A provider is named by its URL.** The id is the key of
// `[model_providers.<id>]` and what `secret:providers/<id>` derives
// from, so a form that asks for it first asks somebody to invent this
// city's bookkeeping before they may paste the one thing they arrived
// with. The host already carries the name the provider is known by, so
// the id is derived from it and both naming boxes sit under `advanced`,
// where somebody who keeps a `config.toml` of their own overrules them.
//
// **The derivation is one rule and no host table.** The label before
// the last one is `openai` for `api.openai.com`, `anthropic` for
// `api.anthropic.com` and `openrouter` for `openrouter.ai`, so a table
// of known hosts on this side would be a second home for what
// `gateway::provider::preset` already holds, and the two would
// disagree the first time a provider moved. `presets.ts` is a
// different fact - which request shapes a known host answers - and it
// only gates a control before the click; it never derives a name.

import { WIRE_APIS, dialectOf } from "../../../core/commands";
import type { Endpoint, Pair, Tuning, WireApi } from "../../../core/commands";
import { referenceFor, referenceText } from "../../../core/enrol";
import type { Key } from "../../../core/lang";
import { get } from "svelte/store";
import { preferences } from "../../../core/prefs";
import type { Proxying } from "../../../wire";
import type { Choice, Group } from "../../parts/segmented";

// What a provider id may be spelled with, which is what a TOML table key
// and a secret reference can both carry unquoted.
export const ID_SHAPE = /^[a-z0-9][a-z0-9-]*$/;

// What a base URL must look like, spelled once for its two readers:
// the box's own `pattern`, which the browser checks while the person
// is still filling the form in, and `hostOf` below, which is what the
// look and attach controls are gated on.
//
// **`type="url"` is not that rule.** It accepts `mailto:somebody` and
// every other scheme, so a value this form refuses used to sit in a
// box the browser had called valid, and the person learnt of it by
// pressing a control that stayed grey without saying why.
//
// A provider states the root of its API and never the full path, so
// the shape is an optional scheme, an ASCII host, an optional port, an
// optional path, and nothing after it. A query string is refused
// because a root never carries one, and every face hangs its own path
// off this value.
//
// **The scheme is optional because the city fills it in.**
// `gateway::normalise` reads a missing scheme as `https://`, or
// `http://` for an address on this machine, so a form that demanded
// one refused text the city would have accepted - and the two
// documented forms a vendor prints, `api.openai.com/v1` and
// `127.0.0.1:11434`, are exactly the text that got refused. This box
// now makes the widest judgement the city makes and never a narrower
// one; the scheme a value is called under is settled once, there.
export const BASE_URL = /^(?:https?:\/\/)?([a-zA-Z0-9.-]+)(?::[0-9]+)?(?:\/[^\s?#]*)?$/;

// The host a base URL names, which is what a person reads a reachability
// report about. A URL this form would refuse has no host, and the form
// says so before it lets anybody look.
export function hostOf(baseUrl: string): string | null {
  return BASE_URL.exec(baseUrl.trim())?.[1] ?? null;
}

// A name this form derives and a person may overrule.
//
// A cleared box is `derived` rather than a typed empty string: somebody
// who empties the box is asking for the derivation back, which is what
// the placeholder in front of them is already showing.
export type Naming =
  | { readonly kind: "derived" }
  | { readonly kind: "typed"; readonly value: string };

// What a box holding this text means. Trimming happens where the value
// is read rather than here: a name is edited a character at a time, and
// a store that ate the space just typed would eat the word after it.
export function naming(typed: string): Naming {
  return typed.trim() === "" ? { kind: "derived" } : { kind: "typed", value: typed };
}

// The text a naming box shows. Empty for a derived name, so the
// placeholder beside it is what the person reads.
export function boxed(held: Naming): string {
  return held.kind === "typed" ? held.value : "";
}

// One row of either key-value table as the form edits it. `key` is the
// row's own name, so a list of rows keeps each row's boxes with the row
// when one is removed - a list diffed by position would hand the box a
// person is typing in to whichever row slid into its place. The wire's
// `Pair` is what these fold into at send time.
export interface Line {
  readonly key: string;
  name: string;
  value: string;
}

// A row nobody has typed into yet, carrying a name it will keep.
export function blankLine(): Line {
  return { key: crypto.randomUUID(), name: "", value: "" };
}

// Everything the form holds. One value rather than a dozen signals:
// these fields are read together by two previews and two commands, and
// a preview that read eleven signals one at a time was eleven chances
// to forget one.
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

// The name a host suggests, in the shape a TOML table key and a secret
// reference can both carry.
//
// The label before the last one is what a provider is known by. A host
// with no such label - a bare name, or an address - is its own name,
// spelled the way a table key can be, because `127.0.0.1` has a `0`
// where every other host has its name.
export function idFrom(host: string | null): string {
  if (host === null) return "";
  const lower = host.toLowerCase();
  const labels = lower.split(".");
  const second = labels.at(-2);
  const named = labels.every((label) => /^[0-9]+$/.test(label)) ? lower : (second ?? lower);
  return named.replace(/[^a-z0-9-]+/g, "-").replace(/^-+/, "").replace(/-+$/, "");
}

// The id this draft is filed under: the one somebody typed, or the one
// its URL suggests.
export function idOf(draft: Draft): string {
  return draft.id.kind === "typed" ? draft.id.value.trim() : idFrom(hostOf(draft.baseUrl));
}

// The reference a key filed under this id would be filed as, shown
// before anything is enrolled so a person can see what the form is
// about to file their key under. The spelling is `core/enrol.ts`'s,
// which is also where the vault's own answer replaces it.
export function referenceOf(id: string): string {
  return referenceText(referenceFor(id === "" ? "<id>" : id));
}

// The three settings, each with the word it is offered under and the
// sentence that says which machine it is right for. A table rather than
// three branches: the control draws itself from it, and a fourth
// setting would be a row.
export const PROXYINGS: readonly (readonly [Proxying, Key, Key])[] = [
  ["except_local", "setup_proxying_except_local", "setup_proxying_note_except_local"],
  ["always", "setup_proxying_always", "setup_proxying_note_always"],
  ["never", "setup_proxying_never", "setup_proxying_note_never"],
];

// The sentence that says which machine one rule is right for, which
// the network screen and this form both draw under the control.
export function proxyingNote(rule: Proxying): Key | undefined {
  return PROXYINGS.find(([setting]) => setting === rule)?.[2];
}

// The two laboratories whose request shapes this form offers, and the
// token each paints its cells with. The mapping lives here and nowhere
// else: no other screen colours anything by which laboratory it came
// from.
// wording-ok: two company names, which no language translates
const OPEN_AI: Group = { label: "OpenAI", tone: "plain" };
// wording-ok: a company name, which no language translates
const ANTHROPIC: Group = { label: "Anthropic", tone: "alert" };

// The three `wire_api` values as cells of one control, grouped by the
// laboratory that defined each shape. Which cells a particular host
// offers is `presets.ts`'s fact, applied by the form as a `why` on the
// cells it does not answer in.
//
// A cell is worded with the wire value itself, not out of the phrase
// table: each of the three is one token that reads the same in both
// languages, and a translated `wire_api` would be a value nobody can
// paste into a `config.toml`.
export function wireChoices(): readonly Choice<WireApi>[] {
  // wording-ok: the wire values `chat` / `responses` / `messages`, spelled the same in both languages
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
  timeoutMs: "60000",
  requestRetries: "4",
  streamIdleMs: "300000",
  headers: [],
  overrides: [],
};

// An empty form, carrying the proxy rule this machine was last told to
// start new endpoints with.
export function freshDraft(): Draft {
  return { ...FRESH, proxying: get(preferences().held).proxying };
}

// A box of digits, or nothing. An empty box and a box holding letters
// both mean "the city's own", which is what absence is on the wire.
function figureIn(text: string): number | null {
  const trimmed = text.trim();
  return /^[0-9]+$/.test(trimmed) ? Number.parseInt(trimmed, 10) : null;
}

// What the advanced section settled, as the two commands carry it.
export function tuningOf(draft: Draft): Tuning {
  const named = (rows: readonly Line[]): Pair[] =>
    rows
      .filter((row) => row.name.trim() !== "")
      .map((row) => ({ name: row.name.trim(), value: row.value }));
  return {
    // Absent unless somebody typed another name. The city falls back to
    // the id when no label was stated (`AttachedEndpoint::label`), so a
    // client that sent the id as the label would be a second home for
    // that fallback and would outlive a rename.
    label: draft.label.kind === "typed" ? draft.label.value : null,
    timeoutMs: figureIn(draft.timeoutMs),
    requestMaxRetries: figureIn(draft.requestRetries),
    streamIdleTimeoutMs: figureIn(draft.streamIdleMs),
    headers: named(draft.headers),
    overrides: named(draft.overrides),
    proxying: draft.proxying,
  };
}

// The endpoint this draft describes. `secret` is the reference the
// vault answered with for the key typed just now, and nothing when the
// key box is empty - which is not the same as no key: the city keeps
// what it has archived under this id (sprawling-SPEC 8-81). Every wire
// a provider states now has a kind this city can call, so there is no
// shape left for this to refuse.
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
