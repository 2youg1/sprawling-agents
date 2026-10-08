// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the ACP agents page decides before anything is drawn
// (client/Spec.lean §4-67, design H2): whether the one box holds a
// search or a pasted launch spec, which offers a search leaves, and the
// words each consent card and each added agent are drawn with. The
// paste grammar itself is the city's (`Query::ParseAgentSpec`); this
// only decides when to ask it.

import type { Key } from "../../core/lang";
import type { Weight } from "../parts/glyph";
import type { AgentCatalogAnswer, AgentLine, AgentOffer, AgentSource, LoginKind, PinState } from "../../wire";

export type BoxReading =
  | { readonly kind: "search"; readonly typed: string }
  | { readonly kind: "paste"; readonly text: string };

// A box that holds a space inside its text, a line break or a JSON
// object is a launch spec to parse; anything else is a name to search
// for. A registry entry's name may hold a space ("Gemini CLI"), so a
// search that matches some offer by name stays a search.
export function readingOf(text: string, answer: AgentCatalogAnswer | undefined): BoxReading {
  const trimmed = text.trim();
  const shaped = trimmed.startsWith("{") || /\s/.test(trimmed);
  const named = answer !== undefined && offersOf(answer, trimmed).length > 0;
  return shaped && !named ? { kind: "paste", text: trimmed } : { kind: "search", typed: trimmed };
}

// Every offer whose name or id holds the typed text, without case.
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

// The offers a search leaves, with the agents already added taken out:
// an added agent is listed once, in the summary at the top.
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

// The pin word beside a version; an unknown pin says nothing rather
// than guess.
export const PINNED: Readonly<Record<PinState, Key | undefined>> = {
  exact: "acp_pinned",
  floating: "acp_unpinned",
  unknown: undefined,
};

// How an agent says it signs in, in one word; an agent that declares
// no method needs none.
export function loginKey(kinds: readonly LoginKind[]): Key {
  if (kinds.includes("terminal")) return "acp_login_terminal";
  if (kinds.includes("agent")) return "acp_login_agent";
  return "acp_login_none";
}

// Where an added agent stands. A sign-in is asked for only once the
// agent answered that it needs one (ACP's `-32000`).
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

// The program a detected agent was found as: the launch line's first word.
export function programOf(offer: AgentOffer): string {
  return offer.launch_preview.trim().split(/\s+/)[0] ?? "";
}

// Claude's own agent never offers the claude.ai subscription sign-in
// to this city; a person signs that in inside Claude Code itself
// (a ruling, recorded in client/Spec.lean §4-67).
export function signsInElsewhere(line: AgentLine): boolean {
  return line.id === "claude-acp" && line.login_state === "required";
}
