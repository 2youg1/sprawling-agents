// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { expect, test } from "bun:test";
import { derived, derivedList, get, readable, readonly, writable } from "./store";

test("subscriptions share an active source and release it after the last reader", () => {
  let opened = 0;
  let closed = 0;
  const source = readable(0, (set) => {
    opened += 1;
    set(opened);
    return () => {
      closed += 1;
    };
  });
  const mapped = derived(source, (value) => value * 2);
  expect(opened).toBe(0);
  const calls: number[] = [];
  const a = mapped.subscribe((value) => calls.push(value));
  const b = mapped.subscribe((value) => calls.push(value));
  expect(calls).toEqual([2, 2]);
  a();
  expect(closed).toBe(0);
  b();
  b();
  expect(closed).toBe(1);
  expect(get(mapped)).toBe(4);
  expect(closed).toBe(2);
});

test("a nested write preserves ordering across stores and shared ancestors", () => {
  const root = writable(1);
  const left = derived(root, (value) => value + 1);
  const right = derived(root, (value) => value * 2);
  const log: number[][] = [];
  const stop = derivedList([left, right], (values) => [...values]).subscribe((pair) => {
    log.push(pair);
    if (pair[0] === 3) root.set(3);
  });
  root.set(2);
  stop();
  expect(log).toEqual([
    [2, 2],
    [3, 4],
    [4, 6],
  ]);
  expect(get(derivedList([], (values) => values.length))).toBe(0);
});

test("mutating an object republishes it while repeated primitives and NaN stay quiet", () => {
  const object = { version: 1 };
  const source = writable<number | typeof object>(NaN);
  const seen: (number | typeof object)[] = [];
  const stop = readonly(source).subscribe((value) => seen.push(value));
  source.set(NaN);
  source.set(-0);
  source.set(0);
  source.set(object);
  object.version = 2;
  source.set(object);
  stop();
  expect(seen).toHaveLength(4);
});
