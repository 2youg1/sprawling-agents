// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the file finder's look is handed (`search.look.svelte`), built
// from the seat's state (`search.svelte`): the title, the box as an APG
// Combobox, the paths as its listbox, and the sentences that say what
// the city could not. Every role, id and handler is decided here, so a
// replacement look draws the same wiring by spreading the bags.

import { fill, say } from "../../core/lang";
import type { Lang } from "../../core/lang";
import type { Address, FindAnswer } from "../../wire";
import { boxKey } from "../cursor";

// What a handler reads from an event, and nothing more, so the wiring
// test presses keys and types without a DOM.
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
  readonly "aria-labelledby": string;
  readonly placeholder: string;
  readonly value: string;
  readonly oninput: (event: Typed) => void;
  readonly onkeydown: (event: KeyPress) => void;
}

export interface ListWire {
  readonly id: string;
  readonly role: "listbox";
  readonly "aria-labelledby": string;
}

export interface OptionWire {
  readonly id: string;
  readonly role: "option";
  readonly "aria-selected": boolean;
  readonly onmouseenter: () => void;
  readonly onclick: () => void;
}

export interface OptionLook {
  // The path, unique in the list; a key for `#each`.
  readonly key: string;
  // The file's own name, then the directory it sits in.
  readonly name: string;
  readonly dir: string;
  readonly active: boolean;
  readonly wire: OptionWire;
}

export interface SearchLook {
  readonly title: { readonly text: string; readonly wire: { readonly id: string } };
  readonly box: BoxWire;
  readonly list: ListWire;
  readonly options: readonly OptionLook[];
  // What the city could not say, one sentence each, in order: nothing
  // matched, and the walk was cut short.
  readonly notes: readonly string[];
}

export interface SearchState {
  readonly under: Address;
  readonly titleId: string;
  readonly uid: string;
  readonly text: string;
  readonly cursor: number;
  readonly found: FindAnswer | undefined;
  readonly lang: Lang;
}

// What only the seat can do: hold the typed text and the cursor, and
// open a path.
export interface SearchHands {
  readonly type: (text: string) => void;
  readonly point: (at: number) => void;
  readonly open: (path: string) => void;
}

// A path drawn as its name, then the directory it sits in.
export function partsOf(path: string): { readonly name: string; readonly dir: string } {
  const slash = path.lastIndexOf("/");
  return slash < 0 ? { name: path, dir: "" } : { name: path.slice(slash + 1), dir: path.slice(0, slash) };
}

export function optionId(uid: string, at: number): string {
  return `${uid}-${String(at)}`;
}

export function searchLookOf(state: SearchState, hands: SearchHands): SearchLook {
  const { uid, titleId, cursor, found, lang } = state;
  const paths = found?.paths ?? [];
  const listId = `${uid}-list`;
  const notes: string[] = [];
  if (found !== undefined && found.text !== "" && paths.length === 0) {
    notes.push(fill(say(lang, "finder_none"), { text: found.text }));
  }
  if (found?.walked === "cut") notes.push(say(lang, "finder_cut"));
  return {
    title: { text: fill(say(lang, "finder_title"), { building: state.under }), wire: { id: titleId } },
    box: {
      role: "combobox",
      "aria-autocomplete": "list",
      "aria-expanded": paths.length > 0,
      "aria-controls": listId,
      "aria-activedescendant": paths.length > 0 ? optionId(uid, cursor) : undefined,
      "aria-labelledby": titleId,
      placeholder: say(lang, "finder_placeholder"),
      value: state.text,
      oninput: (event) => {
        hands.type(event.currentTarget.value);
      },
      onkeydown: (event) => {
        const move = boxKey(event.key, cursor, paths.length);
        switch (move.kind) {
          case "move":
            hands.point(move.to);
            break;
          case "pick": {
            const path = paths[cursor];
            if (path !== undefined) hands.open(path);
            break;
          }
          case "pass":
            return;
        }
        event.preventDefault();
      },
    },
    list: { id: listId, role: "listbox", "aria-labelledby": titleId },
    options: paths.map((path, at) => ({
      key: path,
      ...partsOf(path),
      active: at === cursor,
      wire: {
        id: optionId(uid, at),
        role: "option",
        "aria-selected": at === cursor,
        onmouseenter: () => {
          hands.point(at);
        },
        onclick: () => {
          hands.open(path);
        },
      },
    })),
    notes,
  };
}
