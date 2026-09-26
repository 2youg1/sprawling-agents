// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Which waiting approvals become a browser notification (client-SPEC
// 12-3). Only things that need the person's decision are raised; the
// progress of runs already reaches a hidden tab through its title and
// icon. Four gates, all of which must pass: the window is not focused,
// the page is past its warm-up, the item is new to this page rather
// than part of the first snapshot, and the person is not already
// looking at the address the item waits in.

import type { ApprovalId, ApprovalItem } from "../wire";

// How long after the page opens nothing is raised. A reconnect or a
// second answer in the first seconds replays what was already there,
// and those replays are not news.
export const WARMUP_MS = 5_000;

// The person's switch, kept by this browser alone because a browser
// grants the permission to notify on its own.
export type Notifying = "off" | "on";

export type Focus = "focused" | "blurred";

// What this page has already accounted for. Before the first answer
// there is no snapshot; the first answer becomes it whole.
export type Heard =
  | { readonly snapshot: "pending" }
  | { readonly snapshot: "taken"; readonly ids: ReadonlySet<ApprovalId> };

export const UNHEARD: Heard = { snapshot: "pending" };

export interface Scene {
  readonly notifying: Notifying;
  readonly focus: Focus;
  // Milliseconds since the page opened.
  readonly elapsed: number;
  // The address the person has open, when the page shows one.
  readonly watching: string | null;
}

// The next account of what was heard, and the items to raise now.
// Every item seen is added to the account whichever gate held it, so an
// item passed over once is never raised later.
export function notices(
  heard: Heard,
  items: readonly ApprovalItem[],
  scene: Scene,
): readonly [Heard, readonly ApprovalItem[]] {
  const ids = new Set(items.map((item) => item.id));
  return [{ snapshot: "taken", ids }, []];
}
