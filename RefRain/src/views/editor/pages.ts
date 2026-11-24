// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Answered } from "../../core/answered";
import type { Asking } from "../../core/asking";
import { get, readonly, writable } from "../../core/store";
import type { Answer, Query } from "../../wire";

interface Page<T, Piece> {
  query: (value: T) => Query | null;
  read: (answer: Answer | undefined) => Answered<Piece>;
  append: (held: T, piece: Piece) => T;
}

interface Window<T> {
  readonly value: T | null;
  readonly lost: string | null;
}

export function pages<T, Piece>(asking: Asking, page: Page<T, Piece>) {
  const window = writable<Window<T>>({ value: null, lost: null });
  let stop: (() => void) | undefined;
  let generation = 0;

  function clear(): void {
    generation += 1;
    stop?.();
    stop = undefined;
  }

  function ask(value: T): void {
    clear();
    const query = page.query(value);
    if (query === null) return;
    const wanted = generation;
    const release = asking.ask(query).subscribe((answer) => {
      if (generation !== wanted) return;
      const piece = page.read(answer);
      if (piece.kind === "unavailable") {
        clear();
        window.set({ value, lost: piece.query });
      } else if (piece.kind === "held") {
        const next = page.append(value, piece.value);
        if (next === value) return;
        clear();
        const following = generation;
        window.set({ value: next, lost: null });
        queueMicrotask(() => {
          if (generation === following) ask(next);
        });
      }
    });
    if (generation === wanted) stop = release;
    else release();
  }

  return {
    state: readonly(window),
    current: () => get(window),
    start(value: T | null) {
      clear();
      window.set({ value, lost: null });
      if (value !== null) ask(value);
    },
    close: clear,
  };
}
