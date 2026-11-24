// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Result } from "effect";
import { readable, writable } from "./store";
import type { Readable } from "./store";

export interface Store {
  readonly getItem: (key: string) => string | null;
  readonly setItem: (key: string, value: string) => void;
  readonly removeItem: (key: string) => void;
}

export interface Rows extends Store {
  readonly unkept: Readable<ReadonlySet<string>>;
}

const NONE: Readable<ReadonlySet<string>> = readable(new Set());

export function memory(): Rows {
  const held = new Map<string, string>();
  return {
    getItem: (key) => held.get(key) ?? null,
    setItem: (key, value) => {
      held.set(key, value);
    },
    removeItem: (key) => {
      held.delete(key);
    },
    unkept: NONE,
  };
}

function attempted<T>(act: () => T): T | null {
  return Result.getOrNull(Result.try(act));
}

interface Unkept {
  readonly store: Readable<ReadonlySet<string>>;
  mark(key: string, held: "tab" | "browser"): void;
}

function unkeptRows(): Unkept {
  const store = writable<ReadonlySet<string>>(new Set());
  return {
    store,
    mark(key, held) {
      store.update((names) => {
        if ((held === "tab") === names.has(key)) return names;
        const next = new Set(names);
        if (held === "tab") next.add(key);
        else next.delete(key);
        return next;
      });
    },
  };
}

function guarded(store: Store): Rows {
  const unkept = unkeptRows();
  const pending = new Map<string, string | null>();

  function change(key: string, value: string | null, keep: () => void): void {
    const stored = attempted(() => {
      keep();
      return true;
    });
    if (stored === null) pending.set(key, value);
    else pending.delete(key);
    unkept.mark(key, stored === null ? "tab" : "browser");
  }

  return {
    getItem: (key) =>
      pending.has(key) ? (pending.get(key) ?? null) : attempted(() => store.getItem(key)),
    setItem: (key, value) => change(key, value, () => store.setItem(key, value)),
    removeItem: (key) => change(key, null, () => store.removeItem(key)),
    unkept: unkept.store,
  };
}

function tabOnly(spare: Rows): Rows {
  const unkept = unkeptRows();
  return {
    getItem: spare.getItem,
    setItem: (key, value) => {
      spare.setItem(key, value);
      unkept.mark(key, "tab");
    },
    removeItem: (key) => {
      spare.removeItem(key);
      unkept.mark(key, "browser");
    },
    unkept: unkept.store,
  };
}

export function keptRows(reach: () => Store): Rows {
  const store = attempted<Store>(reach);
  return store === null ? tabOnly(memory()) : guarded(store);
}

let reached: Rows | undefined;

export function browserRows(): Rows {
  reached ??= keptRows(() => localStorage);
  return reached;
}
