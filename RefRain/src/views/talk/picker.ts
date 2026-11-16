// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors


import {
  DEFAULT_EFFORT,
  type Effort,
  type EndpointSummary,
  type ModelFactsSummary,
  type ThinkingOffer,
} from "../../wire";
import type { Names } from "./composer";


export interface Offer extends Names {
  
  readonly provider: string;
  
  
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


export type Order = "providers_first" | "models_first";

export function orderOf(offers: readonly Offer[]): Order {
  const models = new Set(offers.map((each) => each.canonical)).size;
  const providers = new Set(offers.map((each) => each.endpoint)).size;
  return models > providers ? "providers_first" : "models_first";
}


export interface Entry {
  readonly id: string;
  readonly offers: readonly Offer[];
}


export function parentOf(order: Order, offer: Names & { readonly canonical: string }): string {
  return order === "models_first" ? offer.canonical : offer.endpoint;
}

export function childOf(order: Order, offer: Names): string {
  return order === "models_first" ? offer.endpoint : offer.model;
}


export function firstLevel(order: Order, offers: readonly Offer[]): readonly Entry[] {
  const groups = new Map<string, Offer[]>();
  for (const offer of offers) {
    const key = parentOf(order, offer);
    groups.set(key, [...(groups.get(key) ?? []), offer]);
  }
  return [...groups].map(([id, held]) => ({ id, offers: held }));
}


export function secondLevel(order: Order, parent: Entry | undefined): readonly Entry[] {
  return (parent?.offers ?? []).map((offer) => ({ id: childOf(order, offer), offers: [offer] }));
}


export const SHOWN = 5;
export const RECENT = 3;


export interface Windowed {
  readonly entries: readonly Entry[];
  readonly more: number;
}

export function windowed(
  entries: readonly Entry[],
  pinned: string | undefined,
  whole: boolean,
): Windowed {
  if (whole || entries.length <= SHOWN) return { entries, more: 0 };
  const head = entries.slice(0, SHOWN);
  const late = entries.slice(SHOWN).find((each) => each.id === pinned);
  return {
    entries: late === undefined ? head : [...head.slice(0, SHOWN - 1), late],
    more: entries.length - SHOWN,
  };
}


export function drawnRows(count: number): number {
  return count > SHOWN ? SHOWN + 1 : count;
}


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


export function matches(entry: Entry, query: string): boolean {
  const needle = query.trim().toLowerCase();
  return entry.offers.some((offer) =>
    [offer.model, offer.canonical, offer.provider].some((said) =>
      said.toLowerCase().includes(needle),
    ),
  );
}


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


export type Segment = "model" | "provider" | "level";


export const SECTION = {
  recent: "recent",
  first: "first",
  second: "second",
  thinking: "thinking",
} as const;
export type Section = (typeof SECTION)[keyof typeof SECTION];


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


export interface Combination extends Names {
  readonly level: Effort | null;
}

export const sameCombination = (a: Combination, b: Combination): boolean =>
  a.endpoint === b.endpoint && a.model === b.model && a.level === b.level;


export function recentOf(
  now: Combination | null,
  kept: readonly Combination[],
  offers: readonly Offer[],
): readonly Combination[] {
  const served = (each: Combination): boolean =>
    offers.some((offer) => offer.endpoint === each.endpoint && offer.model === each.model);
  return [
    ...(now === null ? [] : [now]),
    ...kept.filter((each) => now === null || !sameCombination(each, now)),
  ]
    .filter(served)
    .slice(0, RECENT);
}
