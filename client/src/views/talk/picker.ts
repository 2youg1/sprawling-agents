// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the model picker on the settings row decides before anything is
// drawn (docs/frontend-method.md §7I, client/spec/Views/Parts/Picker.lean):
// which models the city serves and from whom, which of the two choice
// levels comes first, which rows each list shows, how much room each
// section keeps, and which thinking level a request would use. The seat
// (`settings_row.svelte`) holds what is open and typed; `picker_look.ts`
// turns this into the value `picker.look.svelte` draws.

import { DEFAULT_EFFORT, type Effort, type EndpointSummary, type ModelFactsSummary, type ThinkingOffer } from "../../wire";
import type { Names } from "./composer";

// One model one provider serves: the pair a selection names, and what
// the provider said about it.
export interface Offer extends Names {
  // The provider's name a person reads.
  readonly provider: string;
  // The model's identity across providers (`gateway::provider::identity`);
  // two offers are the same model only when these are equal.
  readonly canonical: string;
  readonly facts: ModelFactsSummary;
  readonly local: boolean;
}

export function offersOf(endpoints: readonly EndpointSummary[]): readonly Offer[] {
  return endpoints.flatMap((endpoint) =>
    endpoint.models.map((facts) => ({
      endpoint: endpoint.name,
      provider: endpoint.label,
      model: facts.id,
      canonical: facts.canonical,
      facts,
      local: endpoint.local,
    })),
  );
}

// Which choice level the picker lists first. A city with more models
// than providers (an aggregator such as OpenRouter) is narrowed by
// provider first; any other is narrowed by model first, and the
// providers of the chosen model form the band under it. The order is a
// function of the offers alone, so it moves only when the endpoints do.
export type Order = "providers_first" | "models_first";

export function orderOf(offers: readonly Offer[]): Order {
  const models = new Set(offers.map((each) => each.canonical)).size;
  const providers = new Set(offers.map((each) => each.endpoint)).size;
  return models > providers ? "providers_first" : "models_first";
}

// What one row of a choice level stands for: a model across its
// providers, or a provider across its models.
export interface Entry {
  readonly id: string;
  readonly offers: readonly Offer[];
}

// The entry an offer is listed under on the first level, and its row
// on the second.
export function parentOf(order: Order, offer: Names & { readonly canonical: string }): string {
  return order === "models_first" ? offer.canonical : offer.endpoint;
}

export function childOf(order: Order, offer: Names): string {
  return order === "models_first" ? offer.endpoint : offer.model;
}

// The first level: each model (grouped by `canonical`) or each provider
// once, in the order the city lists its endpoints and their models.
export function firstLevel(order: Order, offers: readonly Offer[]): readonly Entry[] {
  const groups = new Map<string, Offer[]>();
  for (const offer of offers) {
    const key = parentOf(order, offer);
    groups.set(key, [...(groups.get(key) ?? []), offer]);
  }
  return [...groups].map(([id, held]) => ({ id, offers: held }));
}

// The second level under one first-level entry: each of its offers.
export function secondLevel(order: Order, parent: Entry | undefined): readonly Entry[] {
  return (parent?.offers ?? []).map((offer) => ({ id: childOf(order, offer), offers: [offer] }));
}

// ---------------------------------------------------------------- rows

// How many rows a list shows before "more…", and how many recent
// combinations the picker keeps. Each list of a picker is short enough
// to read in one glance; the rest is one activation or a few letters
// away.
export const SHOWN = 5;
export const RECENT = 3;

// The rows one list shows: every entry while it is filtered or opened in
// full, else the first `SHOWN`, with the entry chosen when the picker
// opened taking the last place when it would fall past them, and the
// count of what "more…" holds.
export interface Windowed {
  readonly entries: readonly Entry[];
  readonly more: number;
}

export function windowed(entries: readonly Entry[], pinned: string | undefined, whole: boolean): Windowed {
  if (whole || entries.length <= SHOWN) return { entries, more: 0 };
  const head = entries.slice(0, SHOWN);
  const late = entries.slice(SHOWN).find((each) => each.id === pinned);
  return {
    entries: late === undefined ? head : [...head.slice(0, SHOWN - 1), late],
    more: entries.length - SHOWN,
  };
}

// How many rows a list of `count` entries takes when nothing is typed:
// the shown entries and the "more…" row. Opening a list in full keeps
// this height and scrolls inside it, so no reachable state is taller.
export function drawnRows(count: number): number {
  return count > SHOWN ? SHOWN + 1 : count;
}

// The room each section keeps for the largest state reachable from an
// open picker without typing (client/spec/Views/Parts/Picker.lean
// `room`): the first level as drawn, the tallest second level any first
// entry expands, and the thinking band when any offer has levels. The
// popover's height is fixed by this when it opens, so choosing a model
// or a provider never moves a target already on the screen.
export interface Room {
  readonly first: number;
  readonly second: number;
  readonly band: boolean;
}

export function roomOf(order: Order, offers: readonly Offer[]): Room {
  const first = firstLevel(order, offers);
  return {
    first: drawnRows(first.length),
    second: Math.max(0, ...first.map((entry) => drawnRows(entry.offers.length))),
    band: offers.some((offer) => offer.facts.thinking.levels.length > 0),
  };
}

// Whether a typed filter keeps an entry: the model's own id at any
// provider, its canonical name, or the provider's name.
export function matches(entry: Entry, query: string): boolean {
  const needle = query.trim().toLowerCase();
  return entry.offers.some((offer) =>
    [offer.model, offer.canonical, offer.provider].some((said) => said.toLowerCase().includes(needle)),
  );
}

// ------------------------------------------------------------ thinking

// The level a request would use, and why: the stated or inherited level
// when this (Endpoint, model) offers it, else the default when offered
// (`DEFAULT_EFFORT`, generated from the kernel's constant), else the
// upstream's own stated default; `null` when the field is left out.
// `gateway::provider::thinking` holds the rule; this reads it for the
// mark, and the request itself is resolved there.
export interface Used {
  readonly level: Effort;
  readonly by: "stated" | "default";
}

export function usedLevel(offer: ThinkingOffer, stated: Effort | null): Used | null {
  const offered = (level: Effort | null | undefined): level is Effort =>
    level !== null && level !== undefined && offer.levels.includes(level);
  if (offered(stated)) return { level: stated, by: "stated" };
  if (offered(DEFAULT_EFFORT)) return { level: DEFAULT_EFFORT, by: "default" };
  if (offered(offer.default)) return { level: offer.default, by: "default" };
  return null;
}

// --------------------------------------------------- the open picker

// The three segments of the token, each opening the picker at its own
// section: the model, the provider and the thinking level.
export type Segment = "model" | "provider" | "level";

// The sections of the open picker, in drawing order: the popover's
// lists carry these ids, so a key moves between them in this order.
export const SECTION = { recent: "recent", first: "first", second: "second", thinking: "thinking" } as const;
export type Section = (typeof SECTION)[keyof typeof SECTION];

// The section a segment opens on. Activating the token by key, with no
// segment under a pointer, opens on the first choice level.
export function sectionOf(order: Order, segment: Segment | null): Section {
  switch (segment) {
    case "model":
      return order === "models_first" ? SECTION.first : SECTION.second;
    case "provider":
      return order === "models_first" ? SECTION.second : SECTION.first;
    case "level":
      return SECTION.thinking;
    case null:
      return SECTION.first;
  }
}

// What applying one row does to the picker (client/spec/Views/Parts/
// Picker.lean `after`): a model or a provider keeps it open, so the
// band under it can change; a level or a recent combination applies the
// whole choice and closes it; "more…" opens its list in full.
export type Applied = "stays" | "closes";

export function appliedIn(section: Section): Applied {
  switch (section) {
    case SECTION.first:
    case SECTION.second:
      return "stays";
    case SECTION.recent:
    case SECTION.thinking:
      return "closes";
  }
}

// One combination a person ended a choice with: the pair and the level
// the request used, kept so it is one activation away next time.
export interface Combination extends Names {
  readonly level: Effort | null;
}

export const sameCombination = (a: Combination, b: Combination): boolean =>
  a.endpoint === b.endpoint && a.model === b.model && a.level === b.level;

// The recent combinations a picker lists: the one in force first, then
// the ones before it that the city still serves, at most `RECENT`.
export function recentOf(now: Combination | null, kept: readonly Combination[], offers: readonly Offer[]): readonly Combination[] {
  const served = (each: Combination): boolean =>
    offers.some((offer) => offer.endpoint === each.endpoint && offer.model === each.model);
  return [...(now === null ? [] : [now]), ...kept.filter((each) => now === null || !sameCombination(each, now))]
    .filter(served)
    .slice(0, RECENT);
}
