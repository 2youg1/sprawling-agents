// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, createSignal, from, onCleanup } from "solid-js";
import type { Accessor, Setter } from "solid-js";

import type { Readable } from "./store";

export function storeValue<T>(source: Readable<T> | Accessor<Readable<T>>): Accessor<T>;
export function storeValue<T>(
  source: Accessor<Readable<T> | null | undefined>,
): Accessor<T | undefined>;
export function storeValue<T>(
  source: Readable<T> | Accessor<Readable<T> | null | undefined>,
): Accessor<T | undefined> {
  if (typeof source === "function") {
    return createMemo<T | undefined>(() => {
      const store = source();
      return store == null ? undefined : from<T>((next) => store.subscribe(next));
    });
  }
  let initial!: T;
  const feed: { setter?: Setter<{ value: T }> } = {};
  const stop = source.subscribe((value) => {
    if (feed.setter === undefined) initial = value;
    else feed.setter({ value });
  });
  const [value, set] = createSignal({ value: initial });
  feed.setter = set;
  onCleanup(stop);
  return () => value().value;
}
