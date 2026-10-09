// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What one drawing of the model picker reads - the facts the composer
// hands it, what the seat holds, and the scene worked out from both -
// and the words each row is given: a model by its name and how many
// providers serve it, a provider by its name and what the model costs
// there, a level by the upstream's own word. `picker_look.ts` builds the
// look from these.

import type { Lang } from "../../core/lang";
import { fill, say } from "../../core/lang";
import { kilo } from "../../core/time";
import type { Effort, EndpointSummary } from "../../wire";
import type { PopoverBinding } from "../parts/popover";
import type { Names } from "./composer";
import type { Combination, Entry, Offer, Order, Section, Segment, Used } from "./picker";
import { SECTION, firstLevel, offersOf, orderOf, parentOf, usedLevel } from "./picker";

// What the composer hands the picker.
export interface PickerFacts {
  readonly endpoints: readonly EndpointSummary[];
  // The city's `main` selection.
  readonly chosen: Names | undefined;
  // The level this page states, else the one the room inherits; `null`
  // when nobody stated one.
  readonly stated: Effort | null;
  readonly pick: {
    readonly model: (names: Names) => void;
    readonly level: (level: Effort) => void;
  };
}

// What the seat holds between draws.
export interface PickerHeld {
  readonly open: boolean;
  // The segment the picker was opened from; `null` for a key press.
  readonly segment: Segment | null;
  // The selection made inside this opening, ahead of the city's answer.
  readonly pick: Names | null;
  // The first-level entry whose second level is shown.
  readonly parent: string | null;
  // The entries chosen when the picker opened, kept inside the first five.
  readonly pinned: { readonly first: string | undefined; readonly second: string | undefined };
  // The combination in force when the picker opened, which heads the
  // recent section for as long as it stays open: a choice made inside
  // this opening adds no row there, so the popover keeps its height.
  readonly opened: Combination | null;
  readonly query: string;
  readonly whole: readonly Section[];
  readonly kept: readonly Combination[];
  readonly binding: PopoverBinding | null;
  readonly active: string | null;
}

// Everything one drawing of the picker reads, worked out once.
export interface Scene {
  readonly lang: Lang;
  readonly order: Order;
  readonly offers: readonly Offer[];
  readonly offer: Offer | undefined;
  readonly used: Used | null;
  readonly first: readonly Entry[];
  readonly parent: Entry | undefined;
}

export function sceneOf(lang: Lang, facts: PickerFacts, held: PickerHeld): Scene {
  const offers = offersOf(facts.endpoints);
  const order = orderOf(offers);
  const names = held.pick ?? facts.chosen;
  const offer = offers.find((each) => each.endpoint === names?.endpoint && each.model === names.model);
  const first = firstLevel(order, offers);
  const parentId = held.parent ?? (offer === undefined ? first.at(0)?.id : parentOf(order, offer));
  return {
    lang,
    order,
    offers,
    offer,
    used: offer === undefined ? null : usedLevel(offer.facts.thinking, facts.stated),
    first,
    parent: first.find((entry) => entry.id === parentId),
  };
}

// The combination a scene shows: its pair, and the level a request
// there would use.
export function combinationOf(scene: Pick<Scene, "offer" | "used">): Combination | null {
  const { offer, used } = scene;
  return offer === undefined ? null : { endpoint: offer.endpoint, model: offer.model, level: used?.level ?? null };
}

// The combination in force before anything is chosen: the city's `main`
// and the level a request to it would use.
export function inForce(facts: PickerFacts): Combination | null {
  const offer = offersOf(facts.endpoints).find((each) => each.endpoint === facts.chosen?.endpoint && each.model === facts.chosen.model);
  return combinationOf({ offer, used: offer === undefined ? null : usedLevel(offer.facts.thinking, facts.stated) });
}

// A level as the upstream words it, else as the city does.
export function levelWord(scene: Scene, level: Effort): string {
  return scene.offer?.facts.thinking.words.find((each) => each.effort === level)?.word ?? say(scene.lang, `effort_${level}`);
}

// What an offer costs and holds, in the provider's own figures.
export function priceOf(lang: Lang, offer: Offer): string {
  const { input_price: input, output_price: output, context_tokens: context } = offer.facts;
  const price = offer.local
    ? say(lang, "picker_price_local")
    : typeof input === "string" && typeof output === "string"
      ? `${input}/${output}`
      : say(lang, "picker_price_unstated");
  return context === null || context === undefined ? price : `${price} · ${kilo(context)}`;
}

export function comboName(scene: Scene, combo: Combination): string {
  const offer = scene.offers.find((each) => each.endpoint === combo.endpoint && each.model === combo.model);
  const level = combo.level === null ? [] : [say(scene.lang, `effort_${combo.level}`)];
  return [combo.model, offer?.provider ?? combo.endpoint, ...level].join(" · ");
}

// The list a typed filter narrows: the one that holds models.
export function modelsSection(order: Order): Section {
  return order === "models_first" ? SECTION.first : SECTION.second;
}

export function noMatch(lang: Lang, order: Order, section: Section): string {
  return say(lang, (section === SECTION.first) === (order === "models_first") ? "picker_no_model_match" : "picker_no_provider_match");
}

export function comboId(combo: Combination): string {
  return [combo.endpoint, combo.model, combo.level ?? ""].join("\u0000");
}

// The words of one entry: a model reads its name and how many providers
// serve it, a provider its name and what the model costs there, with the
// provider's own id for the model where it differs from the shared one.
export function entryWords(scene: Scene, section: Section, entry: Entry): { label: string; secondary: string | undefined } {
  const { lang, order } = scene;
  const only = entry.offers.length === 1 ? entry.offers.at(0) : undefined;
  const listsModels = (section === SECTION.first) === (order === "models_first");
  if (section === SECTION.first && order === "models_first") {
    return { label: entry.id, secondary: only === undefined ? fill(say(lang, "picker_provider_count"), { n: String(entry.offers.length) }) : only.provider };
  }
  if (section === SECTION.first) return { label: entry.offers.at(0)?.provider ?? entry.id, secondary: entry.offers.at(0)?.local === true ? say(lang, "picker_price_local") : undefined };
  if (only === undefined) return { label: entry.id, secondary: undefined };
  if (listsModels) return { label: only.model, secondary: priceOf(lang, only) };
  const own = only.model === only.canonical ? [] : [only.model];
  return { label: only.provider, secondary: [...own, priceOf(lang, only)].join(" · ") };
}
