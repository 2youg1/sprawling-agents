// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// **The client's one reach for browser storage.** Every row the page
// keeps is written and read back through this door, so a browser that
// offers no storage is answered once and every reader above degrades
// the same way. What the rows are called and what their values mean is
// `core/prefs.ts`'s business; this file knows only that a row is a
// name and a string.
//
// **Browser storage refuses by throwing, in three places a person can
// reach**: a profile that denies storage throws on the property access
// itself, a private window can grant a store whose quota is zero, and
// a quota can fill while the tab is open. Each throw is turned into a
// value here, so no reader above has to know that keeping a row is an
// operation that can fail, and the first paint cannot die on one.
//
// **A refusal is not silent.** The row the browser would not take is
// held by this tab instead, and `unkept` names it until it is written
// again with success or removed, so a page that keeps a person's words
// can say they will not outlive the tab (refrain §3-14, the second row;
// client/Spec.lean §4-63).

import { Result } from "effect";
import { readable, writable } from "svelte/store";
import type { Readable } from "svelte/store";

// The little of `Storage` this client needs: a test hands it a map and
// a browser hands it `localStorage`. Narrow on purpose - nothing above
// may enumerate or clear rows it did not write.
export interface Store {
  readonly getItem: (key: string) => string | null;
  readonly setItem: (key: string, value: string) => void;
  readonly removeItem: (key: string) => void;
}

// The door every reader goes through: a store, and the rows of it only
// this tab holds because the browser would not keep them.
export interface Rows extends Store {
  readonly unkept: Readable<ReadonlySet<string>>;
}

const NONE: Readable<ReadonlySet<string>> = readable(new Set());

// A plain table of rows. It stands in for the browser's store where a
// test or the gallery needs one, and holds what the browser refused;
// as a store it never refuses, so it names nothing unkept.
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

// What a browser did when asked, as a value: the result, or nothing
// when the browser refused. A failure that would otherwise be thrown is
// read as data (client/Spec.lean §4-6).
function attempted<T>(act: () => T): T | null {
  return Result.getOrNull(Result.try(act));
}

// The row written and dropped to find out whether this browser keeps
// anything at all. Named like every other row so a store shared with
// another application cannot mistake it for theirs.
const PROBE_KEY = "sprawling.probe";

// The names of the rows this tab alone holds, as one store a page can
// follow.
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

// The browser's own store with its refusals answered: a row the quota
// will not take is kept for this session instead, so a person typing
// into a box never loses the sentence that filled the quota and no
// page dies on a write.
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
      // Both copies are asked to drop the row and neither answer is
      // needed: a store that refuses a removal is one the probe would
      // not have admitted, and the session copy is dropped regardless.
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

// No store at all: the tab holds every row, and every row it holds is
// one the browser did not keep.
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

// The door over the store `reach` returns, decided by writing one row
// and dropping it again. A store that answers the probe is used; a
// store that throws on the reach, or takes nothing, is stood in for by
// a table that lasts as long as the tab.
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

// Where this browser's rows live, and the only reach for that global
// in the whole client. A hardened profile, a document rendered before
// storage is granted, and a test runner all arrive here, and what
// happens to the three of them is decided once.
let reached: Rows | undefined;

export function browserRows(): Rows {
  // One table for every caller, and one probe for the whole session. A
  // fresh one per call would let the shell and a settings panel each
  // write their own preferences into a map the other never reads.
  reached ??= keptRows(() => localStorage);
  return reached;
}
