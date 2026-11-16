// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the inspector's tab strip decides, apart from how it is drawn
// (client D95, the key table of client/Spec.lean §7-11): which tab is
// the strip's one Tab stop, where an arrow, Home or End lands, what
// Delete closes, and the wire bags that carry every role, `aria-*`
// value and handler onto the elements a look draws. Nothing here
// touches the DOM; the seat (`strip.svelte`) holds the drawn tabs and
// moves the focus.

import { createAttachmentKey } from "svelte/attachments";
import type { Attachment } from "svelte/attachments";

// One open item as the strip shows it, already resolved by the seat.
export interface StripTab {
  // Unique within the strip; a key for `#each` and for the drawn element.
  readonly key: string;
  // Already in the person's language.
  readonly label: string;
  // A terminal's tab carries the terminal mark.
  readonly terminal: boolean;
  // A document whose RefRain session holds words the city has not taken.
  readonly unsaved: boolean;
  // The tab whose item the region below shows.
  readonly front: boolean;
  // The item beside this conversation as the address bar spells it, or
  // `null` for an item it has no spelling for.
  readonly href: string | null;
  // The id of the region panel this tab brings forward.
  readonly controls: string;
}

// The strip's words, already in the person's language.
export interface StripWords {
  readonly tabs: string;
  readonly unsaved: string;
  readonly closeAll: string;
  readonly closeItem: (name: string) => string;
}

// What only the seat can do: bring an item forward or close it, close
// the whole inspector, and move the focus to the tab at a place once
// the strip has redrawn - the list a Delete shortened is the one the
// focus lands in. `hold` answers the same attachment for the same key on
// every call.
export interface StripHands {
  readonly pick: (at: number) => void;
  readonly close: (at: number) => void;
  readonly closeAll: () => void;
  readonly focus: (at: number) => void;
  readonly hold: (key: string) => Attachment<HTMLElement>;
}

// What a handler reads from an event, and nothing more, so the wiring
// test can press a key or a pointer without a DOM.
export type KeyPress = Pick<KeyboardEvent, "key" | "preventDefault">;
export type Press = Pick<MouseEvent, "preventDefault">;

// The bag spread on the row of tabs.
export interface ListWire {
  readonly role: "tablist";
  readonly "aria-label": string;
}

// The bag spread on one tab. A tab is a link to its item (client D41):
// a press only brings it forward, so the click handler keeps the
// address bar where it is.
export interface TabWire {
  readonly role: "tab";
  readonly href: string | undefined;
  readonly "aria-selected": boolean;
  readonly "aria-controls": string;
  readonly tabindex: 0 | -1;
  readonly onclick: (event: Press) => void;
  readonly onkeydown: (event: KeyPress) => void;
  readonly [hold: symbol]: Attachment<HTMLElement>;
}

// The bag spread on a tab's own close mark. It is not a Tab stop of its
// own, because Delete on the tab is that action's key.
export interface CloseWire {
  readonly type: "button";
  readonly tabindex: -1;
  readonly "aria-label": string;
  readonly onclick: () => void;
}

// The bag spread on the key that closes the whole inspector.
export interface CloseAllWire {
  readonly type: "button";
  readonly "aria-label": string;
  readonly onclick: () => void;
}

export interface TabLook {
  readonly key: string;
  readonly label: string;
  readonly terminal: boolean;
  readonly front: boolean;
  // The words a screen reader hears for the unsaved mark, when it shows.
  readonly unsaved: string | undefined;
  readonly wire: TabWire;
  readonly close: CloseWire;
}

// Everything a look is given.
export interface StripLook {
  readonly list: ListWire;
  readonly tabs: readonly TabLook[];
  readonly closeAll: CloseAllWire;
}

// The strip is one Tab stop: the front tab, or the first one when none
// of them is in front, so the keyboard always has a way in.
export function tabStop(tabs: readonly StripTab[]): number {
  const front = tabs.findIndex((tab) => tab.front);
  return front === -1 ? 0 : front;
}

// Where an arrow, Home or End lands from `at` among `count` tabs; the
// arrows wrap at either end. `undefined` for any other key.
export function landing(count: number, at: number, key: string): number | undefined {
  const last = count - 1;
  switch (key) {
    case "ArrowLeft":
      return at === 0 ? last : at - 1;
    case "ArrowRight":
      return at === last ? 0 : at + 1;
    case "Home":
      return 0;
    case "End":
      return last;
    default:
      return undefined;
  }
}

// One key for every tab's attachment; Svelte keeps one attachment per
// element under each symbol key.
const HOLD = createAttachmentKey();

// The whole value a look draws.
export function lookOf(tabs: readonly StripTab[], words: StripWords, hands: StripHands): StripLook {
  const stop = tabStop(tabs);

  const keys = (event: KeyPress, at: number): void => {
    if (event.key === "Delete") {
      event.preventDefault();
      hands.close(at);
      hands.focus(Math.min(at, tabs.length - 2));
      return;
    }
    const to = landing(tabs.length, at, event.key);
    if (to === undefined) return;
    event.preventDefault();
    hands.pick(to);
    hands.focus(to);
  };

  return {
    list: { role: "tablist", "aria-label": words.tabs },
    tabs: tabs.map((tab, at) => ({
      key: tab.key,
      label: tab.label,
      terminal: tab.terminal,
      front: tab.front,
      unsaved: tab.unsaved ? words.unsaved : undefined,
      wire: {
        role: "tab",
        href: tab.href ?? undefined,
        "aria-selected": tab.front,
        "aria-controls": tab.controls,
        tabindex: at === stop ? 0 : -1,
        onclick: (event) => {
          event.preventDefault();
          hands.pick(at);
        },
        onkeydown: (event) => {
          keys(event, at);
        },
        [HOLD]: hands.hold(tab.key),
      },
      close: {
        type: "button",
        tabindex: -1,
        "aria-label": words.closeItem(tab.label),
        onclick: () => {
          hands.close(at);
        },
      },
    })),
    closeAll: { type: "button", "aria-label": words.closeAll, onclick: hands.closeAll },
  };
}
