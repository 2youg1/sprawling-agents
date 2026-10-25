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
import type { View } from "../core/route";
import { buildingOf } from "../core/route";
import { ago } from "../core/time";
import type { Address, Seq, SessionLine } from "../wire";
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

// One session of the room main stands in, as a place the palette goes to
// (client/Spec.lean §4-64b): the room's current session goes to the room,
// an earlier one to its read-only view (`#/talk/<room>:<seq>`), because
// going on from an earlier session is a branch, which a row here does not
// make. The label leads with the number the address line draws, so the
// line a person pressed and the row they pick say the same `#<seq>`.
export interface SessionPlace {
  readonly label: string;
  readonly hint: string;
  readonly to: View;
}

// Where the palette stands: the room, and the earlier session main shows
// in it, if it shows one, whose row says so, because a person who pressed
// the address line of an earlier session looks for the row they are in.
export interface SessionsAt {
  readonly room: Address;
  readonly viewing: Seq | undefined;
}

// How many of the room's sessions the palette lists while nothing is
// typed. `Query::Sessions` answers up to 64, and listed whole they would
// push every page and room off the box; typing filters all of them, so
// `#23` or a word of a name still finds an old one.
export const SESSIONS_AT_REST = 5;

// `lines` newest first, as `Query::Sessions` answers them; the first is
// the room's current session.
export function sessionPlaces(at: SessionsAt, lines: readonly SessionLine[], lang: Lang, now: number): SessionPlace[] {
  return lines.map((line, index) => {
    const named = line.name ?? line.preview ?? "";
    const kind = index === 0 ? "palette_session_current" : line.began === at.viewing ? "palette_session_shown" : "palette_session";
    return {
      label: named === "" ? `#${String(line.began)}` : `#${String(line.began)} · ${named}`,
      hint: index === 0 ? say(lang, kind) : `${say(lang, kind)} · ${ago(lang, line.at, now)}`,
      to: index === 0 ? { kind: "talk", address: at.room } : { kind: "talk", address: at.room, session: line.began },
    };
  });
}

// Carrying the conversation in main into another room (client/Spec.lean
// §4-64b): every room the palette offers, each with the reason it cannot
// take the conversation, so a row says so before it is pressed. The city
// branches a session from any line into any room; what is refused here
// is what would fail there or do something else.
export interface CarryReach {
  // The room main stands in, whose conversation goes.
  readonly here: Address;
  // The rooms the city knows, in the order the palette lists them.
  readonly rooms: readonly Address[];
  // An address typed in the box, offered when the city knows no room by
  // that name: a new room is made by going to it.
  readonly typed: Address | null;
  readonly working: (room: Address) => boolean;
}

export interface CarryPlace {
  readonly to: Address;
  readonly why: Key | undefined;
}

export function carryPlaces(reach: CarryReach): CarryPlace[] {
  const rooms = reach.typed === null || reach.rooms.includes(reach.typed) ? reach.rooms : [...reach.rooms, reach.typed];
  return rooms.map((to) => ({ to, why: carryWhy(reach, to) }));
}

// The same room is a fork from one of its turns, which the thread
// already offers; a building is not a room, and a dispatch there would
// open a room of its own that the branch is not waiting in; a room with
// a run working refuses a new session under it (`E_BUSY`).
function carryWhy(reach: CarryReach, to: Address): Key | undefined {
  if (to === reach.here) return "palette_carry_here";
  if (buildingOf(to) === to) return "palette_carry_building";
  if (reach.working(to)) return "palette_carry_busy";
  return undefined;
}

export type KeyPress = Pick<KeyboardEvent, "key" | "isComposing" | "preventDefault">;
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

// What the box asks for: a place to go, or the room this conversation
// is carried to. The carrying list says the second in the box once, so
// its rows need not say it each (client/Spec.lean 4-64b).
export type Asking = "places" | "carry";

export interface BoxState {
  readonly asking: Asking;
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
  const { asking, query, cursor, count, list, lang } = state;
  const verbs = query.trim().startsWith("/");
  return {
    role: "combobox",
    "aria-autocomplete": "list",
    "aria-expanded": count > 0,
    "aria-controls": list,
    "aria-activedescendant": count > 0 ? rowId(list, cursor) : undefined,
    "aria-label": say(lang, "nav_palette"),
    placeholder: say(lang, asking === "carry" ? "palette_carry" : "palette_placeholder"),
    value: query,
    oninput: (event) => {
      hands.write(event.currentTarget.value);
    },
    onkeydown: (event) => {
      // Tab during a composition is the input method's, as every key
      // in `boxKey` is.
      if (event.key === "Tab" && verbs && !event.isComposing) {
        event.preventDefault();
        hands.write(completed(query.trim()));
        return;
      }
      const move = boxKey(event, cursor, count);
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
