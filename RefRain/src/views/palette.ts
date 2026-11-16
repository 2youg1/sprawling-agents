// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Key } from "../core/lang";
import { fill, say } from "../core/lang";
import type { Lang } from "../core/lang";
import { completed } from "../core/completion";
import type { View } from "../core/route";
import { buildingOf, readWorkspace, toFragment, workspaceDestination } from "../core/route";
import { sessionTree } from "../core/session_tree";
import { stretchesOf, tailOf } from "../core/stretches";
import type { BuildingProgress } from "../wire";
import { ago } from "../core/time";
import type { Address, Origin, Seq, SessionLine } from "../wire";
import { boxKey } from "./cursor";
import { rowId } from "./palette/rows";

export function paletteWorkspace(
  query: string,
  buildings: readonly BuildingProgress[],
): View | null {
  const view = readWorkspace(query);
  if (view === null) return null;
  const destination = workspaceDestination(view, buildings);
  return destination?.kind === "raise" ? null : destination;
}

export function workspaceProblem(
  query: string,
  buildings: readonly BuildingProgress[],
): Key | null {
  if (query.trim() === "") return null;
  if (readWorkspace(query) === null) return "palette_address_expected";
  return paletteWorkspace(query, buildings) === null ? "palette_building_missing" : null;
}

export function directEntry(
  query: string,
  buildings: readonly BuildingProgress[],
  lang: Lang,
): {
  readonly label: string;
  readonly hint: string;
  readonly to: View;
} | null {
  const to = paletteWorkspace(query, buildings);
  if (to === null) return null;
  return {
    label: fill(say(lang, "palette_go_address"), { address: readWorkspace(query)?.address ?? "" }),
    hint: toFragment(to),
    to,
  };
}

export interface Reach {
  readonly here: string | null;
  readonly live: boolean;
  readonly models: number;
}

export function whyFor(spelling: string, reach: Reach): Key | undefined {
  switch (spelling) {
    case "/dispatch":
    case "/new":
    case "/compact":
    case "/tag":
    case "/untag":
      return reach.here === null ? "palette_needs_room" : undefined;

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

export interface SessionPlace {
  readonly label: string;
  readonly hint: string;
  readonly to: View;
  readonly origin: Origin | null;
}

export interface SessionsAt {
  readonly room: Address;
  readonly viewing: Seq | undefined;
}

export const SESSIONS_AT_REST = 5;

export function sessionPlaces(
  at: SessionsAt,
  lines: readonly SessionLine[],
  lang: Lang,
  now: number,
): SessionPlace[] {
  const stretches = stretchesOf([{ room: at.room, sessions: lines, earlier: 0 }], () => []);
  return sessionTree(stretches, new Set()).map((node) => {
    const { stretch, depth } = node;
    const line = stretch.line;
    const named = line.name ?? line.preview ?? "";
    const kind = stretch.current
      ? "palette_session_current"
      : line.began === at.viewing
        ? "palette_session_shown"
        : "palette_session";
    const label = named === "" ? `#${String(line.began)}` : `#${String(line.began)} · ${named}`;
    return {
      label: `${"  ".repeat(depth)}${label}`,
      hint: stretch.current ? say(lang, kind) : `${say(lang, kind)} · ${ago(lang, line.at, now)}`,
      to: stretch.current
        ? { kind: "talk", address: at.room }
        : { kind: "talk", address: at.room, session: line.began },
      origin: stretch.current ? null : tailOf(stretch),
    };
  });
}

export interface CarryReach {
  readonly here: Address;

  readonly rooms: readonly Address[];

  readonly typed: Address | null;
  readonly working: (room: Address) => boolean;
}

export interface CarryPlace {
  readonly to: Address;
  readonly why: Key | undefined;
}

export function carryPlaces(reach: CarryReach): CarryPlace[] {
  const rooms =
    reach.typed === null || reach.rooms.includes(reach.typed)
      ? reach.rooms
      : [...reach.rooms, reach.typed];
  return rooms.map((to) => ({ to, why: carryWhy(reach, to) }));
}

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

export type Asking = "places" | "carry";

export interface BoxState {
  readonly asking: Asking;
  readonly query: string;
  readonly cursor: number;

  readonly count: number;

  readonly list: string;
  readonly lang: Lang;
}

export interface BoxHands {
  readonly write: (line: string) => void;
  readonly point: (at: number) => void;
  readonly pick: () => void;
}

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
