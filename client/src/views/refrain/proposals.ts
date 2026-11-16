// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Snippet } from "svelte";

import type {
  Address,
  AxError,
  B3Hash,
  Command,
  ProposalCard,
  ProposalDecision,
  SliceVerdict,
} from "../../wire";

export type Standing =
  { readonly kind: "current" } | { readonly kind: "stale"; readonly now: B3Hash | null };

export function standingOf(card: ProposalCard, version: B3Hash | null): Standing {
  return card.baseline === version ? { kind: "current" } : { kind: "stale", now: version };
}

export interface Take {
  readonly taken: boolean;
  readonly text: string;
}

export type Edit = ReadonlyMap<number, Take>;

export function editOf(card: ProposalCard): Edit {
  return new Map(
    card.slices.flatMap((slice, place) =>
      slice.kind === "same" ? [] : [[place, { taken: true, text: slice.text }]],
    ),
  );
}

export function retaken(edit: Edit, place: number, change: Partial<Take>): Edit {
  const take = edit.get(place);
  return take === undefined ? edit : new Map(edit).set(place, { ...take, ...change });
}

export function takesAny(edit: Edit): boolean {
  return [...edit.values()].some((take) => take.taken);
}

export function verdictsOf(card: ProposalCard, edit: Edit | null): SliceVerdict[] {
  return card.slices.flatMap((slice, place): SliceVerdict[] => {
    const take = edit === null ? { taken: true, text: slice.text } : edit.get(place);
    if (slice.kind === "same" || !take?.taken) return [];
    if (slice.kind === "insert" && take.text !== slice.text) {
      return [{ slice: place, verdict: { amend: { text: take.text } } }];
    }
    return [{ slice: place, verdict: "accept" }];
  });
}

export function rejectionOf(card: ProposalCard): ProposalDecision {
  return { proposal: card.id, verdicts: [] };
}

export type Deciding =
  | { readonly kind: "idle" }
  | { readonly kind: "sent"; readonly command: Command }
  | { readonly kind: "pending"; readonly command: Command }
  | { readonly kind: "refused"; readonly error: AxError };

export type Happened =
  | { readonly kind: "sent"; readonly command: Command }
  | { readonly kind: "lost" }
  | { readonly kind: "relinked" }
  | { readonly kind: "refusal"; readonly error: AxError };

export function advance(deciding: Deciding, happened: Happened): Deciding {
  switch (happened.kind) {
    case "sent":
      return { kind: "sent", command: happened.command };
    case "lost":
      return deciding.kind === "sent" ? { kind: "pending", command: deciding.command } : deciding;
    case "relinked":
      return deciding.kind === "pending" ? { kind: "sent", command: deciding.command } : deciding;
    case "refusal":
      return deciding.kind === "sent" || deciding.kind === "pending"
        ? { kind: "refused", error: happened.error }
        : deciding;
  }
}

const DECIDING_ACTIONS: ReadonlySet<string> = new Set(["decide proposals", "decide a proposal"]);

const NAMES_A_CARD = /\b[0-9a-f]{64}\b/u;

export function refusesCard(error: AxError, doc: Address, card: B3Hash): boolean {
  if (error.subject === doc) return true;
  if (!DECIDING_ACTIONS.has(error.action)) return false;
  return error.subject.includes(card) || !NAMES_A_CARD.test(error.subject);
}

export interface ProposalsLook {
  readonly band: { readonly "aria-labelledby": string };
  readonly heading: { readonly id: string };

  readonly title: string;
  readonly cards: Snippet;
}
