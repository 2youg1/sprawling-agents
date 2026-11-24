// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { get, readonly, writable } from "../../core/store";

import type { RunId, Seq } from "../../wire";
import type { Sample } from "./rhythm";

const KEPT = 200;

const growing: Record<string, Sample[] | undefined> = {};
const order: string[] = [];
const filed = writable<Readonly<Record<string, readonly Sample[]>>>({});
export const filedArrivals = readonly(filed);

function keyOf(run: RunId, opened: Seq): string {
  return `${run}@${String(opened)}`;
}

export function heard(run: RunId, length: number, at: number): void {
  const held = growing[run] ?? [];
  if (length <= (held.at(-1)?.length ?? 0)) return;
  held.push({ at, length });
  growing[run] = held;
}

export function landed(run: RunId, opened: Seq): void {
  const held = growing[run];
  growing[run] = undefined;
  if (held === undefined) return;
  const key = keyOf(run, opened);
  order.push(key);
  order.splice(0, Math.max(0, order.length - KEPT));
  filed.update((previous) =>
    Object.fromEntries(
      Object.entries({ ...previous, [key]: held }).filter(([each]) => order.includes(each)),
    ),
  );
}

export function lost(run: RunId): void {
  growing[run] = undefined;
}

export function arrivalsOf(run: RunId, opened: Seq, file = get(filed)): readonly Sample[] | null {
  return file[keyOf(run, opened)] ?? null;
}
