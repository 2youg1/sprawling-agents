// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What this page saw of each reply while it streamed, kept for the life
// of the page so a thread that remounts - a tier switched, a room left
// and come back to - still draws the rhythm it watched (`rhythm.ts`).
//
// A reply is watched under its run while it grows, and filed under the
// turn it became once the Ledger names that turn. A reply whose turn the
// page never learned is dropped: a rhythm drawn on the wrong turn would be
// a fact about another reply.

import type { RunId, Seq } from "../../wire";
import type { Sample } from "./rhythm";

// How many watched replies a page keeps; the oldest goes first. A page
// open for a week holds the same as a page open for an hour.
const KEPT = 200;

// Mutable on purpose: a reply grows by one sample a frame, and copying the
// list on every frame would cost a long reply its square.
const growing: Record<string, Sample[] | undefined> = {};
const order: string[] = [];
let filed = $state<Readonly<Record<string, readonly Sample[]>>>({});

function keyOf(run: RunId, opened: Seq): string {
  return `${run}@${String(opened)}`;
}

// The growing text of `run` was painted at `length` characters at `at`.
// A length that did not grow is not an arrival.
export function heard(run: RunId, length: number, at: number): void {
  const held = growing[run] ?? [];
  if (length <= (held.at(-1)?.length ?? 0)) return;
  held.push({ at, length });
  growing[run] = held;
}

// The reply `run` was streaming became the turn opened at `opened`.
export function landed(run: RunId, opened: Seq): void {
  const held = growing[run];
  growing[run] = undefined;
  if (held === undefined) return;
  const key = keyOf(run, opened);
  order.push(key);
  order.splice(0, Math.max(0, order.length - KEPT));
  filed = Object.fromEntries(Object.entries({ ...filed, [key]: held }).filter(([each]) => order.includes(each)));
}

// The reply stopped and the page never learned which turn it became.
export function lost(run: RunId): void {
  growing[run] = undefined;
}

export function arrivalsOf(run: RunId, opened: Seq): readonly Sample[] | null {
  return filed[keyOf(run, opened)] ?? null;
}
