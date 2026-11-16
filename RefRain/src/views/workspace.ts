// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Where each region of the conversation page stands on the shell grid
// (docs/frontend-method.md §4-33, §7H, §7K, client D113). Pure so the
// layout can be judged without mounting a component.

import type { Tier } from "../core/prefs";
import type { Pane, Workbench } from "../core/workbench";

export type Lines = readonly [number, number];

export interface Placed {
  readonly pane: Pane;
  readonly lines: Lines;
}

export interface Layout {
  readonly world: "none" | "beside" | "sheet";
  readonly panes: readonly Placed[];
  readonly talk: Lines;
  readonly right: Lines | null;
}

export type RightSide = "open" | "closed";

const MIDDLE: Lines = [4, 10];
const LEFT: Lines = [1, 4];
const RIGHT: Lines = [10, 13];

const RIGHT_FROM = 8;
const RIGHT_OPEN: Lines = [RIGHT_FROM, 13];
const TALK_BESIDE_RIGHT: Lines = [2, RIGHT_FROM];

const SIDES: readonly Placed[] = [
  { pane: "sessions", lines: LEFT },
  { pane: "commits", lines: RIGHT },
];

export function layoutOf(tier: Tier, right: RightSide, _bench: Workbench): Layout {
  if (right === "open") {
    return { world: "none", panes: [], talk: TALK_BESIDE_RIGHT, right: RIGHT_OPEN };
  }
  switch (tier) {
    case "zen":
      return { world: "none", panes: [], talk: MIDDLE, right: null };
    case "blend":
    case "panorama":
      return { world: "beside", panes: SIDES, talk: MIDDLE, right: null };
  }
}

export function oneColumnOf(tier: Tier, bench: Workbench): Layout {
  return tier === "panorama"
    ? {
        world: "sheet",
        panes: bench.map((column) => ({ pane: column.pane, lines: [1, 13] })),
        talk: [1, 13],
        right: null,
      }
    : { world: "none", panes: [], talk: [1, 13], right: null };
}

export function gridColumn(lines: Lines): string {
  return `${String(lines[0])} / ${String(lines[1])}`;
}
