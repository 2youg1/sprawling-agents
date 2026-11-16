// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Key, Lang } from "../../core/lang";
import { calendarDay } from "../../core/time";
import type { Weight } from "../parts/glyph";
import type {
  AgentCatalogAnswer,
  AgentLine,
  AgentOffer,
  AgentSource,
  LoginKind,
  PinState,
} from "../../wire";

export type BoxReading =
  | { readonly kind: "search"; readonly typed: string }
  | { readonly kind: "paste"; readonly text: string };

export function readingOf(text: string, answer: AgentCatalogAnswer | undefined): BoxReading {
  const trimmed = text.trim();
  const shaped = trimmed.startsWith("{") || /\s/.test(trimmed);
  const named = answer !== undefined && offersOf(answer, trimmed).length > 0;
  return shaped && !named ? { kind: "paste", text: trimmed } : { kind: "search", typed: trimmed };
}

function offersOf(answer: AgentCatalogAnswer, typed: string): readonly AgentOffer[] {
  return [...answer.detected, ...answer.catalog].filter((offer) => matches(offer, typed));
}

function matches(offer: AgentOffer, typed: string): boolean {
  const folded = typed.toLowerCase();
  return offer.name.toLowerCase().includes(folded) || offer.id.toLowerCase().includes(folded);
}

export interface Shown {
  readonly detected: readonly AgentOffer[];
  readonly catalog: readonly AgentOffer[];
}

export function shownOf(answer: AgentCatalogAnswer, typed: string): Shown {
  const added = new Set(answer.added.map((line) => line.id));
  const left = (offer: AgentOffer) => !added.has(offer.id) && matches(offer, typed);
  return { detected: answer.detected.filter(left), catalog: answer.catalog.filter(left) };
}

export const SOURCE: Readonly<Record<AgentSource, Key>> = {
  registry: "acp_source_registry",
  detected: "acp_source_detected",
  pasted: "acp_source_pasted",
};

export const PINNED: Readonly<Record<PinState, Key | undefined>> = {
  exact: "acp_pinned",
  floating: "acp_unpinned",
  unknown: undefined,
};

export function snapshotDay(lang: Lang, date: string): string {
  const at = Date.parse(date);
  return Number.isNaN(at) ? date : calendarDay(lang, at);
}

export function loginKey(kinds: readonly LoginKind[]): Key {
  if (kinds.includes("terminal")) return "acp_login_terminal";
  if (kinds.includes("agent")) return "acp_login_agent";
  return "acp_login_unknown";
}

export function standingOf(line: AgentLine): { readonly key: Key; readonly weight: Weight } {
  switch (line.login_state) {
    case "required":
      return { key: "acp_needs_login", weight: "alert" };
    case "ready":
      return { key: "acp_signed_in", weight: "live" };
    case "unasked":
      return { key: "acp_ready", weight: "quiet" };
  }
}

export function programOf(offer: AgentOffer): string {
  return offer.launch_preview.trim().split(/\s+/)[0] ?? "";
}

export function signsInElsewhere(line: AgentLine): boolean {
  return line.id === "claude-acp" && line.login_state === "required";
}
