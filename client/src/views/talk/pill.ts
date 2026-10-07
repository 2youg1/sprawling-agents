// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What a pill under the box decides before anything is drawn: which
// rows a filter keeps, where a key moves the cursor, which edge the
// menu hangs from, and the whole value its look draws. The pill is a
// seat and a look (client D95): `pill.svelte` holds the state and the
// drawn elements, `pill.look.svelte` draws, and `lookOf` builds the
// value between them, every role, `aria-*` value and key handler of
// client/Spec.lean §4-60 inside a wire bag the look spreads unchanged.
// Nothing here touches the DOM, so `bun test` reads it without a
// compiled component.

import type { Snippet } from "svelte";
import { createAttachmentKey } from "svelte/attachments";
import type { Attachment } from "svelte/attachments";

import type { Choice, Pill } from "./composer";
import { FILTER_AFTER } from "./composer";

// One key for every element a seat holds. Svelte keeps one attachment
// per element under each symbol key, and each bag is spread on its own
// element, so one key is enough for every bag of the settings row.
export const HOLD = createAttachmentKey();

// What a key handler reads from the event, and nothing more, so a
// wiring test presses a key without a DOM.
export type KeyPress = Pick<KeyboardEvent, "key" | "preventDefault" | "stopPropagation">;

// Where focus went when it left an element, read the same way.
export interface Left {
  readonly relatedTarget: EventTarget | null;
}

// The bag spread on a fact of the settings row that opens a menu: the
// pill's trigger, the model entry and the permissions entry. The
// accessible name says which fact it is before its value, because the
// drawn face shows the value alone.
export interface TriggerWire {
  readonly "aria-label": string;
  readonly "aria-haspopup": "listbox" | "dialog";
  readonly "aria-expanded": boolean;
  readonly "aria-controls"?: string | undefined;
  readonly onclick: () => void;
  readonly [hold: symbol]: Attachment<HTMLElement>;
}

// The bag spread on the element holding a trigger and its menu: focus
// leaving it closes the menu without taking focus back.
export interface FrameWire {
  readonly onfocusout: (event: Left) => void;
  readonly [hold: symbol]: Attachment<HTMLElement>;
}

// Which edge of the trigger the menu hangs from.
export type Edge = "left" | "right";

// The bag spread on the filter box of a long list (client/Spec.lean
// §7-4, the combobox): it keeps the focus and points into the list.
export interface FilterWire {
  readonly role: "combobox";
  readonly "aria-label": string;
  readonly "aria-expanded": true;
  readonly "aria-controls": string;
  readonly "aria-describedby": string | undefined;
  readonly "aria-activedescendant": string | undefined;
  readonly placeholder: string;
  readonly value: string;
  readonly oninput: (event: { readonly currentTarget: { readonly value: string } }) => void;
  readonly onkeydown: (event: KeyPress) => void;
  readonly [hold: symbol]: Attachment<HTMLElement>;
}

// The bag spread on the list. It holds the focus when there is no
// filter, so the cursor is its `aria-activedescendant`; a press on a
// row keeps the focus where it is.
export interface ListWire {
  readonly id: string;
  readonly role: "listbox";
  readonly tabindex: 0 | -1;
  readonly "aria-label": string;
  readonly "aria-describedby": string | undefined;
  readonly "aria-activedescendant": string | undefined;
  readonly onkeydown: (event: KeyPress) => void;
  readonly onmousedown: (event: Pick<MouseEvent, "preventDefault">) => void;
  readonly [hold: symbol]: Attachment<HTMLElement>;
}

// The bag spread on one row. A row is picked by pointer; its keys
// belong to the list, which holds the focus.
export interface OptionWire {
  readonly id: string;
  readonly role: "option";
  readonly "aria-selected": boolean;
  readonly onmouseenter: () => void;
  readonly onclick: () => void;
}

export interface OptionLook {
  // The row's value, unique within the pill; a key for `#each`.
  readonly key: string;
  readonly label: string;
  readonly note: string | undefined;
  // The value in force, marked with a check.
  readonly chosen: boolean;
  // Where the cursor stands, lifted a step.
  readonly cursor: boolean;
  readonly wire: OptionWire;
}

export interface MenuLook {
  readonly edge: Edge;
  // The bag spread on the menu's box, so the seat can measure it.
  readonly box: Readonly<Record<symbol, Attachment<HTMLElement>>>;
  // What heads the menu: the pill's `about`, or its name.
  readonly about: string;
  // What the fact has to it beyond its value, read out with the list.
  readonly told: { readonly id: string; readonly says: Snippet } | undefined;
  readonly filter: FilterWire | undefined;
  readonly list: ListWire;
  readonly options: readonly OptionLook[];
  // Said in place of the rows when the filter keeps none.
  readonly empty: string;
}

// Everything a look is given.
export interface PillLook {
  readonly frame: FrameWire;
  readonly trigger: TriggerWire;
  // The value in force, or the placeholder.
  readonly face: string;
  readonly menu: MenuLook | undefined;
}

// What the seat holds between draws.
export interface PillHeld {
  readonly open: boolean;
  readonly query: string;
  readonly at: number;
  readonly edge: Edge;
}

// What a look is drawn from besides the seat's state.
export interface PillDrawing {
  readonly spec: Pill;
  readonly uid: string;
  readonly told: Snippet | undefined;
  // `part_no_match`, already in the person's language.
  readonly empty: string;
}

// What only the seat can do, because only it holds the state and the
// drawn elements.
export interface PillHands {
  readonly open: () => void;
  readonly close: (focus: "opener" | "leave") => void;
  // The cursor moved by a key, and the row it lands on scrolled into
  // view; the cursor moved by the pointer, which is over the row already.
  readonly point: (at: number) => void;
  readonly hover: (at: number) => void;
  readonly query: (text: string) => void;
  // Whether an element is inside the pill's frame.
  readonly inside: (target: EventTarget | null) => boolean;
  readonly hold: {
    readonly frame: Attachment<HTMLElement>;
    readonly trigger: Attachment<HTMLElement>;
    readonly box: Attachment<HTMLElement>;
    readonly filter: Attachment<HTMLElement>;
    readonly list: Attachment<HTMLElement>;
  };
}

// A list long enough to scroll gets a filter; a list of six does not.
export function filtered(spec: Pill): boolean {
  return spec.choices.length > FILTER_AFTER;
}

// The rows a filter keeps: those whose name, note or value hold the
// typed text, ignoring case.
export function rowsOf(choices: readonly Choice[], query: string): readonly Choice[] {
  const needle = query.trim().toLowerCase();
  if (needle === "") return choices;
  return choices.filter((each) => `${each.label} ${each.note ?? ""} ${each.value}`.toLowerCase().includes(needle));
}

// The cursor inside a list of `total` rows; 0 for an empty list.
export function cursorOf(at: number, total: number): number {
  return Math.max(0, Math.min(at, total - 1));
}

// What a key does in the open menu (client/Spec.lean §7-4, the listbox
// of a combobox): the arrows and Home and End move the cursor, Enter
// takes the row under it, Escape closes and gives focus back to the
// trigger, and Tab closes and lets focus go on.
export type PillKey =
  | { readonly kind: "move"; readonly to: number }
  | { readonly kind: "take" }
  | { readonly kind: "close" }
  | { readonly kind: "leave" };

export function keyOf(key: string, cursor: number, total: number): PillKey | undefined {
  switch (key) {
    case "ArrowDown":
      return { kind: "move", to: cursorOf(cursor + 1, total) };
    case "ArrowUp":
      return { kind: "move", to: cursorOf(cursor - 1, total) };
    case "Home":
      return { kind: "move", to: 0 };
    case "End":
      return { kind: "move", to: cursorOf(total - 1, total) };
    case "Enter":
      return { kind: "take" };
    case "Escape":
      return { kind: "close" };
    case "Tab":
      return { kind: "leave" };
    default:
      return undefined;
  }
}

// The edge a menu of `width` hangs from, under a trigger whose left
// edge is at `left`, in a window `room` wide: the trigger's left edge,
// unless the menu would then run past the window's right edge.
export function edgeFor(left: number, width: number, room: number): Edge {
  return left + width > room ? "right" : "left";
}

// The id of the row at `at`, which the cursor's `aria-activedescendant`
// names and the seat scrolls into view.
export function rowId(uid: string, at: number): string {
  return `${uid}-${String(at)}`;
}

// The whole value a look draws.
export function lookOf(drawing: PillDrawing, held: PillHeld, hands: PillHands): PillLook {
  const { spec, uid } = drawing;
  const chosen = spec.choices.find((each) => each.value === spec.value);
  const face = chosen?.label ?? spec.placeholder;
  return {
    frame: {
      onfocusout: (event) => {
        if (held.open && !hands.inside(event.relatedTarget)) hands.close("leave");
      },
      [HOLD]: hands.hold.frame,
    },
    trigger: {
      "aria-label": `${spec.label}: ${face}`,
      "aria-haspopup": "listbox",
      "aria-expanded": held.open,
      "aria-controls": held.open ? `${uid}-list` : undefined,
      onclick: () => {
        if (held.open) hands.close("opener");
        else hands.open();
      },
      [HOLD]: hands.hold.trigger,
    },
    face,
    menu: held.open ? menuOf(drawing, held, hands) : undefined,
  };
}

function menuOf(drawing: PillDrawing, held: PillHeld, hands: PillHands): MenuLook {
  const { spec, uid, told } = drawing;
  const rows = rowsOf(spec.choices, held.query);
  const cursor = cursorOf(held.at, rows.length);
  const toldId = told === undefined ? undefined : `${uid}-told`;
  const active = rows.length > 0 ? rowId(uid, cursor) : undefined;
  const hasFilter = filtered(spec);
  const take = (value: string | undefined): void => {
    if (value === undefined) return;
    spec.pick(value);
    hands.close("opener");
  };
  const onkeydown = (event: KeyPress): void => {
    const pressed = keyOf(event.key, cursor, rows.length);
    if (pressed === undefined) return;
    switch (pressed.kind) {
      case "move":
        event.preventDefault();
        hands.point(pressed.to);
        return;
      case "take":
        event.preventDefault();
        take(rows[cursor]?.value);
        return;
      case "close":
        event.preventDefault();
        event.stopPropagation();
        hands.close("opener");
        return;
      case "leave":
        hands.close("leave");
        return;
    }
  };
  return {
    edge: held.edge,
    box: { [HOLD]: hands.hold.box },
    about: spec.about ?? spec.label,
    told: told === undefined || toldId === undefined ? undefined : { id: toldId, says: told },
    filter: hasFilter
      ? {
          role: "combobox",
          "aria-label": spec.label,
          "aria-expanded": true,
          "aria-controls": `${uid}-list`,
          "aria-describedby": toldId,
          "aria-activedescendant": active,
          placeholder: spec.placeholder,
          value: held.query,
          oninput: (event) => {
            hands.query(event.currentTarget.value);
          },
          onkeydown,
          [HOLD]: hands.hold.filter,
        }
      : undefined,
    list: {
      id: `${uid}-list`,
      role: "listbox",
      tabindex: hasFilter ? -1 : 0,
      "aria-label": spec.label,
      "aria-describedby": toldId,
      "aria-activedescendant": hasFilter ? undefined : active,
      onkeydown,
      onmousedown: (event) => {
        event.preventDefault();
      },
      [HOLD]: hands.hold.list,
    },
    options: rows.map((row, index) => ({
      key: row.value,
      label: row.label,
      note: row.note === "" ? undefined : row.note,
      chosen: row.value === spec.value,
      cursor: index === cursor,
      wire: {
        id: rowId(uid, index),
        role: "option",
        "aria-selected": row.value === spec.value,
        onmouseenter: () => {
          hands.hover(index);
        },
        onclick: () => {
          take(row.value);
        },
      },
    })),
    empty: drawing.empty,
  };
}
