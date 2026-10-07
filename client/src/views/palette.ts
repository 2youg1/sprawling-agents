// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the palette decides without the page: why a verb cannot run from
// this box, and the box itself - an APG Combobox over the list
// (`palette/rows.ts`) - as the bag `palette.look.svelte` spreads.

import type { Key } from "../core/lang";
import { say } from "../core/lang";
import type { Lang } from "../core/lang";
import { completed } from "../core/completion";
import { boxKey } from "./cursor";
import { rowId } from "./palette/rows";

// What a verb typed here may reach (`SlashHands`): the room the address
// bar names, whether a run is in front of the person or in that room,
// and how many models the city offers.
export interface Reach {
  readonly here: string | null;
  readonly live: boolean;
  readonly models: number;
}

// Why a verb cannot run from this box, as a `lang.json` key. Every
// verb is reachable; the ones missing a capability say which.
export function whyFor(spelling: string, reach: Reach): Key | undefined {
  switch (spelling) {
    case "/dispatch":
    case "/new":
    case "/compact":
    case "/tag":
    case "/untag":
      return reach.here === null ? "palette_needs_room" : undefined;
    // The run in hand, or else the newest run of the room in hand.
    case "/diff":
      return !reach.live && reach.here === null ? "palette_needs_room" : undefined;
    case "/steer":
    case "/stop":
      return reach.live ? undefined : "no_run_in_front";
    case "/model":
      return reach.models === 0 ? "palette_needs_model" : undefined;
    default:
      return undefined;
  }
}

export type KeyPress = Pick<KeyboardEvent, "key" | "preventDefault">;
export interface Typed {
  readonly currentTarget: { readonly value: string };
}

export interface BoxWire {
  readonly role: "combobox";
  readonly "aria-autocomplete": "list";
  readonly "aria-expanded": boolean;
  readonly "aria-controls": string;
  readonly "aria-activedescendant": string | undefined;
  readonly "aria-label": string;
  readonly placeholder: string;
  readonly value: string;
  readonly oninput: (event: Typed) => void;
  readonly onkeydown: (event: KeyPress) => void;
}

export interface BoxState {
  readonly query: string;
  readonly cursor: number;
  // How many rows the cursor walks.
  readonly count: number;
  // The listbox's id.
  readonly list: string;
  readonly lang: Lang;
}

// What only the seat can do: hold the line and the cursor, and take
// the row under the cursor.
export interface BoxHands {
  readonly write: (line: string) => void;
  readonly point: (at: number) => void;
  readonly pick: () => void;
}

// The box. Typing writes the line and puts the cursor back on the
// first row; Tab completes a verb once the line begins with `/`
// (`core/completion.ts`); ↓, ↑ and Enter walk and take the rows
// (`cursor.ts`).
export function boxOf(state: BoxState, hands: BoxHands): BoxWire {
  const { query, cursor, count, list, lang } = state;
  const verbs = query.trim().startsWith("/");
  return {
    role: "combobox",
    "aria-autocomplete": "list",
    "aria-expanded": count > 0,
    "aria-controls": list,
    "aria-activedescendant": count > 0 ? rowId(list, cursor) : undefined,
    "aria-label": say(lang, "nav_palette"),
    placeholder: say(lang, "palette_placeholder"),
    value: query,
    oninput: (event) => {
      hands.write(event.currentTarget.value);
    },
    onkeydown: (event) => {
      if (event.key === "Tab" && verbs) {
        event.preventDefault();
        hands.write(completed(query.trim()));
        return;
      }
      const move = boxKey(event.key, cursor, count);
      switch (move.kind) {
        case "move":
          hands.point(move.to);
          break;
        case "pick":
          hands.pick();
          break;
        case "pass":
          return;
      }
      event.preventDefault();
    },
  };
}
