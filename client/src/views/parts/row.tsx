// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// One line of a list: what the thing is called, what else is worth
// knowing about it, where it stands, and what can be done to it - and
// the list itself, which is what a keyboard walks.
//
// When the row leads somewhere, the two texts are the button that goes
// there, so the keyboard reaches the same place the pointer does and the
// actions on the right stay separately reachable. The whole row lights
// up while anything inside it holds the focus ring, because a ring
// around one word in a wide row is a position a person has to hunt for.
//
// The second text is the row's summary, and a compact page does not
// draw it: `theme.css` owns that judgement under the one density
// attribute the six spacing steps already read, so this file marks the
// line and states no rule about it.

import { Show, type JSX } from "solid-js";

// What a keyboard can land on inside a row. A row usually offers one
// control and sometimes three; the walk stops at the first of them, and
// a row offering none - a spacer, a sentinel - is stepped over.
const REACHABLE = 'a[href], button:not([disabled]), [tabindex="0"]';

// A key the control under the pointer is already using is not the
// list's to take: an arrow moves the caret in a box somebody is typing
// in, Home and End reach the ends of that line, and a select opens its
// own menu.
function occupied(target: EventTarget | null): boolean {
  if (target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement) {
    return true;
  }
  if (target instanceof HTMLSelectElement) {
    return true;
  }
  return target instanceof HTMLElement && target.isContentEditable;
}

// Which row an arrow key asks for.
type Step = "next" | "previous" | "first" | "last";

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

function landing(rows: readonly Element[], step: Step, from: number): HTMLElement | null {
  const [start, by] = courseOf(step, from, rows.length - 1);
  for (let at = start; at >= 0 && at < rows.length; at += by) {
    const reach = rows[at]?.querySelector(REACHABLE);
    if (reach instanceof HTMLElement) {
      return reach;
    }
  }
  return null;
}

export interface RowListProps {
  // The accessible name of the list, already in the person's language.
  readonly label: string;
  // One element per row, in the order they are drawn. The caller keeps
  // its own `For`, because which rows exist is the page's knowledge.
  readonly children: JSX.Element;
}

// A list a keyboard walks: Up and Down step between rows, Home and End
// reach its ends, and the focus lands on whatever that row offers -
// its own control, or the first of the several it carries.
//
// Every row stays a tab stop, which is what makes the arrows an
// addition rather than a replacement: Tab still crosses the list the
// way it always did, and a person who never presses an arrow loses
// nothing.
export function RowList(props: RowListProps) {
  return (
    <ul
      aria-label={props.label}
      onKeyDown={(event) => {
        const step = stepOf(event.key);
        if (step === null || occupied(event.target)) {
          return;
        }
        const rows = [...event.currentTarget.children];
        const from = rows.findIndex(
          (row) => event.target instanceof Node && row.contains(event.target),
        );
        if (from < 0) {
          return;
        }
        const next = landing(rows, step, from);
        if (next === null) {
          return;
        }
        // The page must not scroll out from under the row that just
        // took the focus.
        event.preventDefault();
        next.focus();
      }}
    >
      {props.children}
    </ul>
  );
}

export interface RowProps {
  // Already in the person's language, or an identifier.
  readonly primary: string;
  readonly secondary?: string;
  // Usually a Badge: where the thing stands.
  readonly status?: JSX.Element;
  // Usually Buttons: what can be done to it.
  readonly actions?: JSX.Element;
  // Present makes the row lead somewhere.
  readonly onOpen?: () => void;
}

export function Row(props: RowProps) {
  const texts = () => (
    <>
      <span class="truncate text-body text-text">{props.primary}</span>
      <Show when={props.secondary}>
        {(secondary) => <span class="summary truncate text-note text-text-faint">{secondary()}</span>}
      </Show>
    </>
  );
  return (
    <div class="flex w-full min-w-0 items-center gap-base border-b border-g2 px-base py-snug hover:bg-g1 has-[:focus-visible]:bg-g1">
      <Show
        when={props.onOpen}
        fallback={<div class="flex min-w-0 flex-1 flex-col text-left">{texts()}</div>}
      >
        {(open) => (
          <button
            type="button"
            class="flex min-w-0 flex-1 flex-col text-left"
            onClick={() => {
              open()();
            }}
          >
            {texts()}
          </button>
        )}
      </Show>
      <Show when={props.status}>{(status) => <div class="shrink-0">{status()}</div>}</Show>
      <Show when={props.actions}>{(actions) => <div class="flex shrink-0 items-center gap-tight">{actions()}</div>}</Show>
    </div>
  );
}
