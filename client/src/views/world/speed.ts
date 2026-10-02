// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the speed cell of the session's sheet says (client/Spec.lean §7K): the
// median time to first content over the turns that measured one, and
// how many turns that is.
//
// A median rather than a mean, because one turn that waited on a cold
// provider would otherwise move the figure a person compares sessions
// by. A turn whose times are not measurements (`Turn.timing`), or that
// streamed nothing, has no figure and is left out rather than counted
// as zero.

import type { Turn } from "../../wire";
import { ttftOf } from "../talk/timing";

export interface Speed {
  readonly ttft: number;
  readonly turns: number;
}

export function speedOf(turns: readonly Turn[]): Speed | null {
  const measured = turns.flatMap((turn) => {
    const ms = ttftOf(turn);
    return ms === null ? [] : [ms];
  });
  const sorted = [...measured].sort((a, b) => a - b);
  const middle = Math.floor(sorted.length / 2);
  const upper = sorted[middle];
  if (upper === undefined) return null;
  const lower = sorted.length % 2 === 0 ? (sorted[middle - 1] ?? upper) : upper;
  return { ttft: Math.round((lower + upper) / 2), turns: sorted.length };
}
