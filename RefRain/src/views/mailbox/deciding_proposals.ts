// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors


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


export interface Offered {
  readonly doc: Address;
  readonly at: TimeMs | null;
}


export function documentsOf(cards: readonly OfferedCard[]): readonly Offered[] {
  const seen = new Map<Address, TimeMs | null>();
  for (const card of cards) if (!seen.has(card.doc)) seen.set(card.doc, card.at ?? null);
  return [...seen].map(([doc, at]) => ({ doc, at }));
}
