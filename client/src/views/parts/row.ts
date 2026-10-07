// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What a row and the list of rows decide before anything is drawn:
// which keys the list takes, where an arrow lands, and everything a
// row's look is handed. The walk is the model in
// `client/spec/Views/Parts/Row.lean`, and `row.test.ts` replays its
// properties against these functions.

import type { Snippet } from "svelte";

export interface RowProps {
  // Already in the person's language, or an identifier.
  readonly primary: string;
  readonly secondary?: string;
  // Usually a Badge: where the thing stands.
  readonly status?: Snippet;
  // Usually Buttons: what can be done to it.
  readonly actions?: Snippet;
  // Present makes the row lead somewhere.
  readonly onOpen?: () => void;
  // The one row of the list that is the thing shown beside it.
  readonly chosen?: boolean;
}

export interface RowListProps {
  // The accessible name of the list, already in the person's language.
  readonly label: string;
  // The rows, in the order they are drawn. Each renders an `<li>` -
  // a `Row` here, or a row the page draws for itself.
  readonly rows: Snippet;
}

// Spread on the `<li>`: the chosen row says so to a screen reader as
// well as with the bar.
export interface ItemWire {
  readonly "aria-current"?: "true";
}

// Spread on the button the two texts become when the row leads
// somewhere.
export interface OpenWire {
  readonly type: "button";
  readonly onclick: () => void;
}

// Everything a row's look draws, and nothing else.
export interface RowLook {
  readonly item: ItemWire;
  readonly chosen: boolean;
  readonly primary: string;
  readonly secondary: string | undefined;
  readonly status: Snippet | undefined;
  readonly actions: Snippet | undefined;
  readonly open: OpenWire | undefined;
}

export function lookOf(props: RowProps): RowLook {
  const chosen = props.chosen === true;
  const onOpen = props.onOpen;
  return {
    item: chosen ? { "aria-current": "true" } : {},
    chosen,
    primary: props.primary,
    secondary: props.secondary,
    status: props.status,
    actions: props.actions,
    open:
      onOpen === undefined
        ? undefined
        : {
            type: "button",
            onclick: () => {
              onOpen();
            },
          },
  };
}

// What kind of element a key arrived from. A key the control under the
// pointer is already using is not the list's to take: an arrow moves
// the caret in a box somebody is typing in, Home and End reach the ends
// of that line, and a select opens its own menu.
export type Target = "row" | "text" | "select" | "editable";

// Which row an arrow key asks for.
export type Step = "next" | "previous" | "first" | "last";

// The step a key asks the list for, or null when the key is not the
// list's: Tab belongs to the platform, and a field keeps every key.
export function taken(target: Target, key: string): Step | null {
  switch (target) {
    case "row":
      return stepOf(key);
    case "text":
    case "select":
    case "editable":
      return null;
  }
}

function stepOf(key: string): Step | null {
  switch (key) {
    case "ArrowDown":
      return "next";
    case "ArrowUp":
      return "previous";
    case "Home":
      return "first";
    case "End":
      return "last";
    default:
      return null;
  }
}

// Where a step starts and which way it runs. The ends do not wrap: a
// thousand-row ledger that jumps from the last row to the first has
// moved a person somewhere they cannot see they went.
function courseOf(step: Step, from: number, last: number): readonly [number, number] {
  switch (step) {
    case "next":
      return [from + 1, 1];
    case "previous":
      return [from - 1, -1];
    case "first":
      return [0, 1];
    case "last":
      return [last, -1];
  }
}

// The control a step lands on, given each row's first reachable control
// in order (undefined for a row offering none - a spacer, a sentinel -
// which the walk steps over). Undefined when the step runs off an end.
export function landing<T>(reach: readonly (T | undefined)[], step: Step, from: number): T | undefined {
  const [start, by] = courseOf(step, from, reach.length - 1);
  for (let at = start; at >= 0 && at < reach.length; at += by) {
    const found = reach[at];
    if (found !== undefined) {
      return found;
    }
  }
  return undefined;
}
