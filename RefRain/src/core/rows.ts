// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors


import { Result } from "effect";
import { readable, writable } from "svelte/store";
import type { Readable } from "svelte/store";


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


const PROBE_KEY = "sprawling.probe";


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


function guarded(store: Store, spare: Rows): Rows {
  const unkept = unkeptRows();
  return {
    getItem: (key) => attempted(() => store.getItem(key)) ?? spare.getItem(key),
    setItem: (key, value) => {
      const took = attempted(() => {
        store.setItem(key, value);
        return true;
      });
      if (took === null) {
        spare.setItem(key, value);
      } else {
        spare.removeItem(key);
      }
      unkept.mark(key, took === null ? "tab" : "browser");
    },
    removeItem: (key) => {
      
      
      attempted(() => {
        store.removeItem(key);
        return true;
      });
      spare.removeItem(key);
      unkept.mark(key, "browser");
    },
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
  const spare = memory();
  const store = attempted<Store>(reach);
  if (store === null) {
    return tabOnly(spare);
  }
  const kept = attempted(() => {
    store.setItem(PROBE_KEY, PROBE_KEY);
    store.removeItem(PROBE_KEY);
    return true;
  });
  return kept === null ? tabOnly(spare) : guarded(store, spare);
}


let reached: Rows | undefined;

export function browserRows(): Rows {
  
  
  reached ??= keptRows(() => localStorage);
  return reached;
}
