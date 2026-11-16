// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What a fact that can be followed is given (client D95: the building's
// sections are the seats, `fact_link.look.svelte` draws the fact): a
// run's id, a commit's oid or a room's name standing in a line of facts,
// which either goes to that thing's page or, for a commit's parent,
// opens the parent's facts in place.

// Whether the fact is a figure (an id, set in the figure face so a column
// of them lines up) or words, and whether it is the line's main fact or
// one of its quieter ones.
export type FactFace = "figure" | "words";
export type FactInk = "text" | "quiet";

// A link to the fact's page, or a press that shows or hides the fact in
// place; only the second says whether it stands open, and only while it
// is something that opens.
export type FactWire =
  | { readonly href: string }
  | {
      readonly type: "button";
      readonly "aria-expanded": boolean | undefined;
      readonly onclick: () => void;
    };

export interface FactLinkLook {
  // Already in the person's language, or an id shortened for reading.
  readonly label: string;
  readonly face: FactFace;
  readonly ink: FactInk;
  readonly wire: FactWire;
}
