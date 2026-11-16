// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { TimeMs, type Call, type Turn } from "../../wire";

export type Share = "model" | "tool" | "person";
export const SHARES: readonly Share[] = ["model", "tool", "person"];

export interface Stretch {
  readonly share: Share;
  readonly turn: number;
  readonly from: number;
  readonly to: number;
}

export interface Segment {
  readonly share: Share;
  readonly first: number;
  readonly end: number;
}

export interface Rows {
  readonly first: number;
  readonly end: number;
}

export const OVERSCAN = 8;

export function stretchesOf(
  turns: readonly Turn[],
  end: number,
  tail: Share | null,
): readonly Stretch[] {
  return turns.flatMap((turn, at) => {
    const to = Math.max(turn.t, turns[at + 1]?.t ?? end);
    const whole = (share: Share): Stretch[] => [{ share, turn: turn.number, from: turn.t, to }];
    if (at === turns.length - 1 && tail !== null) return whole(tail);
    const waits = turn.notes.flatMap((note) => ("waiting" in note ? [note.waiting] : []));
    const first = waits[0];
    if (first === undefined) return worked(turn, to);
    const within = (moment: number): number => Math.min(to, Math.max(turn.t, moment));
    const since = within(first.t);
    return waited(
      turn,
      { since, until: Math.max(since, within(waits.at(-1)?.answered ?? to)) },
      to,
    );
  });
}

function waited(
  turn: Turn,
  wait: { readonly since: number; readonly until: number },
  to: number,
): Stretch[] {
  const before = { ...turn, calls: turn.calls.filter((call) => call.called < wait.since) };
  const after = {
    ...turn,
    t: TimeMs.make(wait.until),
    calls: turn.calls.filter((call) => call.called >= wait.until),
  };
  const person: Stretch = { share: "person", turn: turn.number, from: wait.since, to: wait.until };
  return [...worked(before, wait.since), person, ...worked(after, to)].filter(
    (part) => part.to > part.from,
  );
}

function worked(turn: Turn, to: number): Stretch[] {
  if (turn.calls.length === 0) return [{ share: "model", turn: turn.number, from: turn.t, to }];
  return cut(turn.number, turn.t, to, turn.calls);
}

function cut(number: number, from: number, to: number, calls: readonly Call[]): Stretch[] {
  let earliest = Number.POSITIVE_INFINITY;
  let latest = Number.NEGATIVE_INFINITY;
  for (const call of calls) {
    earliest = Math.min(earliest, call.called);
    latest = Math.max(latest, call.answered ?? to);
  }
  const first = Math.max(from, earliest);
  const last = Math.min(to, latest);
  const parts: readonly Stretch[] = [
    { share: "model", turn: number, from, to: first },
    { share: "tool", turn: number, from: first, to: Math.max(first, last) },
    { share: "model", turn: number, from: Math.max(first, last), to },
  ];
  return parts.filter((part) => part.to > part.from);
}

export function columnsOf(
  stretches: readonly Stretch[],
  from: number,
  to: number,
  columns: number,
): readonly Segment[] {
  const width = Math.max(1, to - from) / Math.max(1, columns);
  const out: Segment[] = [];
  let open: { share: Share; first: number } | null = null;
  for (let column = 0; column < columns; column += 1) {
    const share = under(stretches, from + (column + 0.5) * width);
    if (open !== null && open.share !== share) {
      out.push({ share: open.share, first: open.first, end: column });
      open = null;
    }
    if (open === null && share !== null) open = { share, first: column };
  }
  if (open !== null) out.push({ share: open.share, first: open.first, end: columns });
  return out;
}

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

export function windowOf(total: number, row: number, top: number, height: number): Rows {
  if (row <= 0) return { first: 0, end: Math.min(total, OVERSCAN) };
  const first = Math.max(0, Math.floor(top / row) - OVERSCAN);
  const end = Math.min(total, Math.ceil((top + height) / row) + OVERSCAN);
  return { first: Math.min(first, end), end };
}

export interface Placed {
  readonly turn: number;
  readonly call: Call;
}

export function callsOf(turns: readonly Turn[]): readonly Placed[] {
  return turns.flatMap((turn) => turn.calls.map((call) => ({ turn: turn.number, call })));
}

export interface Figures {
  readonly input: number;
  readonly output: number;
  readonly cached: number | null;
  readonly usd: number;
}

export function figuresOf(turns: readonly Turn[]): Figures {
  return turns.reduce<Figures>(
    (sum, turn) => ({
      input: sum.input + (turn.used?.input ?? 0),
      output: sum.output + (turn.used?.output ?? 0),
      cached:
        turn.used?.cached === undefined || turn.used.cached === null
          ? sum.cached
          : (sum.cached ?? 0) + turn.used.cached,
      usd: sum.usd + (turn.spent ?? 0),
    }),
    { input: 0, output: 0, cached: null, usd: 0 },
  );
}

export function modelsOf(turns: readonly Turn[]): readonly string[] {
  return [...new Set(turns.flatMap((turn) => turn.model ?? []))];
}
