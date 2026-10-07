// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The properties this module must hold are proved in `client/spec/Views/Workspace.lean`.
//
// Walking the mailbox's entries from the keyboard (client/Spec.lean §7-11):
// the line keys of `core/lines.ts` move to the next, the previous, the
// first and the last entry, and a digit reaches the entry it is drawn
// beside. An entry is any element marked `data-entry`, in document
// order, so the sections need not know how many entries stand above
// them; the digit beside each is a CSS counter (`theme/mailbox.css`, the mailbox
// block) counting the same marks in the same order, so the number drawn
// and the number pressed cannot disagree.
//
// The keys are the column's own, as a list's arrows are a list's own:
// keys typed into a field, or pressed with a modifier, are not theirs,
// and an entry the virtual list has not mounted is not one yet. Enter
// and Escape stay the entry's and the panel's own.

import { initialTyped, lineWalker } from "../../core/lines";
import { pressedOf } from "../../core/press";

// How many entries a digit can reach: one to nine.
const DIGITS = 9;

// One mailbox per page, so one walker holds the first g of gg.
const lines = lineWalker();

function entriesIn(root: HTMLElement): HTMLElement[] {
  return [...root.querySelectorAll("[data-entry]")].filter((node) => node instanceof HTMLElement);
}

// An entry that goes somewhere is followed; one that is answered in
// place (a decide card) takes the focus, where its own keys are heard.
function reach(entry: HTMLElement): void {
  if (entry instanceof HTMLAnchorElement) entry.click();
  else entry.focus();
}

export function walk(root: HTMLElement, event: KeyboardEvent): void {
  const pressed = pressedOf(event);
  const entries = entriesIn(root);
  const digit = Number.parseInt(initialTyped(pressed) ?? "", 10);
  if (digit >= 1 && digit <= DIGITS) {
    const entry = entries[digit - 1];
    if (entry === undefined) return;
    event.preventDefault();
    reach(entry);
    return;
  }
  const target = event.target;
  const at = entries.findIndex((entry) => target instanceof Node && entry.contains(target));
  const next = (() => {
    switch (lines(pressed, event.timeStamp)) {
      case "line.next":
        return entries[at < 0 ? 0 : Math.min(entries.length - 1, at + 1)];
      case "line.previous":
        return entries[Math.max(0, at - 1)];
      case "line.first":
        return entries[0];
      case "line.last":
        return entries.at(-1);
      case "line.open":
      case "line.close":
      case null:
        return undefined;
    }
  })();
  if (next === undefined) return;
  event.preventDefault();
  next.focus();
}
