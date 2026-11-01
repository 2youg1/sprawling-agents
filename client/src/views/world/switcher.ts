// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The zen session switcher's keyboard contract and the whole value its
// look draws. The tree itself is core/session_tree.ts; opening a row is
// talk/branching.ts. This file owns neither of those rules.

import { say } from "../../core/lang";
import type { Key, Lang } from "../../core/lang";
import type { WorkspaceView } from "../../core/route";
import { ago } from "../../core/time";
import type { SessionNode } from "../../core/session_tree";
import { sessionTree } from "../../core/session_tree";
import { tailOf } from "../../core/stretches";
import type { Stretch } from "../../core/stretches";
import type { Origin } from "../../wire";
import { titleOf } from "./sessions";
import type { BranchNote } from "../talk/branch_note";

export type SwitchKey = "up" | "down" | "left" | "right" | "enter" | "composing" | "other";

export interface SwitchState {
  readonly cursor: number;
  readonly collapsed: ReadonlySet<string>;
}

// Mirrors client/spec/Views/SessionSwitch.lean. The boolean says whether
// this key asks to open the row under the cursor.
export function answer(
  nodes: readonly Pick<SessionNode, "key" | "children">[],
  state: SwitchState,
  key: SwitchKey,
): readonly [SwitchState, boolean] {
  const node = nodes.at(state.cursor);
  switch (key) {
    case "up":
      return [{ ...state, cursor: Math.max(0, state.cursor - 1) }, false];
    case "down":
      return [
        { ...state, cursor: Math.min(state.cursor + 1, Math.max(0, nodes.length - 1)) },
        false,
      ];
    case "left": {
      if (node?.children !== true) return [state, false];
      return [{ ...state, collapsed: new Set([...state.collapsed, node.key]) }, false];
    }
    case "right": {
      if (node?.children !== true) return [state, false];
      return [
        { ...state, collapsed: new Set([...state.collapsed].filter((key) => key !== node.key)) },
        false,
      ];
    }
    case "enter":
      return [state, node !== undefined];
    case "composing":
    case "other":
      return [state, false];
  }
}

export interface SwitchItem {
  readonly key: string;
  readonly label: string;
  readonly hint: string;
  readonly depth: number;
  readonly children: boolean;
  readonly origin: Origin | null;
  readonly to: WorkspaceView;
  readonly note: BranchNote | null;
  readonly refused: Key | undefined;
}

export function switchItems(
  stretches: readonly Stretch[],
  query: string,
  collapsed: ReadonlySet<string>,
  lang: Lang,
  now: number,
): SwitchItem[] {
  const needle = query.trim().toLowerCase();
  const filtered = stretches.filter((stretch) => {
    const text =
      `${stretch.room} ${stretch.line.name ?? ""} ${stretch.line.preview ?? ""}`.toLowerCase();
    return needle === "" || text.includes(needle);
  });
  return sessionTree(filtered, collapsed).map((node): SwitchItem => {
    const { stretch, depth } = node;
    const title = titleOf(stretch);
    return {
      key: node.key,
      label: `${stretch.room}${stretch.line.name === undefined || stretch.line.name === null || stretch.line.name === "" ? "" : ` · ${title}`}`,
      hint: `${stretch.current ? "" : `${ago(lang, stretch.line.at, now)} · `}#${String(stretch.line.began)}`,
      depth,
      children: node.children,
      origin: stretch.current ? null : tailOf(stretch),
      to: stretch.current
        ? { kind: "talk", address: stretch.room }
        : { kind: "talk", address: stretch.room, session: stretch.line.began },
      note: null,
      refused: undefined,
    };
  });
}

export interface BoxWire {
  readonly role: "combobox";
  readonly "aria-autocomplete": "list";
  readonly "aria-expanded": boolean;
  readonly "aria-controls": string;
  readonly "aria-activedescendant": string | undefined;
  readonly "aria-label": string;
  readonly value: string;
  readonly oninput: (event: { readonly currentTarget: { readonly value: string } }) => void;
  readonly onkeydown: (
    event: Pick<KeyboardEvent, "key" | "isComposing" | "preventDefault">,
  ) => void;
}

export interface TreeWire {
  readonly id: string;
  readonly role: "tree";
  readonly "aria-label": string;
}

export interface SwitchRowWire {
  readonly id: string;
  readonly role: "treeitem";
  readonly "aria-level": number;
  readonly "aria-selected": boolean;
  readonly "aria-expanded": boolean | undefined;
  readonly "aria-disabled": boolean;
  readonly onmouseenter: () => void;
  readonly onclick: () => void;
}

export interface SwitchRow {
  readonly key: string;
  readonly label: string;
  readonly hint: string;
  readonly depth: number;
  readonly wire: SwitchRowWire;
}

export interface SwitcherLook {
  readonly box: BoxWire;
  readonly tree: TreeWire;
  readonly rows: readonly SwitchRow[];
  readonly empty: string | undefined;
}

export interface LookState {
  readonly query: string;
  readonly cursor: number;
  readonly collapsed: ReadonlySet<string>;
  readonly rows: readonly SwitchItem[];
  readonly list: string;
  readonly lang: Lang;
}

export interface LookHands {
  readonly write: (query: string) => void;
  readonly move: (cursor: number) => void;
  readonly fold: (collapsed: ReadonlySet<string>) => void;
  readonly open: (row: SwitchItem) => void;
  readonly hover: (cursor: number) => void;
}

export function switcherLookOf(state: LookState, hands: LookHands): SwitcherLook {
  const { query, cursor, collapsed, rows, list, lang } = state;
  const keyOf = (key: string): SwitchKey => {
    switch (key) {
      case "ArrowUp":
        return "up";
      case "ArrowDown":
        return "down";
      case "ArrowLeft":
        return "left";
      case "ArrowRight":
        return "right";
      case "Enter":
        return "enter";
      default:
        return "other";
    }
  };
  return {
    box: {
      role: "combobox",
      "aria-autocomplete": "list",
      "aria-expanded": rows.length > 0,
      "aria-controls": list,
      "aria-activedescendant": rows.length > 0 ? `${list}-${String(cursor)}` : undefined,
      "aria-label": say(lang, "world_session_filter"),
      value: query,
      oninput: (event) => {
        hands.write(event.currentTarget.value);
      },
      onkeydown: (event) => {
        if (event.isComposing) return;
        const key = keyOf(event.key);
        if (key === "other") return;
        event.preventDefault();
        const [next, open] = answer(rows, { cursor, collapsed }, key);
        if (next.cursor !== cursor) hands.move(next.cursor);
        if (next.collapsed !== collapsed) hands.fold(next.collapsed);
        if (open) {
          const row = rows.at(cursor);
          if (row !== undefined) hands.open(row);
        }
      },
    },
    tree: { id: list, role: "tree", "aria-label": say(lang, "world_session_switcher") },
    rows: rows.map((row, at) => ({
      key: row.key,
      label: row.label,
      hint: row.note?.said ?? row.hint,
      depth: row.depth,
      wire: {
        id: `${list}-${String(at)}`,
        role: "treeitem",
        "aria-level": row.depth + 1,
        "aria-selected": at === cursor,
        "aria-expanded": row.children ? !collapsed.has(row.key) : undefined,
        "aria-disabled": row.refused !== undefined,
        onmouseenter: () => {
          hands.hover(at);
        },
        onclick: () => {
          hands.open(row);
        },
      },
    })),
    empty: rows.length === 0 ? say(lang, "part_no_match") : undefined,
  };
}
