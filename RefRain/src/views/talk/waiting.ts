// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Snippet } from "svelte";

import type { ApprovalItem, Locator } from "../../wire";

export interface Group {
  readonly key: string;
  readonly first: ApprovalItem;
  readonly items: readonly ApprovalItem[];
}

export function grouped(items: readonly ApprovalItem[]): Group[] {
  const groups = new Map<string, ApprovalItem[]>();
  for (const item of items) {
    const key = item.tainted
      ? `item:${String(item.id)}`
      : `${item.cluster_key.class}:${item.cluster_key.detail}`;
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
    if (first === undefined) continue;
    drawn.push({ key, first, items: sorted });
  }
  return drawn.sort((a, b) => a.first.created - b.first.created);
}

export interface Answering {
  readonly label: string;
  readonly onPress: () => void;
}

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
  readonly asked: Snippet<[Locator]>;
}
