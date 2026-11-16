// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { HTMLAnchorAttributes } from "svelte/elements";

import { initialTyped, lineWalker } from "../../core/lines";
import { pressedOf } from "../../core/press";

export type EntryLink = Pick<HTMLAnchorAttributes, "href" | "onclick"> & {
  readonly "data-entry": "";
};

const DIGITS = 9;

const lines = lineWalker();

function entriesIn(root: HTMLElement): HTMLElement[] {
  return [...root.querySelectorAll("[data-entry]")].filter((node) => node instanceof HTMLElement);
}

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
