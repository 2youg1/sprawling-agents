// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Which runs each room has held, working or frozen, oldest start first:
// the one index the room's page, its directory, the city's panel and its
// skyline read (client-SPEC 12-5).

import type { Belief, RunBelief } from "./shape";
import { within } from "./live";

function earlier(left: RunBelief, right: RunBelief): number {
  return (left.started ?? 0) - (right.started ?? 0);
}

// The runs of exactly this room, oldest start first.
export function heldIn(belief: Belief, room: string): RunBelief[] {
  return Object.values(belief.runs)
    .filter((run) => run.addr === room)
    .sort(earlier);
}

// The runs of this room and every room below it, oldest start first.
export function heldWithin(belief: Belief, room: string): RunBelief[] {
  return Object.values(belief.runs)
    .filter((run) => within(run, room))
    .sort(earlier);
}
