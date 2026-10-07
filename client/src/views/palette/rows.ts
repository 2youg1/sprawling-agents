// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the palette's list look is handed (`rows.look.svelte`): the
// listbox the palette's box controls, its sections, and each row with
// its words, its state and its wiring. The box keeps the focus and walks
// the cursor (`../cursor.ts`); a row is an option the box names as
// active, so a screen reader hears the row the cursor is on. A verb
// this place cannot run stays in the list with `aria-disabled` and its
// reason where the hint goes (ux 7-8).

import type { Action } from "../../core/keys";
import { say } from "../../core/lang";
import type { Lang } from "../../core/lang";
import type { Entry, Listing } from "./entry";
import { SECTION_WORD } from "./sections";

export interface ListWire {
  readonly id: string;
  readonly role: "listbox";
  readonly "aria-label": string;
}

// A section of verbs is a group named by its heading; the places are
// one section with no heading, drawn as the list itself.
export type SectionWire = { readonly role: "group"; readonly "aria-labelledby": string } | { readonly role: "presentation" };

export interface RowWire {
  readonly id: string;
  readonly role: "option";
  readonly "aria-selected": boolean;
  readonly "aria-disabled": boolean;
  readonly onmouseenter: () => void;
  readonly onclick: () => void;
}

export interface RowLook {
  readonly key: string;
  readonly label: string;
  // What stands at the row's end: the chord of the key that reaches
  // the same page, or a short line - the address, the verb's purpose,
  // or the reason it cannot run here.
  readonly end: { readonly kind: "chord"; readonly action: Action } | { readonly kind: "hint"; readonly text: string };
  // The row Enter would take.
  readonly active: boolean;
  // A verb this place cannot run; the cursor may still stand on it, so
  // its reason can be read.
  readonly refused: boolean;
  readonly wire: RowWire;
}

export interface SectionLook {
  readonly key: string;
  readonly heading: { readonly text: string; readonly wire: { readonly id: string; readonly role: "presentation" } } | undefined;
  readonly wire: SectionWire;
  readonly rows: readonly RowLook[];
}

export interface RowsLook {
  readonly list: ListWire;
  readonly sections: readonly SectionLook[];
}

export interface RowsState {
  readonly listing: Listing;
  // The flat list the cursor walks, in the order the rows are drawn.
  readonly shown: readonly Entry[];
  readonly cursor: number;
  // The listbox's id; each row's id is derived from it (`rowId`).
  readonly id: string;
  readonly lang: Lang;
}

export interface RowsHands {
  readonly hover: (at: number) => void;
  readonly pick: (entry: Entry) => void;
}

export function rowId(list: string, at: number): string {
  return `${list}-${String(at)}`;
}

export function rowsLookOf(state: RowsState, hands: RowsHands): RowsLook {
  const { listing, shown, cursor, id, lang } = state;
  const rowOf = (entry: Entry): RowLook => {
    const at = shown.indexOf(entry);
    const refused = entry.why !== undefined;
    return {
      key: entry.label,
      label: entry.label,
      end: endOf(entry, lang),
      active: at === cursor,
      refused,
      wire: {
        id: rowId(id, at),
        role: "option",
        "aria-selected": at === cursor,
        "aria-disabled": refused,
        onmouseenter: () => {
          if (at >= 0) hands.hover(at);
        },
        onclick: () => {
          hands.pick(entry);
        },
      },
    };
  };
  const sections: SectionLook[] =
    listing.kind === "verbs"
      ? listing.groups.map((group) => {
          const heading = `${id}-${group.section}`;
          return {
            key: group.section,
            heading: { text: say(lang, SECTION_WORD[group.section]), wire: { id: heading, role: "presentation" } },
            wire: { role: "group", "aria-labelledby": heading },
            rows: group.entries.map(rowOf),
          };
        })
      : [{ key: "places", heading: undefined, wire: { role: "presentation" }, rows: shown.map(rowOf) }];
  return { list: { id, role: "listbox", "aria-label": say(lang, "nav_palette") }, sections };
}

function endOf(entry: Entry, lang: Lang): RowLook["end"] {
  if (entry.why !== undefined) return { kind: "hint", text: say(lang, entry.why) };
  if (entry.action !== undefined) return { kind: "chord", action: entry.action };
  return { kind: "hint", text: entry.hint };
}
