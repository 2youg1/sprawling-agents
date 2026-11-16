// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors


import type { ForkEntry } from "./forking";


export interface ForkWire {
  readonly type: "button";
  readonly onmouseenter: () => void;
  readonly onmouseleave: () => void;
  readonly onfocus: () => void;
  readonly onblur: () => void;
  readonly onclick: () => void;
}

export interface ForkButtonLook {
  
  
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
