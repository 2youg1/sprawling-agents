// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Moving between the lines of a list, and reaching a row by its first
// letter. These keys belong to whichever list holds the focus rather
// than to the shell, so they are a table of their own beside the
// shell's chords (`keys.ts`), and every list reads this one.

import { face, folded } from "./keys";
import type { Pressed } from "./press";

// Moving between the lines of a list: a tool line, a mailbox entry, a
// row of the runs board. Each list answers only the moves it has a use
// for, and spells none of the keys itself (refrain 3-12).
export const LINE_MOVES = ["line.next", "line.previous", "line.first", "line.last", "line.open", "line.close"] as const;

export type LineMove = (typeof LINE_MOVES)[number];

// The keys of each move, each key the sequence of `KeyboardEvent.key`
// values typed one after the other. The first key of a move is the one
// a row draws, because every keyboard labels it; the letters are the
// terminal's and vim's (j, k, gg, G), read as typed, so g and G are two
// keys and Shift needs no mark of its own.
export const LINE_KEYS: Readonly<Record<LineMove, readonly (readonly string[])[]>> = {
  "line.next": [["ArrowDown"], ["j"]],
  "line.previous": [["ArrowUp"], ["k"]],
  "line.first": [["Home"], ["g", "g"]],
  "line.last": [["End"], ["G"]],
  "line.open": [["Enter"]],
  "line.close": [["Escape"]],
};

// How long the second key of a sequence may wait for the first, as in
// vim's own default `timeoutlen`.
export const SEQUENCE_MS = 1000;

function sameKeys(left: readonly string[], right: readonly string[]): boolean {
  return left.length === right.length && left.every((key, at) => right[at] === key);
}

function moveOf(typed: readonly string[]): LineMove | undefined {
  return LINE_MOVES.find((move) => LINE_KEYS[move].some((keys) => sameKeys(keys, typed)));
}

function begins(typed: readonly string[]): boolean {
  return LINE_MOVES.some((move) =>
    LINE_KEYS[move].some((keys) => keys.length > typed.length && sameKeys(keys.slice(0, typed.length), typed)),
  );
}

// A reader of presses for one list, holding the start of a sequence
// between two presses. `at` is the press's own time (`timeStamp`), so
// the reader keeps no clock. A press in a field, or one holding a
// modifier, is not a move and drops a sequence begun before it.
export function lineWalker(): (pressed: Pressed, at: number) => LineMove | null {
  let held: readonly string[] = [];
  let heldAt = Number.NEGATIVE_INFINITY;
  return (pressed, at) => {
    const was = held;
    held = [];
    if (pressed.target === "field" || pressed.ctrlKey || pressed.metaKey || pressed.altKey) return null;
    const continued = at - heldAt <= SEQUENCE_MS ? [...was, pressed.key] : [pressed.key];
    heldAt = at;
    for (const typed of [continued, [pressed.key]]) {
      const move = moveOf(typed);
      if (move !== undefined) return move;
      if (begins(typed)) {
        held = typed;
        return null;
      }
    }
    return null;
  };
}

function lineFace(key: string): string {
  return key.length === 1 ? key : face(key);
}

// How each key of a move is drawn, in the table's order: a sequence is
// its keys run together (gg), a letter keeps the case it is typed in.
export function lineFaces(move: LineMove): readonly string[] {
  return LINE_KEYS[move].map((keys) => keys.map(lineFace).join(""));
}

// --------------------------------------------------------- first letters

function typeable(letter: string): boolean {
  return /^[a-z0-9]$/.test(letter);
}

// The letter a row is reached by when a list moves by first letters:
// the first letter of the name a person reads, when one key types it,
// and otherwise the first letter of `slug`, the row's name in the
// address bar, which every keyboard types (client D40).
export function initialOf(name: string, slug: string): string {
  const first = folded(name.trim().slice(0, 1));
  return typeable(first) ? first : folded(slug.slice(0, 1));
}

// The first letter a press asks for, or none when the press is not one
// letter typed outside a field with no modifier.
export function initialTyped(pressed: Pressed): string | null {
  if (pressed.target === "field" || pressed.ctrlKey || pressed.metaKey || pressed.altKey) return null;
  const letter = folded(pressed.key);
  return typeable(letter) ? letter : null;
}
