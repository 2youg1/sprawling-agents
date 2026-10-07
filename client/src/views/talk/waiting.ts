// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The design questions waiting for the person, grouped the way the
// cards show them, and the value the cards' look draws
// (`waiting_cards.look.svelte`).
//
// Identical questions are grouped by their cluster key so one answer
// covers one question; a tainted item is never grouped, because its
// question may say something its neighbours do not.

import type { Snippet } from "svelte";

import type { ApprovalItem, Locator } from "../../wire";

export interface Group {
  readonly key: string;
  readonly first: ApprovalItem;
  readonly items: readonly ApprovalItem[];
}

// The groups, each in the order its questions arrived and all of them
// in the order their first question arrived.
export function grouped(items: readonly ApprovalItem[]): Group[] {
  const groups = new Map<string, ApprovalItem[]>();
  for (const item of items) {
    const key = item.tainted ? `item:${String(item.id)}` : `${item.cluster_key.class}:${item.cluster_key.detail}`;
    const held = groups.get(key);
    if (held === undefined) {
      groups.set(key, [item]);
    } else {
      held.push(item);
    }
  }
  const drawn: Group[] = [];
  for (const [key, held] of groups) {
    const sorted = [...held].sort((a, b) => a.created - b.created);
    const first = sorted[0];
    // Every group holds the item that created it; an empty one is
    // drawn as nothing rather than as a card with no question.
    if (first === undefined) continue;
    drawn.push({ key, first, items: sorted });
  }
  return drawn.sort((a, b) => a.first.created - b.first.created);
}

// One answer on a card: its words and what pressing it sends.
export interface Answering {
  readonly label: string;
  readonly onPress: () => void;
}

// One card: who asks and when, the two answers, the question, what it
// is about, what denying keeps and where a file comes back from, and
// the two marks a group may carry - how many questions one answer
// covers, and that the question is tainted.
export interface WaitingCardLook {
  readonly key: string;
  readonly asker: string;
  readonly at: string;
  readonly allow: Answering;
  readonly deny: Answering;
  readonly action: string;
  readonly locator: Locator;
  readonly keeps: string;
  readonly bin: { readonly text: string; readonly href: string };
  readonly same: string | undefined;
  readonly tainted: string | undefined;
}

export interface WaitingLook {
  readonly cards: readonly WaitingCardLook[];
  // What the question is about, drawn by the seat that asks for it.
  readonly asked: Snippet<[Locator]>;
}
