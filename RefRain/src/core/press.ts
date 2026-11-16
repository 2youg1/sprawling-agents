// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// One key press, as the key tables read it: the shell's chords
// (`keys.ts`) and a list's own line keys (`lines.ts`) judge the same
// record, read off the event in one place.

// How long a key stays down before it is a hold rather than a press:
// the layers key becomes a look at the blend tier, and the accelerator
// alone draws every key's name (docs/frontend-method.md §7E).
export const HOLD_MS = 300;

// The part of a key press this table judges. A `KeyboardEvent` is one;
// so is the record a test writes.
export interface Pressed {
  readonly key: string;
  readonly ctrlKey: boolean;
  readonly metaKey: boolean;
  readonly shiftKey: boolean;
  readonly altKey: boolean;
  // Where the press landed: a text field the person is writing in, or
  // the page at large. The caller knows the target element; this table
  // only needs the fact.
  readonly target: "field" | "page";
}

// The one reading of a key press off the event. A press that lands in a
// text field, or arrives while an input method is composing, is the
// person's typing, so it counts as landing in a field.
export function pressedOf(event: KeyboardEvent): Pressed {
  const target = event.target;
  const writing =
    target instanceof HTMLInputElement ||
    target instanceof HTMLTextAreaElement ||
    target instanceof HTMLSelectElement ||
    (target instanceof HTMLElement && target.isContentEditable);
  return {
    key: event.key,
    ctrlKey: event.ctrlKey,
    metaKey: event.metaKey,
    shiftKey: event.shiftKey,
    altKey: event.altKey,
    target: writing || event.isComposing ? "field" : "page",
  };
}
