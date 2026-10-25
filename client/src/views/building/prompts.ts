// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the building page's prompts section shows (client/Spec.lean 4-50,
// its ninth part): the bytes each slot of the newest run here was sent,
// and one card for each prompt document of the next run, in the order
// `Query::Layers` gives them. Which document is sent, and which a nearer
// one replaces, is the city's answer; this file only words it and says
// which steps each card offers.

import { fill, say } from "../../core/lang";
import type { Key, Lang } from "../../core/lang";
import { count } from "../../core/time";
import type { Address, B3Hash, LayerDocument, PrefixAnswer, PromptLayer, PromptStep } from "../../wire";
import { SLOT_WORD } from "../run/prompt";

// One room the section can show, as its key.
export interface RoomChoice {
  readonly label: string;
  readonly current: boolean;
  readonly onPress: () => void;
}

// Everything `prompts.look.svelte` draws, already worded by the seat.
export interface PromptsLook {
  readonly rooms: readonly RoomChoice[];
  readonly roomsLabel: string;
  readonly heading: string;
  readonly slots: readonly SlotLook[];
  readonly run: { readonly label: string; readonly href: string } | undefined;
  readonly note: string;
  readonly documentsLabel: string;
  readonly cards: readonly CardLook[];
  readonly actionLabel: (action: CardAction) => string;
  readonly actionName: (card: CardLook, action: CardAction) => string;
  readonly onAction: (card: CardLook, action: CardAction) => void;
}

// One slot of the newest run here, as it was sent.
export interface SlotLook {
  readonly key: string;
  readonly slot: string;
  readonly size: string;
}

// What a card offers. `open` reads the document in RefRain on the
// right; the others are `PutPrompt` steps, and the two that remove or
// replace words are asked first.
export type CardAction = { readonly kind: "open"; readonly path: string } | { readonly kind: "step"; readonly step: PromptStep };

export interface CardLook {
  readonly key: string;
  readonly at: Address;
  // The version a step starts from: what the city saw on disk.
  readonly base: B3Hash;
  readonly title: string;
  readonly about: string;
  readonly layer: string;
  readonly state: string;
  // Whether the next run receives it, which the look marks.
  readonly sent: boolean;
  // How the copy stands against this build's default, for the three
  // documents the binary carries.
  readonly standing: string | undefined;
  readonly actions: readonly CardAction[];
}

const ACTION_WORD: Record<"open" | PromptStep, Key> = {
  open: "prompts_open",
  own: "prompts_own",
  inherit: "prompts_inherit",
  restore: "prompts_restore",
};

// The word a card's action is pressed by.
export function actionWord(action: CardAction | PromptStep, lang: Lang): string {
  const key = typeof action === "string" ? action : action.kind === "open" ? "open" : action.step;
  return say(lang, ACTION_WORD[key]);
}

// The name an action is read by: the verb and the document it acts on.
export function actionName(card: CardLook, action: CardAction, lang: Lang): string {
  return fill(say(lang, "prompts_action_on"), { action: actionWord(action, lang), at: card.at });
}

const LAYER_WORD: Record<PromptLayer, Key> = {
  city: "prompts_layer_city",
  building: "prompts_layer_building",
  room: "prompts_layer_room",
};

// The newest run's four slots, in send order.
export function slotsOf(prefix: PrefixAnswer, lang: Lang): readonly SlotLook[] {
  return prefix.segments.map((segment) => ({
    key: segment.slot,
    slot: say(lang, SLOT_WORD[segment.slot]),
    size: fill(say(lang, "run_prompt_bytes"), { n: count(segment.bytes) }),
  }));
}

// One card per document. `building` is the page's building: a document
// inside it opens by its path there, and any other by its path from the
// city root, which RefRain reads as the city's (`documentAt`).
export function cardsOf(documents: readonly LayerDocument[], building: Address, lang: Lang): readonly CardLook[] {
  return documents.map((doc) => ({
    key: doc.at,
    at: doc.at,
    base: doc.version,
    title: doc.at.slice(doc.at.lastIndexOf("/") + 1),
    about: aboutOf(doc, lang),
    layer: say(lang, LAYER_WORD[doc.layer]),
    state: stateOf(doc, lang),
    sent: typeof doc.state === "object" && "sent" in doc.state,
    standing: standingOf(doc, lang),
    actions: actionsOf(doc, building),
  }));
}

function aboutOf(doc: LayerDocument, lang: Lang): string {
  const kind = doc.kind;
  switch (kind) {
    case "city":
      return say(lang, "prompts_kind_city");
    case "conventions":
      return say(lang, "prompts_kind_conventions");
    case "identity":
      return say(lang, "prompts_kind_identity");
    default:
      return fill(say(lang, "prompts_kind_note"), { model: `${kind.model_note.provider}/${kind.model_note.model}` });
  }
}

function stateOf(doc: LayerDocument, lang: Lang): string {
  const state = doc.state;
  if (state === "absent") return say(lang, "prompts_absent");
  if ("sent" in state) return fill(say(lang, "prompts_sent"), { n: count(state.sent.bytes) });
  if ("replaced" in state) return fill(say(lang, "prompts_replaced"), { by: state.replaced.by });
  return fill(say(lang, "prompts_withheld"), { why: state.withheld.why });
}

function standingOf(doc: LayerDocument, lang: Lang): string | undefined {
  switch (doc.shipped?.standing) {
    case undefined:
    case "absent":
      return undefined;
    case "current":
      return say(lang, "prompts_default_current");
    case "earlier":
      return say(lang, "prompts_default_earlier");
    case "edited":
      return say(lang, "prompts_default_edited");
  }
}

// What a card offers: a document that is there opens. A building's own
// `City.md` and the notes of either layer are what a layer may start
// where it is absent and give up where it is there; the three documents
// the binary carries are restored to the default unless they hold it.
export function actionsOf(doc: LayerDocument, building: Address): readonly CardAction[] {
  const shipped = doc.shipped ?? null;
  const state = doc.state;
  const there = state !== "absent" && !("withheld" in state);
  const layered = shipped === null && (doc.kind === "city" || typeof doc.kind === "object");
  const actions: CardAction[] = [];
  if (there) actions.push({ kind: "open", path: pathOf(doc.at, building) });
  if (layered && state === "absent") actions.push({ kind: "step", step: "own" });
  if (layered && there) actions.push({ kind: "step", step: "inherit" });
  if (shipped !== null && shipped.standing !== "current") actions.push({ kind: "step", step: "restore" });
  return actions;
}

// The path `openDocument` takes: inside the page's building, the part
// after it; anywhere else, the address from the city root after a `/`.
export function pathOf(at: Address, building: Address): string {
  const prefix = `${building}/`;
  return at.startsWith(prefix) ? at.slice(prefix.length) : `/${at}`;
}
