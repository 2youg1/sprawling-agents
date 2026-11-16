// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the versions reading's two lists are given (client D95 shape: a
// seat, a look, and this file): the side compared from and the side
// compared to, each a native list of every version the city knows, and
// the bag that says which list it is and hands a choice back.

// One version in a list. A version the city no longer keeps is listed so
// the history reads whole, and cannot be chosen.
export interface SideOption {
  readonly value: string;
  readonly label: string;
  readonly disabled: boolean;
}

// The bag spread on a list: its accessible name and the hand a choice
// goes back through.
export interface SideWire {
  readonly "aria-label": string;
  readonly onchange: (event: Event & { readonly currentTarget: HTMLSelectElement }) => void;
}

export interface SideLook {
  readonly key: string;
  // The list's visible name, already in the person's language; the same
  // words are its accessible name in the bag.
  readonly label: string;
  readonly held: string;
  readonly options: readonly SideOption[];
  readonly wire: SideWire;
}

export interface PairLook {
  readonly sides: readonly SideLook[];
  // Said under the lists when the city listed only the newest versions.
  readonly more: string | null;
}

export function sideWire(label: string, pick: (value: string) => void): SideWire {
  return {
    "aria-label": label,
    onchange: (event) => {
      pick(event.currentTarget.value);
    },
  };
}
