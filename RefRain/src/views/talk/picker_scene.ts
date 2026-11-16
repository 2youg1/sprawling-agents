// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Lang } from "../../core/lang";
import { fill, say } from "../../core/lang";
import { kilo } from "../../core/time";
import type { Effort, EndpointSummary } from "../../wire";
import type { PopoverBinding } from "../parts/popover";
import type { Names } from "./composer";
import type { Combination, Entry, Offer, Order, Section, Segment, Used } from "./picker";
import { SECTION, firstLevel, offersOf, orderOf, parentOf, usedLevel } from "./picker";

export interface PickerFacts {
  readonly endpoints: readonly EndpointSummary[];
  readonly chosen: Names | undefined;
  readonly stated: Effort | null;
  readonly pick: {
    readonly model: (names: Names) => void;
    readonly level: (level: Effort) => void;
  };
}

export interface PickerHeld {
  readonly open: boolean;
  readonly segment: Segment | null;
  readonly pick: Names | null;
  readonly parent: string | null;
  readonly pinned: { readonly first: string | undefined; readonly second: string | undefined };
  readonly opened: Combination | null;
  readonly query: string;
  readonly whole: readonly Section[];
  readonly kept: readonly Combination[];
  readonly binding: PopoverBinding | null;
  readonly active: string | null;
}

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
  const offer = offers.find(
    (each) => each.endpoint === names?.endpoint && each.model === names.model,
  );
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

export function combinationOf(scene: Pick<Scene, "offer" | "used">): Combination | null {
  const { offer, used } = scene;
  return offer === undefined
    ? null
    : { endpoint: offer.endpoint, model: offer.model, level: used?.level ?? null };
}

export function inForce(facts: PickerFacts): Combination | null {
  const offer = offersOf(facts.endpoints).find(
    (each) => each.endpoint === facts.chosen?.endpoint && each.model === facts.chosen.model,
  );
  return combinationOf({
    offer,
    used: offer === undefined ? null : usedLevel(offer.facts.thinking, facts.stated),
  });
}

export function levelWord(scene: Scene, level: Effort): string {
  return (
    scene.offer?.facts.thinking.words.find((each) => each.effort === level)?.word ??
    say(scene.lang, `effort_${level}`)
  );
}

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
  const offer = scene.offers.find(
    (each) => each.endpoint === combo.endpoint && each.model === combo.model,
  );
  const level = combo.level === null ? [] : [say(scene.lang, `effort_${combo.level}`)];
  return [combo.model, offer?.provider ?? combo.endpoint, ...level].join(" · ");
}

export function modelsSection(order: Order): Section {
  return order === "models_first" ? SECTION.first : SECTION.second;
}

export function noMatch(lang: Lang, order: Order, section: Section): string {
  return say(
    lang,
    (section === SECTION.first) === (order === "models_first")
      ? "picker_no_model_match"
      : "picker_no_provider_match",
  );
}

export function comboId(combo: Combination): string {
  return [combo.endpoint, combo.model, combo.level ?? ""].join("\u0000");
}

export function entryWords(
  scene: Scene,
  section: Section,
  entry: Entry,
): { label: string; secondary: string | undefined } {
  const { lang, order } = scene;
  const only = entry.offers.length === 1 ? entry.offers.at(0) : undefined;
  const listsModels = (section === SECTION.first) === (order === "models_first");
  if (section === SECTION.first && order === "models_first") {
    return {
      label: entry.id,
      secondary:
        only === undefined
          ? fill(say(lang, "picker_provider_count"), { n: String(entry.offers.length) })
          : only.provider,
    };
  }
  if (section === SECTION.first)
    return {
      label: entry.offers.at(0)?.provider ?? entry.id,
      secondary: entry.offers.at(0)?.local === true ? say(lang, "picker_price_local") : undefined,
    };
  if (only === undefined) return { label: entry.id, secondary: undefined };
  if (listsModels) return { label: only.model, secondary: priceOf(lang, only) };
  const own = only.model === only.canonical ? [] : [only.model];
  return { label: only.provider, secondary: [...own, priceOf(lang, only)].join(" · ") };
}
