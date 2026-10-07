// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the branch action's look is given, and the wiring the seat builds
// for it (client/Spec.lean D95 shape: a seat, a look, and this file).
//
// The bag reports the entry under the hand - entered or focused, and
// cleared as the hand leaves - which is the one the `fork.here` chord
// branches from, and a press branches from the entry it stands on.
// Clearing matters: a chord pressed pages away must never reach a stale
// entry.

import type { ForkEntry } from "./forking";

// The bag spread on the button. The hint's id is not in it: `Tip` mints
// that id and hands it to the look, which writes `aria-describedby`.
export interface ForkWire {
  readonly type: "button";
  readonly onmouseenter: () => void;
  readonly onmouseleave: () => void;
  readonly onfocus: () => void;
  readonly onblur: () => void;
  readonly onclick: () => void;
}

export interface ForkButtonLook {
  // Both already in the person's language: the action's name beside its
  // mark, and the hint that says what a branch keeps.
  readonly label: string;
  readonly hint: string;
  readonly wire: ForkWire;
}

export interface ForkHands {
  readonly pick: (entry: ForkEntry) => void;
  readonly hover: (entry: ForkEntry | null) => void;
}

export function forkLookOf(
  entry: ForkEntry,
  words: { readonly label: string; readonly hint: string },
  hands: ForkHands,
): ForkButtonLook {
  return {
    ...words,
    wire: {
      type: "button",
      onmouseenter: () => {
        hands.hover(entry);
      },
      onmouseleave: () => {
        hands.hover(null);
      },
      onfocus: () => {
        hands.hover(entry);
      },
      onblur: () => {
        hands.hover(null);
      },
      onclick: () => {
        hands.pick(entry);
      },
    },
  };
}
