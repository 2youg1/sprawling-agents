// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Every proposal card open in the city, and the documents they are on
// (client/Spec.lean §4-55). The mailbox key counts them and the deciding
// section lists them, so both read this one reading: two counts of the
// same cards would be the first thing to disagree.
//
// The city answers the whole list (`Query::OpenProposals`, wire D15),
// newest offer first, so a card offered before this page opened is
// listed like one offered while it listened. Each card's sentences are
// still asked per document, by the section that draws them.

import { derived } from "svelte/store";
import type { Readable } from "svelte/store";

import { QUERIES } from "../../core/asking";
import type { Asking } from "../../core/asking";
import type { Address, OfferedCard, TimeMs } from "../../wire";

export function openCards(asking: Asking): Readable<readonly OfferedCard[]> {
  return derived(asking.ask(QUERIES.openProposals), (answer) =>
    answer !== undefined && "open_proposals" in answer ? answer.open_proposals.open : [],
  );
}

// One document with open cards, and when its newest card was offered;
// `null` when the city could not say.
export interface Offered {
  readonly doc: Address;
  readonly at: TimeMs | null;
}

// The documents the cards are on, each once, in the order of its newest
// card: the list arrives newest first, so the first card of a document
// is its newest.
export function documentsOf(cards: readonly OfferedCard[]): readonly Offered[] {
  const seen = new Map<Address, TimeMs | null>();
  for (const card of cards) if (!seen.has(card.doc)) seen.set(card.doc, card.at ?? null);
  return [...seen].map(([doc, at]) => ({ doc, at }));
}
