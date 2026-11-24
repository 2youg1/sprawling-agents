// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

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

export type SectionWire =
  | { readonly role: "group"; readonly "aria-labelledby": string }
  | { readonly role: "presentation" };

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
  readonly end:
    | { readonly kind: "chord"; readonly action: Action }
    | { readonly kind: "hint"; readonly text: string };
  readonly active: boolean;
  readonly refused: boolean;
  readonly wire: RowWire;
}

export interface SectionLook {
  readonly key: string;
  readonly heading:
    | {
        readonly text: string;
        readonly wire: { readonly id: string; readonly role: "presentation" };
      }
    | undefined;
  readonly wire: SectionWire;
  readonly rows: readonly RowLook[];
}

export interface RowsLook {
  readonly list: ListWire;
  readonly sections: readonly SectionLook[];
}

export interface RowsState {
  readonly listing: Listing;
  readonly shown: readonly Entry[];
  readonly cursor: number;
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
            heading: {
              text: say(lang, SECTION_WORD[group.section]),
              wire: { id: heading, role: "presentation" },
            },
            wire: { role: "group", "aria-labelledby": heading },
            rows: group.entries.map(rowOf),
          };
        })
      : [
          {
            key: "places",
            heading: undefined,
            wire: { role: "presentation" },
            rows: shown.map(rowOf),
          },
        ];
  return { list: { id, role: "listbox", "aria-label": say(lang, "nav_palette") }, sections };
}

function endOf(entry: Entry, lang: Lang): RowLook["end"] {
  if (entry.why !== undefined) return { kind: "hint", text: say(lang, entry.why) };
  if (entry.keyAction !== undefined) return { kind: "chord", action: entry.keyAction };
  return { kind: "hint", text: entry.hint };
}
