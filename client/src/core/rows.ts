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

import { Effect, Either } from "effect";

// The little of `Storage` this client needs: a test hands it a map and
// a browser hands it `localStorage`. Narrow on purpose - nothing above
// may enumerate or clear rows it did not write.
export interface Rows {
  readonly getItem: (key: string) => string | null;
  readonly setItem: (key: string, value: string) => void;
  readonly removeItem: (key: string) => void;
}

// What a browser without storage remembers: this session, and no
// longer. A choice still takes effect; it just does not outlive the
// tab.
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
  };
}

// What a browser did when asked, as a value: the result, or nothing
// when the browser refused. This is the client's second use of Effect
// at run time and it is the same use as the first - a failure that
// would otherwise be thrown is read as data (client-SPEC 4-6).
function attempted<T>(act: () => T): T | null {
  const ran = Effect.runSync(Effect.either(Effect.try(act)));
  return Either.isRight(ran) ? ran.right : null;
}

// The row written and dropped to find out whether this browser keeps
// anything at all. Named like every other row so a store shared with
// another application cannot mistake it for theirs.
const PROBE_KEY = "sprawling.probe";

// The browser's own store with its refusals answered: a row the quota
// will not take is kept for this session instead, so a person typing
// into a box never loses the sentence that filled the quota and no
// page dies on a write.
function guarded(store: Rows, spare: Rows): Rows {
  return {
    getItem: (key) => attempted(() => store.getItem(key)) ?? spare.getItem(key),
    setItem: (key, value) => {
      const took = attempted(() => {
        store.setItem(key, value);
        return true;
      });
      if (took === null) {
        spare.setItem(key, value);
      }
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
    },
  };
}

// Whether this browser gives the client a place to keep rows, decided
// by writing one and dropping it again. A store that answers the probe
// is used; a store that throws on the reach, or takes nothing, is
// stood in for by a map that lasts as long as the tab.
function decided(): Rows {
  const spare = memory();
  const store = attempted<Rows>(() => localStorage);
  if (store === null) {
    return spare;
  }
  const kept = attempted(() => {
    store.setItem(PROBE_KEY, PROBE_KEY);
    store.removeItem(PROBE_KEY);
    return true;
  });
  return kept === null ? spare : guarded(store, spare);
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
  reached ??= decided();
  return reached;
}
