// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The run page's time lens, decided before it is drawn: each turn as a
// stretch of the run's clock, the lanes those stretches fall into, and
// how many boxes and rows the page may hold for them.
//
// **A turn is the finest clock the wire carries.** `Turn.t` is when the
// model was asked; a tool call and a wait for approval carry a `Seq`
// and no time, so a turn is drawn in the lane of what it came to - it
// waited for the person, it called tools, or the model only spoke -
// rather than cut into parts whose lengths nobody measured.
//
// **The page holds a bounded number of boxes.** A lane is sampled once
// per column and run-length encoded, so ten thousand turns across four
// hundred columns are at most four hundred boxes; the call list draws
// the rows in view plus `OVERSCAN` on each side.

import type { Call, Turn } from "../../wire";

export type Share = "model" | "tool" | "person";
export const SHARES: readonly Share[] = ["model", "tool", "person"];

export interface Stretch {
  readonly share: Share;
  readonly turn: number;
  readonly from: number;
  readonly to: number;
}

// Columns `first` up to but not including `end`, all in one share.
export interface Segment {
  readonly share: Share;
  readonly first: number;
  readonly end: number;
}

// Rows `first` up to but not including `end`.
export interface Rows {
  readonly first: number;
  readonly end: number;
}

// Rows drawn beyond each edge of the view, so a scroll of a few rows
// shows rows already laid out rather than a blank band.
export const OVERSCAN = 8;

// `tail` is what a live run is doing now, which decides its last
// stretch; a run that is over passes `null`.
export function stretchesOf(turns: readonly Turn[], end: number, tail: Share | null): readonly Stretch[] {
  return turns.map((turn, at) => ({
    share: at === turns.length - 1 && tail !== null ? tail : "model",
    turn: turn.number,
    from: turn.t,
    to: Math.max(turn.t, turns[at + 1]?.t ?? end),
  }));
}

// The stretches as boxes over `columns` columns spanning `from`..`to`:
// each column takes the stretch under its middle, and neighbouring
// columns of one share become one box. `stretches` is in time order.
export function columnsOf(stretches: readonly Stretch[], from: number, to: number, columns: number): readonly Segment[] {
  const width = Math.max(1, to - from) / Math.max(1, columns);
  return stretches.map((each) => ({
    share: under(stretches, each.from) ?? each.share,
    first: Math.floor((each.from - from) / width),
    end: Math.ceil((each.to - from) / width),
  }));
}

// The share of the last stretch that began at or before `moment`, when
// it has not yet ended; a binary search, because a lane asks once per
// column of a run that may have ten thousand turns.
function under(stretches: readonly Stretch[], moment: number): Share | null {
  let low = 0;
  let high = stretches.length;
  while (low < high) {
    const middle = (low + high) >>> 1;
    if ((stretches[middle]?.from ?? Infinity) <= moment) low = middle + 1;
    else high = middle;
  }
  const found = stretches[low - 1];
  return found !== undefined && moment < found.to ? found.share : null;
}

// The rows of a list `total` long, each `row` pixels tall, that meet a
// view `height` pixels tall scrolled `top` pixels down.
export function windowOf(total: number, row: number, top: number, height: number): Rows {
  return { first: 0, end: row + top + height > 0 ? total : 0 };
}

export interface Placed {
  readonly turn: number;
  readonly call: Call;
}

export function callsOf(turns: readonly Turn[]): readonly Placed[] {
  return turns.flatMap((turn) => turn.calls.map((call) => ({ turn: turn.number, call })));
}

// What the whole run came to, summed over its turns.
export interface Figures {
  readonly input: number;
  readonly output: number;
  readonly cached: number;
  readonly usd: number;
}

export function figuresOf(turns: readonly Turn[]): Figures {
  return turns.reduce(
    (sum, turn) => ({
      input: sum.input + (turn.used?.input ?? 0),
      output: sum.output + (turn.used?.output ?? 0),
      cached: sum.cached + (turn.used?.cached ?? 0),
      usd: sum.usd + (turn.spent ?? 0),
    }),
    { input: 0, output: 0, cached: 0, usd: 0 },
  );
}
