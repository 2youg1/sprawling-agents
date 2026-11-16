// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { fill, say } from "../../core/lang";
import type { Key, Lang } from "../../core/lang";
import { count } from "../../core/time";
import type {
  Address,
  B3Hash,
  LayerDocument,
  PrefixAnswer,
  PromptLayer,
  PromptStep,
} from "../../wire";
import { SLOT_WORD } from "../run/prompt";

export interface RoomChoice {
  readonly label: string;
  readonly current: boolean;
  readonly onPress: () => void;
}

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

export interface SlotLook {
  readonly key: string;
  readonly slot: string;
  readonly size: string;
}

export type CardAction =
  | { readonly kind: "open"; readonly path: string }
  | { readonly kind: "step"; readonly step: PromptStep };

export interface CardLook {
  readonly key: string;
  readonly at: Address;

  readonly base: B3Hash;
  readonly title: string;
  readonly about: string;
  readonly layer: string;
  readonly state: string;

  readonly sent: boolean;

  readonly standing: string | undefined;
  readonly actions: readonly CardAction[];
}

const ACTION_WORD: Record<"open" | PromptStep, Key> = {
  open: "prompts_open",
  own: "prompts_own",
  inherit: "prompts_inherit",
  restore: "prompts_restore",
};

export function actionWord(action: CardAction | PromptStep, lang: Lang): string {
  if (typeof action === "string") return say(lang, ACTION_WORD[action]);
  return say(lang, ACTION_WORD[action.kind === "open" ? "open" : action.step]);
}

export function actionName(card: CardLook, action: CardAction, lang: Lang): string {
  return fill(say(lang, "prompts_action_on"), { action: actionWord(action, lang), at: card.at });
}

const LAYER_WORD: Record<PromptLayer, Key> = {
  city: "prompts_layer_city",
  building: "prompts_layer_building",
  room: "prompts_layer_room",
};

export function slotsOf(prefix: PrefixAnswer, lang: Lang): readonly SlotLook[] {
  return prefix.segments.map((segment) => ({
    key: segment.slot,
    slot: say(lang, SLOT_WORD[segment.slot]),
    size: fill(say(lang, "run_prompt_bytes"), { n: count(segment.bytes) }),
  }));
}

export function cardsOf(
  documents: readonly LayerDocument[],
  building: Address,
  lang: Lang,
): readonly CardLook[] {
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
      return fill(say(lang, "prompts_kind_note"), {
        model: `${kind.model_note.provider}/${kind.model_note.model}`,
      });
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

export function actionsOf(doc: LayerDocument, building: Address): readonly CardAction[] {
  const shipped = doc.shipped ?? null;
  const state = doc.state;
  const there = state !== "absent" && !("withheld" in state);
  const layered = shipped === null && (doc.kind === "city" || typeof doc.kind === "object");
  const actions: CardAction[] = [];
  if (there) actions.push({ kind: "open", path: pathOf(doc.at, building) });
  if (layered && state === "absent") actions.push({ kind: "step", step: "own" });
  if (layered && there) actions.push({ kind: "step", step: "inherit" });
  if (shipped !== null && shipped.standing !== "current")
    actions.push({ kind: "step", step: "restore" });
  return actions;
}

export function pathOf(at: Address, building: Address): string {
  const prefix = `${building}/`;
  return at.startsWith(prefix) ? at.slice(prefix.length) : `/${at}`;
}
